package compliance

import (
	"testing"

	"github.com/stretchr/testify/assert"
)

func TestLevenshteinDistance(t *testing.T) {
	tests := []struct {
		s1       string
		s2       string
		expected int
	}{
		{"", "", 0},
		{"", "abc", 3},
		{"abc", "", 3},
		{"abc", "abc", 0},
		{"abc", "abd", 1},
		{"abc", "adc", 1},
		{"kitten", "sitting", 3},
		{"Saturday", "Sunday", 3},
		{"JPMORGAN CHASE", "JP MORGAN CHASE", 1},
		{"DEUTSCHE BANK", "DEUTSHE BANK", 1},
	}

	for _, tt := range tests {
		t.Run(tt.s1+"_"+tt.s2, func(t *testing.T) {
			result := levenshteinDistance(tt.s1, tt.s2)
			assert.Equal(t, tt.expected, result, "Expected distance %d, got %d for '%s' vs '%s'", tt.expected, result, tt.s1, tt.s2)
		})
	}
}

func TestNormalizeForMatching(t *testing.T) {
	tests := []struct {
		input    string
		expected string
	}{
		{"JPMorgan Chase", "JPMORGAN CHASE"},
		{"  Deutsche Bank  ", "DEUTSCHE BANK"},
		{"HSBC-Holdings", "HSBC HOLDINGS"},
		{"Bank_of_America", "BANK OF AMERICA"},
		{"Wells Fargo & Co.", "WELLS FARGO & CO"},
		{"Citibank, N.A.", "CITIBANK NA"},
		{"BNP  Paribas", "BNP PARIBAS"},
	}

	for _, tt := range tests {
		t.Run(tt.input, func(t *testing.T) {
			result := normalizeForMatching(tt.input)
			assert.Equal(t, tt.expected, result)
		})
	}
}

func TestMatchName(t *testing.T) {
	screener := &SanctionsScreener{
		fuzzyThreshold: 3,
	}

	entry := &SanctionsEntry{
		ID:      "TEST-001",
		Source:  "OFAC",
		Names:   []string{"BLOCKED ENTITY INC"},
		Aliases: []string{"BLOCKED CO", "BLOCKED COMPANY"},
	}

	tests := []struct {
		name           string
		queryName      string
		expectMatches  bool
		minScore       float64
		expectFuzzy    bool
	}{
		{
			name:          "exact match",
			queryName:     "BLOCKED ENTITY INC",
			expectMatches: true,
			minScore:      1.0,
			expectFuzzy:   false,
		},
		{
			name:          "substring match",
			queryName:     "BLOCKED ENTITY INC USA",
			expectMatches: true,
			minScore:      0.9,
			expectFuzzy:   false,
		},
		{
			name:          "fuzzy match - 1 char diff",
			queryName:     "BLOCKED ENTITI INC",
			expectMatches: true,
			minScore:      0.8,
			expectFuzzy:   true,
		},
		{
			name:          "alias exact match",
			queryName:     "BLOCKED CO",
			expectMatches: true,
			minScore:      1.0,
			expectFuzzy:   false,
		},
		{
			name:          "no match",
			queryName:     "TOTALLY DIFFERENT COMPANY",
			expectMatches: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			matches := screener.matchName(tt.queryName, entry)

			if tt.expectMatches {
				assert.NotEmpty(t, matches, "Expected matches but got none")
				if len(matches) > 0 {
					assert.GreaterOrEqual(t, matches[0].MatchScore, tt.minScore, "Match score too low")
					if tt.expectFuzzy {
						assert.True(t, matches[0].FuzzyMatch, "Expected fuzzy match")
					}
				}
			} else {
				assert.Empty(t, matches, "Expected no matches but got some")
			}
		})
	}
}

func TestCalculateRiskLevel(t *testing.T) {
	screener := &SanctionsScreener{}

	tests := []struct {
		name          string
		matches       []ScreeningMatch
		expectedRisk  string
	}{
		{
			name:         "no matches",
			matches:      []ScreeningMatch{},
			expectedRisk: "LOW",
		},
		{
			name: "exact match",
			matches: []ScreeningMatch{
				{MatchScore: 1.0, FuzzyMatch: false},
			},
			expectedRisk: "HIGH",
		},
		{
			name: "high score fuzzy match",
			matches: []ScreeningMatch{
				{MatchScore: 0.95, FuzzyMatch: true},
			},
			expectedRisk: "HIGH",
		},
		{
			name: "medium score match",
			matches: []ScreeningMatch{
				{MatchScore: 0.75, FuzzyMatch: true},
			},
			expectedRisk: "MEDIUM",
		},
		{
			name: "low score match",
			matches: []ScreeningMatch{
				{MatchScore: 0.65, FuzzyMatch: true},
			},
			expectedRisk: "LOW",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := screener.calculateRiskLevel(tt.matches)
			assert.Equal(t, tt.expectedRisk, result)
		})
	}
}

func TestRequiresReview(t *testing.T) {
	screener := &SanctionsScreener{}

	tests := []struct {
		name           string
		matches        []ScreeningMatch
		expectsReview  bool
	}{
		{
			name:          "no matches",
			matches:       []ScreeningMatch{},
			expectsReview: false,
		},
		{
			name: "high score match",
			matches: []ScreeningMatch{
				{MatchScore: 0.95},
			},
			expectsReview: true,
		},
		{
			name: "single medium match",
			matches: []ScreeningMatch{
				{MatchScore: 0.75},
			},
			expectsReview: false,
		},
		{
			name: "multiple medium matches",
			matches: []ScreeningMatch{
				{MatchScore: 0.75},
				{MatchScore: 0.72},
			},
			expectsReview: true,
		},
		{
			name: "low score matches",
			matches: []ScreeningMatch{
				{MatchScore: 0.6},
				{MatchScore: 0.65},
			},
			expectsReview: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			result := screener.requiresReview(tt.matches)
			assert.Equal(t, tt.expectsReview, result)
		})
	}
}

func BenchmarkLevenshteinDistance(b *testing.B) {
	s1 := "JPMORGAN CHASE BANK"
	s2 := "JP MORGAN CHASE BANK"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		levenshteinDistance(s1, s2)
	}
}

func BenchmarkNormalizeForMatching(b *testing.B) {
	input := "  JPMorgan-Chase_Bank, N.A.  "

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		normalizeForMatching(input)
	}
}

func TestScreenEntity(t *testing.T) {
	screener := &SanctionsScreener{
		fuzzyThreshold: 3,
		cache: map[string]*SanctionsEntry{
			"TEST-001": {
				ID:      "TEST-001",
				Source:  "OFAC",
				Names:   []string{"SANCTIONED BANK"},
				Country: "XX",
				Identifiers: []Identifier{
					{Type: "SWIFT_BIC", Value: "SANCTXXX"},
				},
			},
			"TEST-002": {
				ID:     "TEST-002",
				Source: "EU",
				Names:  []string{"BLOCKED ENTITY INC"},
			},
		},
	}

	tests := []struct {
		name          string
		entityName    string
		entityBIC     string
		entityCountry string
		expectHit     bool
		minMatches    int
	}{
		{
			name:       "BIC exact match",
			entityName: "Some Bank",
			entityBIC:  "SANCTXXX",
			expectHit:  true,
			minMatches: 1,
		},
		{
			name:       "Name exact match",
			entityName: "SANCTIONED BANK",
			entityBIC:  "",
			expectHit:  true,
			minMatches: 1,
		},
		{
			name:       "No match",
			entityName: "LEGITIMATE BANK",
			entityBIC:  "LEGITXXX",
			expectHit:  false,
			minMatches: 0,
		},
		{
			name:       "Fuzzy name match",
			entityName: "SANCTIONED BANC",
			entityBIC:  "",
			expectHit:  true,
			minMatches: 1,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			matches := screener.screenEntity(tt.entityName, tt.entityBIC, tt.entityCountry, "test")

			if tt.expectHit {
				assert.GreaterOrEqual(t, len(matches), tt.minMatches, "Expected at least %d matches", tt.minMatches)
			} else {
				assert.Equal(t, 0, len(matches), "Expected no matches")
			}
		})
	}
}
