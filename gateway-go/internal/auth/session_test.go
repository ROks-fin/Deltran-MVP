package auth

import (
	"context"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"
)

func TestNewSessionManager(t *testing.T) {
	redisClient := redis.NewClient(&redis.Options{Addr: "localhost:6379"})
	defer redisClient.Close()

	manager := NewSessionManager(redisClient)

	if manager == nil {
		t.Fatal("NewSessionManager returned nil")
	}

	if manager.redis == nil {
		t.Error("SessionManager redis client should not be nil")
	}
}

func TestHashToken(t *testing.T) {
	token1 := "test-token-123"
	token2 := "test-token-456"
	token3 := "test-token-123" // Same as token1

	hash1 := hashToken(token1)
	hash2 := hashToken(token2)
	hash3 := hashToken(token3)

	// Hash should be deterministic
	if hash1 != hash3 {
		t.Error("Same token should produce same hash")
	}

	// Different tokens should produce different hashes
	if hash1 == hash2 {
		t.Error("Different tokens should produce different hashes")
	}

	// Hash should be 64 characters (SHA256 hex)
	if len(hash1) != 64 {
		t.Errorf("Hash length = %d, want 64 (SHA256 hex)", len(hash1))
	}
}

func TestFindOldestSession(t *testing.T) {
	now := time.Now()

	tests := []struct {
		name     string
		sessions []*Session
		want     *Session
	}{
		{
			name:     "empty list",
			sessions: []*Session{},
			want:     nil,
		},
		{
			name: "single session",
			sessions: []*Session{
				{ID: "session1", CreatedAt: now, IsRevoked: false},
			},
			want: &Session{ID: "session1", CreatedAt: now, IsRevoked: false},
		},
		{
			name: "multiple sessions",
			sessions: []*Session{
				{ID: "session1", CreatedAt: now.Add(-1 * time.Hour), IsRevoked: false},
				{ID: "session2", CreatedAt: now.Add(-2 * time.Hour), IsRevoked: false},
				{ID: "session3", CreatedAt: now, IsRevoked: false},
			},
			want: &Session{ID: "session2", CreatedAt: now.Add(-2 * time.Hour), IsRevoked: false},
		},
		{
			name: "skip revoked sessions",
			sessions: []*Session{
				{ID: "session1", CreatedAt: now.Add(-1 * time.Hour), IsRevoked: false},
				{ID: "session2", CreatedAt: now.Add(-3 * time.Hour), IsRevoked: true}, // Oldest but revoked
				{ID: "session3", CreatedAt: now, IsRevoked: false},
			},
			want: &Session{ID: "session1", CreatedAt: now.Add(-1 * time.Hour), IsRevoked: false},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got := findOldestSession(tt.sessions)

			if tt.want == nil && got != nil {
				t.Errorf("Expected nil, got %v", got)
			}

			if tt.want != nil && got == nil {
				t.Errorf("Expected %v, got nil", tt.want)
			}

			if tt.want != nil && got != nil && got.ID != tt.want.ID {
				t.Errorf("Expected session %s, got %s", tt.want.ID, got.ID)
			}
		})
	}
}

func TestSession_Structure(t *testing.T) {
	now := time.Now().UTC()
	revokedAt := now.Add(1 * time.Hour)

	session := &Session{
		ID:                "session-123",
		UserID:            "user-456",
		RefreshToken:      "refresh-token",
		RefreshTokenHash:  "hash-value",
		IPAddress:         "192.168.1.1",
		UserAgent:         "Mozilla/5.0",
		DeviceFingerprint: "device-123",
		CreatedAt:         now,
		ExpiresAt:         now.Add(7 * 24 * time.Hour),
		LastActivityAt:    now,
		IsRevoked:         true,
		RevokedAt:         &revokedAt,
		RevokeReason:      "test revoke",
	}

	if session.ID != "session-123" {
		t.Errorf("ID = %s, want session-123", session.ID)
	}

	if session.UserID != "user-456" {
		t.Errorf("UserID = %s, want user-456", session.UserID)
	}

	if !session.IsRevoked {
		t.Error("Session should be revoked")
	}

	if session.RevokedAt == nil {
		t.Error("RevokedAt should not be nil")
	}

	if session.RevokeReason != "test revoke" {
		t.Errorf("RevokeReason = %s, want 'test revoke'", session.RevokeReason)
	}
}

func TestSessionMetadata_Structure(t *testing.T) {
	now := time.Now()

	metadata := &SessionMetadata{
		ID:             "session-123",
		IPAddress:      "192.168.1.1",
		UserAgent:      "Mozilla/5.0",
		CreatedAt:      now,
		LastActivityAt: now,
		ExpiresAt:      now.Add(7 * 24 * time.Hour),
		IsCurrent:      true,
	}

	if metadata.ID != "session-123" {
		t.Errorf("ID = %s, want session-123", metadata.ID)
	}

	if !metadata.IsCurrent {
		t.Error("IsCurrent should be true")
	}

	if metadata.IPAddress != "192.168.1.1" {
		t.Errorf("IPAddress = %s, want 192.168.1.1", metadata.IPAddress)
	}
}

func TestSessionManager_CreateSession_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	session, err := manager.CreateSession(
		ctx,
		"user-123",
		"refresh-token-abc",
		"192.168.1.100",
		"Mozilla/5.0",
		"device-fingerprint-123",
	)

	if err != nil {
		t.Fatalf("CreateSession failed: %v", err)
	}

	if session == nil {
		t.Fatal("Session should not be nil")
	}

	if session.ID == "" {
		t.Error("Session ID should be generated")
	}

	if session.UserID != "user-123" {
		t.Errorf("UserID = %s, want user-123", session.UserID)
	}

	if session.RefreshToken != "refresh-token-abc" {
		t.Error("RefreshToken should be stored")
	}

	if session.RefreshTokenHash == "" {
		t.Error("RefreshTokenHash should be computed")
	}

	if session.IsRevoked {
		t.Error("New session should not be revoked")
	}

	if session.CreatedAt.IsZero() {
		t.Error("CreatedAt should be set")
	}

	if session.ExpiresAt.IsZero() {
		t.Error("ExpiresAt should be set")
	}
}

func TestSessionManager_GetSession_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	// Create session
	created, err := manager.CreateSession(ctx, "user-123", "token", "192.168.1.1", "UA", "device")
	if err != nil {
		t.Fatalf("CreateSession failed: %v", err)
	}

	// Retrieve session
	retrieved, err := manager.GetSession(ctx, created.ID)
	if err != nil {
		t.Fatalf("GetSession failed: %v", err)
	}

	if retrieved.ID != created.ID {
		t.Errorf("Session ID mismatch: got %s, want %s", retrieved.ID, created.ID)
	}

	if retrieved.UserID != "user-123" {
		t.Errorf("UserID = %s, want user-123", retrieved.UserID)
	}
}

func TestSessionManager_GetSession_NotFound(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	_, err := manager.GetSession(ctx, "nonexistent-session")

	if err != ErrSessionNotFound {
		t.Errorf("Expected ErrSessionNotFound, got %v", err)
	}
}

func TestSessionManager_UpdateSessionActivity_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	// Create session
	session, err := manager.CreateSession(ctx, "user-123", "token", "192.168.1.1", "UA", "device")
	if err != nil {
		t.Fatalf("CreateSession failed: %v", err)
	}

	originalActivity := session.LastActivityAt

	// Wait a bit
	time.Sleep(10 * time.Millisecond)

	// Update activity
	err = manager.UpdateSessionActivity(ctx, session.ID)
	if err != nil {
		t.Fatalf("UpdateSessionActivity failed: %v", err)
	}

	// Retrieve and verify
	updated, err := manager.GetSession(ctx, session.ID)
	if err != nil {
		t.Fatalf("GetSession failed: %v", err)
	}

	if !updated.LastActivityAt.After(originalActivity) {
		t.Error("LastActivityAt should be updated")
	}
}

func TestSessionManager_RevokeSession_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	// Create session
	session, err := manager.CreateSession(ctx, "user-123", "token", "192.168.1.1", "UA", "device")
	if err != nil {
		t.Fatalf("CreateSession failed: %v", err)
	}

	// Revoke session
	err = manager.RevokeSession(ctx, session.ID, "user requested logout")
	if err != nil {
		t.Fatalf("RevokeSession failed: %v", err)
	}

	// Try to get session - should return ErrSessionRevoked
	_, err = manager.GetSession(ctx, session.ID)
	if err != ErrSessionRevoked {
		t.Errorf("Expected ErrSessionRevoked, got %v", err)
	}
}

func TestSessionManager_RevokeAllUserSessions_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	userID := "user-multi-session"

	// Create multiple sessions
	session1, _ := manager.CreateSession(ctx, userID, "token1", "192.168.1.1", "UA1", "device1")
	session2, _ := manager.CreateSession(ctx, userID, "token2", "192.168.1.2", "UA2", "device2")
	session3, _ := manager.CreateSession(ctx, userID, "token3", "192.168.1.3", "UA3", "device3")

	// Revoke all
	err := manager.RevokeAllUserSessions(ctx, userID, "security incident")
	if err != nil {
		t.Fatalf("RevokeAllUserSessions failed: %v", err)
	}

	// Check all sessions are revoked
	for _, sessionID := range []string{session1.ID, session2.ID, session3.ID} {
		_, err := manager.GetSession(ctx, sessionID)
		if err != ErrSessionRevoked {
			t.Errorf("Session %s should be revoked, got error: %v", sessionID, err)
		}
	}
}

func TestSessionManager_RevokeOtherSessions_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	userID := "user-keep-current"

	// Create multiple sessions
	session1, _ := manager.CreateSession(ctx, userID, "token1", "192.168.1.1", "UA1", "device1")
	session2, _ := manager.CreateSession(ctx, userID, "token2", "192.168.1.2", "UA2", "device2")
	session3, _ := manager.CreateSession(ctx, userID, "token3", "192.168.1.3", "UA3", "device3")

	// Revoke all except session2
	err := manager.RevokeOtherSessions(ctx, userID, session2.ID, "logout other devices")
	if err != nil {
		t.Fatalf("RevokeOtherSessions failed: %v", err)
	}

	// Session2 should still be active
	retrieved, err := manager.GetSession(ctx, session2.ID)
	if err != nil {
		t.Errorf("Session2 should be active, got error: %v", err)
	}
	if retrieved.IsRevoked {
		t.Error("Session2 should not be revoked")
	}

	// Session1 and Session3 should be revoked
	_, err = manager.GetSession(ctx, session1.ID)
	if err != ErrSessionRevoked {
		t.Errorf("Session1 should be revoked, got: %v", err)
	}

	_, err = manager.GetSession(ctx, session3.ID)
	if err != ErrSessionRevoked {
		t.Errorf("Session3 should be revoked, got: %v", err)
	}
}

func TestSessionManager_GetUserSessions_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	userID := "user-get-sessions"

	// Create sessions
	manager.CreateSession(ctx, userID, "token1", "192.168.1.1", "UA1", "device1")
	manager.CreateSession(ctx, userID, "token2", "192.168.1.2", "UA2", "device2")

	// Get user sessions
	sessions, err := manager.GetUserSessions(ctx, userID)
	if err != nil {
		t.Fatalf("GetUserSessions failed: %v", err)
	}

	if len(sessions) != 2 {
		t.Errorf("Expected 2 sessions, got %d", len(sessions))
	}
}

func TestSessionManager_GetUserSessionsMetadata_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	userID := "user-metadata"

	// Create sessions
	session1, _ := manager.CreateSession(ctx, userID, "token1", "192.168.1.1", "UA1", "device1")
	session2, _ := manager.CreateSession(ctx, userID, "token2", "192.168.1.2", "UA2", "device2")

	// Get metadata with session1 as current
	metadata, err := manager.GetUserSessionsMetadata(ctx, userID, session1.ID)
	if err != nil {
		t.Fatalf("GetUserSessionsMetadata failed: %v", err)
	}

	if len(metadata) != 2 {
		t.Errorf("Expected 2 metadata entries, got %d", len(metadata))
	}

	// Find session1 in metadata
	var found bool
	for _, m := range metadata {
		if m.ID == session1.ID {
			found = true
			if !m.IsCurrent {
				t.Error("Session1 should be marked as current")
			}
		}
		if m.ID == session2.ID {
			if m.IsCurrent {
				t.Error("Session2 should NOT be marked as current")
			}
		}
	}

	if !found {
		t.Error("Session1 should be in metadata")
	}
}

func TestSessionManager_DeleteSession_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	// Create session
	session, err := manager.CreateSession(ctx, "user-delete", "token", "192.168.1.1", "UA", "device")
	if err != nil {
		t.Fatalf("CreateSession failed: %v", err)
	}

	// Delete session
	err = manager.DeleteSession(ctx, session.ID)
	if err != nil {
		t.Fatalf("DeleteSession failed: %v", err)
	}

	// Try to get session - should be not found
	_, err = manager.GetSession(ctx, session.ID)
	if err != ErrSessionNotFound {
		t.Errorf("Expected ErrSessionNotFound, got %v", err)
	}
}

func TestSessionManager_ValidateSessionAndToken_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	token := "my-refresh-token"

	// Create session
	session, err := manager.CreateSession(ctx, "user-validate", token, "192.168.1.1", "UA", "device")
	if err != nil {
		t.Fatalf("CreateSession failed: %v", err)
	}

	// Validate with correct token
	validated, err := manager.ValidateSessionAndToken(ctx, session.ID, token)
	if err != nil {
		t.Fatalf("ValidateSessionAndToken failed: %v", err)
	}

	if validated.ID != session.ID {
		t.Error("Validated session ID mismatch")
	}

	// Validate with wrong token
	_, err = manager.ValidateSessionAndToken(ctx, session.ID, "wrong-token")
	if err == nil {
		t.Error("Validation should fail with wrong token")
	}
}

func TestSessionManager_MaxConcurrentSessions_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewSessionManager(redisClient)

	userID := "user-max-sessions"

	// Create MaxConcurrentSessions sessions
	for i := 0; i < MaxConcurrentSessions; i++ {
		_, err := manager.CreateSession(ctx, userID, "token", "192.168.1.1", "UA", "device")
		if err != nil {
			t.Fatalf("CreateSession %d failed: %v", i, err)
		}
	}

	// Get sessions
	sessions, err := manager.GetUserSessions(ctx, userID)
	if err != nil {
		t.Fatalf("GetUserSessions failed: %v", err)
	}

	activeCount := 0
	for _, s := range sessions {
		if !s.IsRevoked {
			activeCount++
		}
	}

	if activeCount != MaxConcurrentSessions {
		t.Errorf("Expected %d active sessions, got %d", MaxConcurrentSessions, activeCount)
	}

	// Create one more - should revoke oldest
	_, err = manager.CreateSession(ctx, userID, "token-new", "192.168.1.1", "UA", "device")
	if err != nil {
		t.Fatalf("CreateSession (overflow) failed: %v", err)
	}

	// Should still have MaxConcurrentSessions active sessions
	sessions, _ = manager.GetUserSessions(ctx, userID)
	activeCount = 0
	for _, s := range sessions {
		if !s.IsRevoked {
			activeCount++
		}
	}

	if activeCount != MaxConcurrentSessions {
		t.Errorf("After overflow, expected %d active sessions, got %d", MaxConcurrentSessions, activeCount)
	}
}

func TestSessionManager_GetSessionByRefreshToken(t *testing.T) {
	redisClient := redis.NewClient(&redis.Options{Addr: "localhost:6379"})
	defer redisClient.Close()

	manager := NewSessionManager(redisClient)

	_, err := manager.GetSessionByRefreshToken(context.Background(), "any-token")

	// This method is not implemented and should return error
	if err == nil {
		t.Error("GetSessionByRefreshToken should return error (not implemented)")
	}
}

func TestSessionConstants(t *testing.T) {
	if SessionKeyPrefix != "session:" {
		t.Errorf("SessionKeyPrefix = %s, want session:", SessionKeyPrefix)
	}

	if UserSessionsKeyPrefix != "user_sessions:" {
		t.Errorf("UserSessionsKeyPrefix = %s, want user_sessions:", UserSessionsKeyPrefix)
	}

	if MaxConcurrentSessions != 5 {
		t.Errorf("MaxConcurrentSessions = %d, want 5", MaxConcurrentSessions)
	}
}

func TestSessionErrors(t *testing.T) {
	tests := []struct {
		name string
		err  error
		msg  string
	}{
		{"ErrSessionNotFound", ErrSessionNotFound, "session not found"},
		{"ErrSessionExpired", ErrSessionExpired, "session expired"},
		{"ErrTooManySessions", ErrTooManySessions, "too many concurrent sessions"},
		{"ErrSessionRevoked", ErrSessionRevoked, "session has been revoked"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.err == nil {
				t.Errorf("%s should not be nil", tt.name)
			}

			if tt.err.Error() != tt.msg {
				t.Errorf("%s message = %s, want %s", tt.name, tt.err.Error(), tt.msg)
			}
		})
	}
}
