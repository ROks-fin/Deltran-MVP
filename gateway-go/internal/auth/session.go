package auth

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"time"

	"github.com/redis/go-redis/v9"
)

const (
	// SessionKeyPrefix is Redis key prefix for sessions
	SessionKeyPrefix = "session:"
	// UserSessionsKeyPrefix is Redis key prefix for user's sessions list
	UserSessionsKeyPrefix = "user_sessions:"
	// MaxConcurrentSessions is the maximum number of concurrent sessions per user
	MaxConcurrentSessions = 5
)

var (
	ErrSessionNotFound    = errors.New("session not found")
	ErrSessionExpired     = errors.New("session expired")
	ErrTooManySessions    = errors.New("too many concurrent sessions")
	ErrSessionRevoked     = errors.New("session has been revoked")
)

// Session represents a user session
type Session struct {
	ID              string    `json:"id"`
	UserID          string    `json:"user_id"`
	RefreshToken    string    `json:"refresh_token"`
	RefreshTokenHash string   `json:"refresh_token_hash"`

	// Device information
	IPAddress       string    `json:"ip_address"`
	UserAgent       string    `json:"user_agent"`
	DeviceFingerprint string  `json:"device_fingerprint"`

	// Timestamps
	CreatedAt       time.Time `json:"created_at"`
	ExpiresAt       time.Time `json:"expires_at"`
	LastActivityAt  time.Time `json:"last_activity_at"`

	// Status
	IsRevoked       bool      `json:"is_revoked"`
	RevokedAt       *time.Time `json:"revoked_at,omitempty"`
	RevokeReason    string    `json:"revoke_reason,omitempty"`
}

// SessionMetadata represents minimal session info for listing
type SessionMetadata struct {
	ID              string    `json:"id"`
	IPAddress       string    `json:"ip_address"`
	UserAgent       string    `json:"user_agent"`
	CreatedAt       time.Time `json:"created_at"`
	LastActivityAt  time.Time `json:"last_activity_at"`
	ExpiresAt       time.Time `json:"expires_at"`
	IsCurrent       bool      `json:"is_current"`
}

// SessionManager manages user sessions with Redis
type SessionManager struct {
	redis *redis.Client
}

// NewSessionManager creates a new session manager
func NewSessionManager(redisClient *redis.Client) *SessionManager {
	return &SessionManager{
		redis: redisClient,
	}
}

// CreateSession creates a new session
func (s *SessionManager) CreateSession(ctx context.Context, userID, refreshToken string, ipAddress, userAgent, deviceFingerprint string) (*Session, error) {
	// Check concurrent sessions limit
	existingSessions, err := s.GetUserSessions(ctx, userID)
	if err != nil && err != ErrSessionNotFound {
		return nil, fmt.Errorf("failed to check existing sessions: %w", err)
	}

	// Count active (non-revoked) sessions
	activeCount := 0
	for _, sess := range existingSessions {
		if !sess.IsRevoked {
			activeCount++
		}
	}

	if activeCount >= MaxConcurrentSessions {
		// Revoke oldest session
		oldestSession := findOldestSession(existingSessions)
		if oldestSession != nil {
			_ = s.RevokeSession(ctx, oldestSession.ID, "max concurrent sessions exceeded")
		}
	}

	// Create session
	now := time.Now().UTC()
	sessionID := generateTokenID()

	session := &Session{
		ID:                sessionID,
		UserID:            userID,
		RefreshToken:      refreshToken,
		RefreshTokenHash:  hashToken(refreshToken),
		IPAddress:         ipAddress,
		UserAgent:         userAgent,
		DeviceFingerprint: deviceFingerprint,
		CreatedAt:         now,
		ExpiresAt:         now.Add(RefreshTokenTTL),
		LastActivityAt:    now,
		IsRevoked:         false,
	}

	// Store in Redis
	if err := s.saveSession(ctx, session); err != nil {
		return nil, err
	}

	// Add to user's sessions list
	if err := s.addToUserSessions(ctx, userID, sessionID); err != nil {
		return nil, err
	}

	return session, nil
}

// GetSession retrieves session by ID
func (s *SessionManager) GetSession(ctx context.Context, sessionID string) (*Session, error) {
	key := SessionKeyPrefix + sessionID

	data, err := s.redis.Get(ctx, key).Bytes()
	if err != nil {
		if err == redis.Nil {
			return nil, ErrSessionNotFound
		}
		return nil, fmt.Errorf("failed to get session: %w", err)
	}

	var session Session
	if err := json.Unmarshal(data, &session); err != nil {
		return nil, fmt.Errorf("failed to unmarshal session: %w", err)
	}

	// Check if expired
	if time.Now().UTC().After(session.ExpiresAt) {
		return nil, ErrSessionExpired
	}

	// Check if revoked
	if session.IsRevoked {
		return nil, ErrSessionRevoked
	}

	return &session, nil
}

// GetSessionByRefreshToken retrieves session by refresh token
func (s *SessionManager) GetSessionByRefreshToken(ctx context.Context, refreshToken string) (*Session, error) {
	// We need to scan user sessions (this is why we maintain user_sessions list)
	// In production, consider using Redis hash or secondary index

	// For now, return error - caller should use session ID from JWT claims
	return nil, errors.New("lookup by refresh token not implemented - use session ID from JWT")
}

// UpdateSessionActivity updates last activity timestamp
func (s *SessionManager) UpdateSessionActivity(ctx context.Context, sessionID string) error {
	session, err := s.GetSession(ctx, sessionID)
	if err != nil {
		return err
	}

	session.LastActivityAt = time.Now().UTC()

	return s.saveSession(ctx, session)
}

// RevokeSession revokes a session
func (s *SessionManager) RevokeSession(ctx context.Context, sessionID, reason string) error {
	session, err := s.GetSession(ctx, sessionID)
	if err != nil {
		if err == ErrSessionNotFound {
			return nil // Already gone
		}
		return err
	}

	now := time.Now().UTC()
	session.IsRevoked = true
	session.RevokedAt = &now
	session.RevokeReason = reason

	return s.saveSession(ctx, session)
}

// RevokeAllUserSessions revokes all sessions for a user
func (s *SessionManager) RevokeAllUserSessions(ctx context.Context, userID, reason string) error {
	sessions, err := s.GetUserSessions(ctx, userID)
	if err != nil {
		return err
	}

	for _, session := range sessions {
		if !session.IsRevoked {
			if err := s.RevokeSession(ctx, session.ID, reason); err != nil {
				return err
			}
		}
	}

	return nil
}

// RevokeOtherSessions revokes all sessions except the current one
func (s *SessionManager) RevokeOtherSessions(ctx context.Context, userID, currentSessionID, reason string) error {
	sessions, err := s.GetUserSessions(ctx, userID)
	if err != nil {
		return err
	}

	for _, session := range sessions {
		if session.ID != currentSessionID && !session.IsRevoked {
			if err := s.RevokeSession(ctx, session.ID, reason); err != nil {
				return err
			}
		}
	}

	return nil
}

// GetUserSessions retrieves all sessions for a user
func (s *SessionManager) GetUserSessions(ctx context.Context, userID string) ([]*Session, error) {
	key := UserSessionsKeyPrefix + userID

	sessionIDs, err := s.redis.SMembers(ctx, key).Result()
	if err != nil {
		if err == redis.Nil {
			return nil, ErrSessionNotFound
		}
		return nil, fmt.Errorf("failed to get user sessions: %w", err)
	}

	if len(sessionIDs) == 0 {
		return nil, ErrSessionNotFound
	}

	sessions := make([]*Session, 0, len(sessionIDs))
	for _, sessionID := range sessionIDs {
		session, err := s.GetSession(ctx, sessionID)
		if err != nil {
			if err == ErrSessionNotFound || err == ErrSessionExpired {
				// Clean up expired/deleted session from set
				s.redis.SRem(ctx, key, sessionID)
				continue
			}
			// Skip sessions with other errors
			continue
		}
		sessions = append(sessions, session)
	}

	return sessions, nil
}

// GetUserSessionsMetadata retrieves session metadata for user (for UI display)
func (s *SessionManager) GetUserSessionsMetadata(ctx context.Context, userID, currentSessionID string) ([]*SessionMetadata, error) {
	sessions, err := s.GetUserSessions(ctx, userID)
	if err != nil {
		return nil, err
	}

	metadata := make([]*SessionMetadata, 0, len(sessions))
	for _, session := range sessions {
		if !session.IsRevoked {
			metadata = append(metadata, &SessionMetadata{
				ID:             session.ID,
				IPAddress:      session.IPAddress,
				UserAgent:      session.UserAgent,
				CreatedAt:      session.CreatedAt,
				LastActivityAt: session.LastActivityAt,
				ExpiresAt:      session.ExpiresAt,
				IsCurrent:      session.ID == currentSessionID,
			})
		}
	}

	return metadata, nil
}

// DeleteSession permanently deletes a session
func (s *SessionManager) DeleteSession(ctx context.Context, sessionID string) error {
	session, err := s.GetSession(ctx, sessionID)
	if err != nil {
		if err == ErrSessionNotFound {
			return nil
		}
		return err
	}

	// Remove from Redis
	key := SessionKeyPrefix + sessionID
	if err := s.redis.Del(ctx, key).Err(); err != nil {
		return fmt.Errorf("failed to delete session: %w", err)
	}

	// Remove from user's sessions set
	userKey := UserSessionsKeyPrefix + session.UserID
	if err := s.redis.SRem(ctx, userKey, sessionID).Err(); err != nil {
		return fmt.Errorf("failed to remove from user sessions: %w", err)
	}

	return nil
}

// CleanupExpiredSessions removes expired sessions (should be run periodically)
func (s *SessionManager) CleanupExpiredSessions(ctx context.Context, userID string) error {
	sessions, err := s.GetUserSessions(ctx, userID)
	if err != nil {
		if err == ErrSessionNotFound {
			return nil
		}
		return err
	}

	now := time.Now().UTC()
	for _, session := range sessions {
		if now.After(session.ExpiresAt) {
			_ = s.DeleteSession(ctx, session.ID)
		}
	}

	return nil
}

// saveSession saves session to Redis
func (s *SessionManager) saveSession(ctx context.Context, session *Session) error {
	key := SessionKeyPrefix + session.ID

	data, err := json.Marshal(session)
	if err != nil {
		return fmt.Errorf("failed to marshal session: %w", err)
	}

	// Calculate TTL
	ttl := time.Until(session.ExpiresAt)
	if ttl <= 0 {
		return ErrSessionExpired
	}

	if err := s.redis.Set(ctx, key, data, ttl).Err(); err != nil {
		return fmt.Errorf("failed to save session: %w", err)
	}

	return nil
}

// addToUserSessions adds session to user's sessions set
func (s *SessionManager) addToUserSessions(ctx context.Context, userID, sessionID string) error {
	key := UserSessionsKeyPrefix + userID

	if err := s.redis.SAdd(ctx, key, sessionID).Err(); err != nil {
		return fmt.Errorf("failed to add to user sessions: %w", err)
	}

	// Set expiration on the set (refresh on each addition)
	if err := s.redis.Expire(ctx, key, RefreshTokenTTL+24*time.Hour).Err(); err != nil {
		return fmt.Errorf("failed to set expiration: %w", err)
	}

	return nil
}

// hashToken creates SHA256 hash of token
func hashToken(token string) string {
	hash := sha256.Sum256([]byte(token))
	return hex.EncodeToString(hash[:])
}

// findOldestSession finds the oldest session from a list
func findOldestSession(sessions []*Session) *Session {
	if len(sessions) == 0 {
		return nil
	}

	oldest := sessions[0]
	for _, session := range sessions[1:] {
		if !session.IsRevoked && session.CreatedAt.Before(oldest.CreatedAt) {
			oldest = session
		}
	}

	return oldest
}

// ValidateSessionAndToken validates session exists and token matches
func (s *SessionManager) ValidateSessionAndToken(ctx context.Context, sessionID, refreshToken string) (*Session, error) {
	session, err := s.GetSession(ctx, sessionID)
	if err != nil {
		return nil, err
	}

	// Validate token hash
	tokenHash := hashToken(refreshToken)
	if session.RefreshTokenHash != tokenHash {
		return nil, errors.New("token mismatch")
	}

	return session, nil
}
