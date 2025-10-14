package auth

import (
	"context"
	"database/sql"
	"encoding/json"
	"net/http"
	"time"
)

// AuthHandler handles authentication-related HTTP requests
type AuthHandler struct {
	db             *sql.DB
	jwtManager     *JWTManager
	totpManager    *TOTPManager
	sessionManager *SessionManager
	passwordValidator *PasswordValidator
}

// NewAuthHandler creates a new auth handler
func NewAuthHandler(db *sql.DB, jwtManager *JWTManager, totpManager *TOTPManager, sessionManager *SessionManager) *AuthHandler {
	return &AuthHandler{
		db:                db,
		jwtManager:        jwtManager,
		totpManager:       totpManager,
		sessionManager:    sessionManager,
		passwordValidator: NewPasswordValidator(),
	}
}

// LoginRequest represents login request
type LoginRequest struct {
	Email    string `json:"email"`
	Password string `json:"password"`
	TOTPCode string `json:"totp_code,omitempty"`
}

// LoginResponse represents login response
type LoginResponse struct {
	AccessToken  string    `json:"access_token"`
	RefreshToken string    `json:"refresh_token"`
	TokenType    string    `json:"token_type"`
	ExpiresIn    int64     `json:"expires_in"`
	ExpiresAt    time.Time `json:"expires_at"`
	User         UserInfo  `json:"user"`
	Requires2FA  bool      `json:"requires_2fa"`
}

// UserInfo represents user information
type UserInfo struct {
	ID       string   `json:"id"`
	Email    string   `json:"email"`
	Username string   `json:"username"`
	Role     UserRole `json:"role"`
	BankID   string   `json:"bank_id,omitempty"`
}

// Login handles user login
func (h *AuthHandler) Login(w http.ResponseWriter, r *http.Request) {
	var req LoginRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, "invalid request body", http.StatusBadRequest)
		return
	}

	// Get user from database
	user, err := h.getUserByEmail(r.Context(), req.Email)
	if err != nil {
		if err == sql.ErrNoRows {
			writeError(w, "invalid credentials", http.StatusUnauthorized)
			return
		}
		writeError(w, "internal server error", http.StatusInternalServerError)
		return
	}

	// Check if account is locked
	if user.IsLocked() {
		writeError(w, "account is locked", http.StatusForbidden)
		return
	}

	// Verify password
	if err := VerifyPassword(req.Password, user.PasswordHash); err != nil {
		// Increment failed login attempts
		h.incrementFailedAttempts(r.Context(), user.ID)
		writeError(w, "invalid credentials", http.StatusUnauthorized)
		return
	}

	// Check 2FA if enabled
	if user.Is2FAEnabled {
		if req.TOTPCode == "" {
			// Return response indicating 2FA is required
			w.Header().Set("Content-Type", "application/json")
			w.WriteHeader(http.StatusOK)
			json.NewEncoder(w).Encode(map[string]interface{}{
				"requires_2fa": true,
				"message":      "TOTP code required",
			})
			return
		}

		// Validate TOTP code
		valid, err := h.totpManager.ValidateCode(user.TOTPSecret, req.TOTPCode)
		if err != nil || !valid {
			writeError(w, "invalid TOTP code", http.StatusUnauthorized)
			return
		}
	}

	// Reset failed attempts on successful login
	h.resetFailedAttempts(r.Context(), user.ID)

	// Generate token pair
	tokenPair, err := h.jwtManager.GenerateTokenPair(&User{
		ID:       user.ID,
		Email:    user.Email,
		Username: user.Username,
		Role:     user.Role,
		BankID:   user.BankID,
	})
	if err != nil {
		writeError(w, "failed to generate tokens", http.StatusInternalServerError)
		return
	}

	// Create session
	ipAddress := getClientIP(r)
	userAgent := r.Header.Get("User-Agent")
	deviceFingerprint := r.Header.Get("X-Device-Fingerprint")

	_, err = h.sessionManager.CreateSession(
		r.Context(),
		user.ID,
		tokenPair.RefreshToken,
		ipAddress,
		userAgent,
		deviceFingerprint,
	)
	if err != nil {
		writeError(w, "failed to create session", http.StatusInternalServerError)
		return
	}

	// Update last login
	h.updateLastLogin(r.Context(), user.ID, ipAddress)

	// Return response
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(LoginResponse{
		AccessToken:  tokenPair.AccessToken,
		RefreshToken: tokenPair.RefreshToken,
		TokenType:    tokenPair.TokenType,
		ExpiresIn:    tokenPair.ExpiresIn,
		ExpiresAt:    tokenPair.ExpiresAt,
		User: UserInfo{
			ID:       user.ID,
			Email:    user.Email,
			Username: user.Username,
			Role:     user.Role,
			BankID:   user.BankID,
		},
		Requires2FA: false,
	})
}

// RefreshTokenRequest represents refresh token request
type RefreshTokenRequest struct {
	RefreshToken string `json:"refresh_token"`
}

// RefreshToken handles token refresh
func (h *AuthHandler) RefreshToken(w http.ResponseWriter, r *http.Request) {
	var req RefreshTokenRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, "invalid request body", http.StatusBadRequest)
		return
	}

	// Validate refresh token
	claims, err := h.jwtManager.ValidateRefreshToken(req.RefreshToken)
	if err != nil {
		writeError(w, "invalid refresh token", http.StatusUnauthorized)
		return
	}

	// Validate session exists and matches
	session, err := h.sessionManager.ValidateSessionAndToken(r.Context(), claims.ID, req.RefreshToken)
	if err != nil {
		writeError(w, "invalid session", http.StatusUnauthorized)
		return
	}

	// Get user
	user, err := h.getUserByID(r.Context(), claims.UserID)
	if err != nil {
		writeError(w, "user not found", http.StatusUnauthorized)
		return
	}

	// Check if user is still active
	if !user.IsActive {
		writeError(w, "account is inactive", http.StatusForbidden)
		return
	}

	// Generate new token pair
	tokenPair, err := h.jwtManager.GenerateTokenPair(&User{
		ID:       user.ID,
		Email:    user.Email,
		Username: user.Username,
		Role:     user.Role,
		BankID:   user.BankID,
	})
	if err != nil {
		writeError(w, "failed to generate tokens", http.StatusInternalServerError)
		return
	}

	// Update session with new refresh token
	session.RefreshToken = tokenPair.RefreshToken
	session.RefreshTokenHash = hashToken(tokenPair.RefreshToken)
	session.LastActivityAt = time.Now().UTC()

	// Save updated session (implemented in session.go)
	// This would need to be added to SessionManager

	// Return response
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(tokenPair)
}

// Logout handles user logout
func (h *AuthHandler) Logout(w http.ResponseWriter, r *http.Request) {
	// Get claims from context
	claims, ok := GetClaims(r)
	if !ok {
		writeError(w, "unauthorized", http.StatusUnauthorized)
		return
	}

	// Revoke session
	if err := h.sessionManager.RevokeSession(r.Context(), claims.ID, "user logout"); err != nil {
		writeError(w, "failed to logout", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// LogoutAll handles logout from all devices
func (h *AuthHandler) LogoutAll(w http.ResponseWriter, r *http.Request) {
	claims := MustGetClaims(r)

	if err := h.sessionManager.RevokeAllUserSessions(r.Context(), claims.UserID, "logout all devices"); err != nil {
		writeError(w, "failed to logout from all devices", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Setup2FARequest represents 2FA setup request
type Setup2FARequest struct{}

// Setup2FAResponse represents 2FA setup response
type Setup2FAResponse struct {
	Secret      string   `json:"secret"`
	QRCodeURL   string   `json:"qr_code_url"`
	BackupCodes []string `json:"backup_codes"`
}

// Setup2FA initiates 2FA setup
func (h *AuthHandler) Setup2FA(w http.ResponseWriter, r *http.Request) {
	claims := MustGetClaims(r)

	// Generate TOTP setup
	setup, err := h.totpManager.SetupTOTP(claims.Email)
	if err != nil {
		writeError(w, "failed to setup 2FA", http.StatusInternalServerError)
		return
	}

	// Store secret temporarily (don't enable 2FA until verified)
	// This would need to be stored in a temporary table or Redis

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(Setup2FAResponse{
		Secret:      setup.Secret,
		QRCodeURL:   setup.QRCodeURL,
		BackupCodes: setup.BackupCodes,
	})
}

// Verify2FARequest represents 2FA verification request
type Verify2FARequest struct {
	Secret   string `json:"secret"`
	TOTPCode string `json:"totp_code"`
}

// Verify2FA verifies and enables 2FA
func (h *AuthHandler) Verify2FA(w http.ResponseWriter, r *http.Request) {
	claims := MustGetClaims(r)

	var req Verify2FARequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		writeError(w, "invalid request body", http.StatusBadRequest)
		return
	}

	// Verify TOTP code
	if err := h.totpManager.VerifyTOTPSetup(req.Secret, req.TOTPCode); err != nil {
		writeError(w, "invalid TOTP code", http.StatusBadRequest)
		return
	}

	// Enable 2FA for user
	if err := h.enable2FA(r.Context(), claims.UserID, req.Secret); err != nil {
		writeError(w, "failed to enable 2FA", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{
		"message": "2FA enabled successfully",
	})
}

// Disable2FA disables 2FA for user
func (h *AuthHandler) Disable2FA(w http.ResponseWriter, r *http.Request) {
	claims := MustGetClaims(r)

	if err := h.disable2FA(r.Context(), claims.UserID); err != nil {
		writeError(w, "failed to disable 2FA", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]string{
		"message": "2FA disabled successfully",
	})
}

// GetSessions returns user's active sessions
func (h *AuthHandler) GetSessions(w http.ResponseWriter, r *http.Request) {
	claims := MustGetClaims(r)

	sessions, err := h.sessionManager.GetUserSessionsMetadata(r.Context(), claims.UserID, claims.ID)
	if err != nil {
		writeError(w, "failed to get sessions", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(map[string]interface{}{
		"sessions": sessions,
	})
}

// RevokeSession revokes a specific session
func (h *AuthHandler) RevokeSession(w http.ResponseWriter, r *http.Request) {
	claims := MustGetClaims(r)

	sessionID := r.URL.Query().Get("session_id")
	if sessionID == "" {
		writeError(w, "session_id required", http.StatusBadRequest)
		return
	}

	// Verify session belongs to user
	session, err := h.sessionManager.GetSession(r.Context(), sessionID)
	if err != nil {
		writeError(w, "session not found", http.StatusNotFound)
		return
	}

	if session.UserID != claims.UserID {
		writeError(w, "forbidden", http.StatusForbidden)
		return
	}

	if err := h.sessionManager.RevokeSession(r.Context(), sessionID, "revoked by user"); err != nil {
		writeError(w, "failed to revoke session", http.StatusInternalServerError)
		return
	}

	w.WriteHeader(http.StatusNoContent)
}

// Database helper methods

type dbUser struct {
	ID               string
	Email            string
	Username         string
	PasswordHash     string
	Role             UserRole
	BankID           string
	IsActive         bool
	Is2FAEnabled     bool
	TOTPSecret       string
	FailedAttempts   int
	LockedUntil      *time.Time
	LastLoginAt      *time.Time
	LastLoginIP      string
}

func (u *dbUser) IsLocked() bool {
	if u.LockedUntil == nil {
		return false
	}
	return time.Now().UTC().Before(*u.LockedUntil)
}

func (h *AuthHandler) getUserByEmail(ctx context.Context, email string) (*dbUser, error) {
	query := `
		SELECT id, email, username, password_hash, role, COALESCE(bank_id, ''),
		       is_active, is_2fa_enabled, COALESCE(totp_secret, ''),
		       failed_login_attempts, locked_until, last_login_at, COALESCE(last_login_ip, '')
		FROM deltran.users
		WHERE email = $1
	`

	var user dbUser
	err := h.db.QueryRowContext(ctx, query, email).Scan(
		&user.ID, &user.Email, &user.Username, &user.PasswordHash, &user.Role, &user.BankID,
		&user.IsActive, &user.Is2FAEnabled, &user.TOTPSecret,
		&user.FailedAttempts, &user.LockedUntil, &user.LastLoginAt, &user.LastLoginIP,
	)

	if err != nil {
		return nil, err
	}

	return &user, nil
}

func (h *AuthHandler) getUserByID(ctx context.Context, userID string) (*dbUser, error) {
	query := `
		SELECT id, email, username, password_hash, role, COALESCE(bank_id, ''),
		       is_active, is_2fa_enabled, COALESCE(totp_secret, ''),
		       failed_login_attempts, locked_until, last_login_at, COALESCE(last_login_ip, '')
		FROM deltran.users
		WHERE id = $1
	`

	var user dbUser
	err := h.db.QueryRowContext(ctx, query, userID).Scan(
		&user.ID, &user.Email, &user.Username, &user.PasswordHash, &user.Role, &user.BankID,
		&user.IsActive, &user.Is2FAEnabled, &user.TOTPSecret,
		&user.FailedAttempts, &user.LockedUntil, &user.LastLoginAt, &user.LastLoginIP,
	)

	if err != nil {
		return nil, err
	}

	return &user, nil
}

func (h *AuthHandler) incrementFailedAttempts(ctx context.Context, userID string) error {
	query := `
		UPDATE deltran.users
		SET failed_login_attempts = failed_login_attempts + 1,
		    locked_until = CASE
		        WHEN failed_login_attempts + 1 >= 5 THEN NOW() + INTERVAL '30 minutes'
		        ELSE locked_until
		    END
		WHERE id = $1
	`
	_, err := h.db.ExecContext(ctx, query, userID)
	return err
}

func (h *AuthHandler) resetFailedAttempts(ctx context.Context, userID string) error {
	query := `
		UPDATE deltran.users
		SET failed_login_attempts = 0,
		    locked_until = NULL
		WHERE id = $1
	`
	_, err := h.db.ExecContext(ctx, query, userID)
	return err
}

func (h *AuthHandler) updateLastLogin(ctx context.Context, userID, ipAddress string) error {
	query := `
		UPDATE deltran.users
		SET last_login_at = NOW(),
		    last_login_ip = $2
		WHERE id = $1
	`
	_, err := h.db.ExecContext(ctx, query, userID, ipAddress)
	return err
}

func (h *AuthHandler) enable2FA(ctx context.Context, userID, totpSecret string) error {
	query := `
		UPDATE deltran.users
		SET is_2fa_enabled = true,
		    totp_secret = $2
		WHERE id = $1
	`
	_, err := h.db.ExecContext(ctx, query, userID, totpSecret)
	return err
}

func (h *AuthHandler) disable2FA(ctx context.Context, userID string) error {
	query := `
		UPDATE deltran.users
		SET is_2fa_enabled = false,
		    totp_secret = NULL
		WHERE id = $1
	`
	_, err := h.db.ExecContext(ctx, query, userID)
	return err
}
