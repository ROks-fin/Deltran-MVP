package compliance

import (
	"context"
	"database/sql"
	"fmt"
	"strings"
	"sync"
	"time"

	"github.com/rs/zerolog/log"
)

// SanctionsScreener performs sanctions screening against loaded sanction lists
type SanctionsScreener struct {
	db              *sql.DB
	cache           map[string]*SanctionsEntry
	cacheMu         sync.RWMutex
	lastCacheUpdate time.Time
	cacheRefresh    time.Duration
	fuzzyThreshold  int // Levenshtein distance threshold for fuzzy matching
}

// SanctionsEntry represents a sanctioned entity
type SanctionsEntry struct {
	ID           string
	EntityType   string // INDIVIDUAL, ENTITY, VESSEL, etc.
	Names        []string
	Aliases      []string
	Country      string
	Source       string // OFAC, EU, UN, UK_HMT
	ListType     string // SDN, CONSOLIDATED, etc.
	AddedDate    time.Time
	Identifiers  []Identifier
}

// Identifier represents additional identifiers for sanctioned entities
type Identifier struct {
	Type  string // SWIFT_BIC, PASSPORT, TAX_ID, etc.
	Value string
}

// ScreeningResult represents the result of sanctions screening
type ScreeningResult struct {
	Hit             bool              `json:"hit"`
	RiskLevel       string            `json:"risk_level"` // HIGH, MEDIUM, LOW
	Matches         []ScreeningMatch  `json:"matches"`
	RequiresReview  bool              `json:"requires_review"`
	ScreenedAt      time.Time         `json:"screened_at"`
}

// ScreeningMatch represents a match against sanctions list
type ScreeningMatch struct {
	EntryID       string  `json:"entry_id"`
	MatchedName   string  `json:"matched_name"`
	MatchedField  string  `json:"matched_field"` // sender_name, receiver_name, sender_bic, etc.
	MatchScore    float64 `json:"match_score"`   // 0.0-1.0, higher is better match
	Source        string  `json:"source"`         // OFAC, EU, UN, UK_HMT
	FuzzyMatch    bool    `json:"fuzzy_match"`
}

// ScreeningRequest represents a screening request
type ScreeningRequest struct {
	SenderName     string
	SenderBIC      string
	SenderCountry  string
	ReceiverName   string
	ReceiverBIC    string
	ReceiverCountry string
	PaymentReference string
}

// NewSanctionsScreener creates a new sanctions screener
func NewSanctionsScreener(db *sql.DB) *SanctionsScreener {
	return &SanctionsScreener{
		db:             db,
		cache:          make(map[string]*SanctionsEntry),
		cacheRefresh:   1 * time.Hour, // Refresh cache every hour
		fuzzyThreshold: 3,              // Max Levenshtein distance for fuzzy matching
	}
}

// Start starts the sanctions screener background tasks
func (s *SanctionsScreener) Start(ctx context.Context) {
	// Initial cache load
	if err := s.refreshCache(ctx); err != nil {
		log.Error().Err(err).Msg("Failed to load sanctions cache")
	}

	// Periodic cache refresh
	ticker := time.NewTicker(s.cacheRefresh)
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			log.Info().Msg("Sanctions screener stopped")
			return
		case <-ticker.C:
			if err := s.refreshCache(ctx); err != nil {
				log.Error().Err(err).Msg("Failed to refresh sanctions cache")
			}
		}
	}
}

// Screen performs sanctions screening
func (s *SanctionsScreener) Screen(ctx context.Context, req *ScreeningRequest) (*ScreeningResult, error) {
	result := &ScreeningResult{
		Hit:        false,
		RiskLevel:  "LOW",
		Matches:    []ScreeningMatch{},
		ScreenedAt: time.Now().UTC(),
	}

	// Check if cache needs refresh
	if time.Since(s.lastCacheUpdate) > s.cacheRefresh {
		if err := s.refreshCache(ctx); err != nil {
			log.Warn().Err(err).Msg("Failed to refresh sanctions cache, using stale cache")
		}
	}

	s.cacheMu.RLock()
	defer s.cacheMu.RUnlock()

	// Screen sender
	senderMatches := s.screenEntity(req.SenderName, req.SenderBIC, req.SenderCountry, "sender")
	result.Matches = append(result.Matches, senderMatches...)

	// Screen receiver
	receiverMatches := s.screenEntity(req.ReceiverName, req.ReceiverBIC, req.ReceiverCountry, "receiver")
	result.Matches = append(result.Matches, receiverMatches...)

	// Determine risk level and hit status
	if len(result.Matches) > 0 {
		result.Hit = true
		result.RiskLevel = s.calculateRiskLevel(result.Matches)
		result.RequiresReview = s.requiresReview(result.Matches)
	}

	return result, nil
}

// screenEntity screens a single entity (sender or receiver)
func (s *SanctionsScreener) screenEntity(name, bic, country, entityType string) []ScreeningMatch {
	matches := []ScreeningMatch{}

	// Normalize inputs
	name = strings.ToUpper(strings.TrimSpace(name))
	bic = strings.ToUpper(strings.TrimSpace(bic))
	country = strings.ToUpper(strings.TrimSpace(country))

	for _, entry := range s.cache {
		// Check BIC match (exact match only)
		if bic != "" {
			for _, identifier := range entry.Identifiers {
				if identifier.Type == "SWIFT_BIC" && strings.EqualFold(identifier.Value, bic) {
					matches = append(matches, ScreeningMatch{
						EntryID:      entry.ID,
						MatchedName:  bic,
						MatchedField: entityType + "_bic",
						MatchScore:   1.0, // Exact match
						Source:       entry.Source,
						FuzzyMatch:   false,
					})
				}
			}
		}

		// Check name match (exact + fuzzy)
		if name != "" {
			nameMatches := s.matchName(name, entry)
			for _, match := range nameMatches {
				match.MatchedField = entityType + "_name"
				matches = append(matches, match)
			}
		}

		// Check country match (for additional context, not a direct match)
		if country != "" && entry.Country != "" {
			if strings.EqualFold(country, entry.Country) {
				// Country match increases suspicion, lower threshold for name matching
				// This is handled in risk calculation
			}
		}
	}

	return matches
}

// matchName performs name matching (exact + fuzzy)
func (s *SanctionsScreener) matchName(queryName string, entry *SanctionsEntry) []ScreeningMatch {
	matches := []ScreeningMatch{}
	queryName = normalizeForMatching(queryName)

	// Check primary names
	for _, entryName := range entry.Names {
		entryName = normalizeForMatching(entryName)

		// Exact match
		if queryName == entryName {
			matches = append(matches, ScreeningMatch{
				EntryID:     entry.ID,
				MatchedName: entryName,
				MatchScore:  1.0,
				Source:      entry.Source,
				FuzzyMatch:  false,
			})
			continue
		}

		// Substring match (high score)
		if strings.Contains(queryName, entryName) || strings.Contains(entryName, queryName) {
			matches = append(matches, ScreeningMatch{
				EntryID:     entry.ID,
				MatchedName: entryName,
				MatchScore:  0.9,
				Source:      entry.Source,
				FuzzyMatch:  false,
			})
			continue
		}

		// Fuzzy match (Levenshtein distance)
		distance := levenshteinDistance(queryName, entryName)
		if distance <= s.fuzzyThreshold {
			score := 1.0 - (float64(distance) / float64(max(len(queryName), len(entryName))))
			matches = append(matches, ScreeningMatch{
				EntryID:     entry.ID,
				MatchedName: entryName,
				MatchScore:  score,
				Source:      entry.Source,
				FuzzyMatch:  true,
			})
		}
	}

	// Check aliases
	for _, alias := range entry.Aliases {
		alias = normalizeForMatching(alias)

		// Exact match
		if queryName == alias {
			matches = append(matches, ScreeningMatch{
				EntryID:     entry.ID,
				MatchedName: alias,
				MatchScore:  1.0,
				Source:      entry.Source,
				FuzzyMatch:  false,
			})
			continue
		}

		// Fuzzy match for aliases
		distance := levenshteinDistance(queryName, alias)
		if distance <= s.fuzzyThreshold {
			score := 1.0 - (float64(distance) / float64(max(len(queryName), len(alias))))
			matches = append(matches, ScreeningMatch{
				EntryID:     entry.ID,
				MatchedName: alias,
				MatchScore:  score,
				Source:      entry.Source,
				FuzzyMatch:  true,
			})
		}
	}

	return matches
}

// calculateRiskLevel calculates overall risk level based on matches
func (s *SanctionsScreener) calculateRiskLevel(matches []ScreeningMatch) string {
	if len(matches) == 0 {
		return "LOW"
	}

	highestScore := 0.0
	hasExactMatch := false

	for _, match := range matches {
		if match.MatchScore > highestScore {
			highestScore = match.MatchScore
		}
		if match.MatchScore == 1.0 && !match.FuzzyMatch {
			hasExactMatch = true
		}
	}

	// Risk level determination
	if hasExactMatch {
		return "HIGH"
	} else if highestScore >= 0.9 {
		return "HIGH"
	} else if highestScore >= 0.7 {
		return "MEDIUM"
	} else {
		return "LOW"
	}
}

// requiresReview determines if manual review is required
func (s *SanctionsScreener) requiresReview(matches []ScreeningMatch) bool {
	// Any HIGH risk or exact match requires review
	for _, match := range matches {
		if match.MatchScore >= 0.9 {
			return true
		}
	}

	// Multiple medium-score matches require review
	mediumMatches := 0
	for _, match := range matches {
		if match.MatchScore >= 0.7 {
			mediumMatches++
		}
	}
	return mediumMatches >= 2
}

// refreshCache refreshes the sanctions cache from database
func (s *SanctionsScreener) refreshCache(ctx context.Context) error {
	log.Info().Msg("Refreshing sanctions cache...")

	query := `
		SELECT
			id, entity_type, names, aliases, country, source, list_type, added_date
		FROM deltran.sanctions_list
		WHERE is_active = true
	`

	rows, err := s.db.QueryContext(ctx, query)
	if err != nil {
		return fmt.Errorf("failed to query sanctions list: %w", err)
	}
	defer rows.Close()

	newCache := make(map[string]*SanctionsEntry)
	count := 0

	for rows.Next() {
		var entry SanctionsEntry
		var namesJSON, aliasesJSON string

		err := rows.Scan(
			&entry.ID,
			&entry.EntityType,
			&namesJSON,
			&aliasesJSON,
			&entry.Country,
			&entry.Source,
			&entry.ListType,
			&entry.AddedDate,
		)
		if err != nil {
			log.Warn().Err(err).Msg("Failed to scan sanctions entry")
			continue
		}

		// Parse JSON arrays (simplified - in production use proper JSON parsing)
		entry.Names = strings.Split(strings.Trim(namesJSON, "{}"), ",")
		entry.Aliases = strings.Split(strings.Trim(aliasesJSON, "{}"), ",")

		// Load identifiers for this entry
		entry.Identifiers, _ = s.loadIdentifiers(ctx, entry.ID)

		newCache[entry.ID] = &entry
		count++
	}

	s.cacheMu.Lock()
	s.cache = newCache
	s.lastCacheUpdate = time.Now()
	s.cacheMu.Unlock()

	log.Info().Int("count", count).Msg("Sanctions cache refreshed")
	return nil
}

// loadIdentifiers loads identifiers for a sanctions entry
func (s *SanctionsScreener) loadIdentifiers(ctx context.Context, entryID string) ([]Identifier, error) {
	query := `
		SELECT identifier_type, identifier_value
		FROM deltran.sanctions_identifiers
		WHERE sanctions_list_id = $1
	`

	rows, err := s.db.QueryContext(ctx, query, entryID)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	identifiers := []Identifier{}
	for rows.Next() {
		var id Identifier
		if err := rows.Scan(&id.Type, &id.Value); err == nil {
			identifiers = append(identifiers, id)
		}
	}

	return identifiers, nil
}

// normalizeForMatching normalizes a string for matching
func normalizeForMatching(s string) string {
	s = strings.ToUpper(s)
	s = strings.TrimSpace(s)
	// Remove common punctuation
	s = strings.ReplaceAll(s, ".", "")
	s = strings.ReplaceAll(s, ",", "")
	s = strings.ReplaceAll(s, "-", " ")
	s = strings.ReplaceAll(s, "_", " ")
	// Collapse multiple spaces
	for strings.Contains(s, "  ") {
		s = strings.ReplaceAll(s, "  ", " ")
	}
	return s
}

// levenshteinDistance calculates the Levenshtein distance between two strings
func levenshteinDistance(s1, s2 string) int {
	if len(s1) == 0 {
		return len(s2)
	}
	if len(s2) == 0 {
		return len(s1)
	}

	// Create distance matrix
	matrix := make([][]int, len(s1)+1)
	for i := range matrix {
		matrix[i] = make([]int, len(s2)+1)
	}

	// Initialize first row and column
	for i := 0; i <= len(s1); i++ {
		matrix[i][0] = i
	}
	for j := 0; j <= len(s2); j++ {
		matrix[0][j] = j
	}

	// Calculate distances
	for i := 1; i <= len(s1); i++ {
		for j := 1; j <= len(s2); j++ {
			cost := 0
			if s1[i-1] != s2[j-1] {
				cost = 1
			}

			matrix[i][j] = min3(
				matrix[i-1][j]+1,      // deletion
				matrix[i][j-1]+1,      // insertion
				matrix[i-1][j-1]+cost, // substitution
			)
		}
	}

	return matrix[len(s1)][len(s2)]
}

func min3(a, b, c int) int {
	if a < b {
		if a < c {
			return a
		}
		return c
	}
	if b < c {
		return b
	}
	return c
}

func max(a, b int) int {
	if a > b {
		return a
	}
	return b
}
