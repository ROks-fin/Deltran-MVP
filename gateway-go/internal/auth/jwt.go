package auth

import (
	"crypto/rand"
	"encoding/base64"
	"errors"
	"fmt"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

// Configuration
const (
	AccessTokenTTL  = 15 * time.Minute
	RefreshTokenTTL = 7 * 24 * time.Hour
	Issuer          = "deltran"
)

var (
	ErrInvalidToken      = errors.New("invalid token")
	ErrExpiredToken      = errors.New("token expired")
	ErrInvalidSignature  = errors.New("invalid signature")
	ErrInsufficientPerms = errors.New("insufficient permissions")
)

// UserRole represents user role
type UserRole string

const (
	RoleAdmin    UserRole = "admin"
	RoleOperator UserRole = "operator"
	RoleAuditor  UserRole = "auditor"
	RoleViewer   UserRole = "viewer"
)

// Permission represents a specific permission
type Permission string

const (
	// Payment permissions
	PermPaymentCreate Permission = "payment:create"
	PermPaymentRead   Permission = "payment:read"
	PermPaymentUpdate Permission = "payment:update"
	PermPaymentCancel Permission = "payment:cancel"

	// Batch permissions
	PermBatchCreate   Permission = "batch:create"
	PermBatchRead     Permission = "batch:read"
	PermBatchFinalize Permission = "batch:finalize"

	// Compliance permissions
	PermComplianceCheck  Permission = "compliance:check"
	PermComplianceReview Permission = "compliance:review"

	// User management permissions
	PermUserCreate Permission = "user:create"
	PermUserRead   Permission = "user:read"
	PermUserUpdate Permission = "user:update"
	PermUserDelete Permission = "user:delete"

	// Limit management permissions
	PermLimitCreate Permission = "limit:create"
	PermLimitUpdate Permission = "limit:update"
	PermLimitRead   Permission = "limit:read"

	// System permissions
	PermSystemConfig Permission = "system:config"
	PermSystemAudit  Permission = "system:audit"
)

// RolePermissions defines permissions for each role
var RolePermissions = map[UserRole][]Permission{
	RoleAdmin: {
		// All permissions
		PermPaymentCreate, PermPaymentRead, PermPaymentUpdate, PermPaymentCancel,
		PermBatchCreate, PermBatchRead, PermBatchFinalize,
		PermComplianceCheck, PermComplianceReview,
		PermUserCreate, PermUserRead, PermUserUpdate, PermUserDelete,
		PermLimitCreate, PermLimitUpdate, PermLimitRead,
		PermSystemConfig, PermSystemAudit,
	},
	RoleOperator: {
		// Payment operations
		PermPaymentCreate, PermPaymentRead, PermPaymentUpdate, PermPaymentCancel,
		PermBatchCreate, PermBatchRead, PermBatchFinalize,
		PermComplianceCheck,
		PermLimitRead,
	},
	RoleAuditor: {
		// Read-only access
		PermPaymentRead,
		PermBatchRead,
		PermUserRead,
		PermLimitRead,
		PermSystemAudit,
	},
	RoleViewer: {
		// Minimal read access
		PermPaymentRead,
		PermBatchRead,
	},
}

// Claims represents JWT claims
type Claims struct {
	UserID      string       `json:"user_id"`
	Email       string       `json:"email"`
	Username    string       `json:"username"`
	Role        UserRole     `json:"role"`
	BankID      string       `json:"bank_id,omitempty"`
	Permissions []Permission `json:"permissions"`
	TokenType   string       `json:"token_type"` // "access" or "refresh"
	jwt.RegisteredClaims
}

// TokenPair represents access and refresh token pair
type TokenPair struct {
	AccessToken  string    `json:"access_token"`
	RefreshToken string    `json:"refresh_token"`
	TokenType    string    `json:"token_type"` // "Bearer"
	ExpiresIn    int64     `json:"expires_in"` // seconds
	ExpiresAt    time.Time `json:"expires_at"`
}

// User represents authenticated user
type User struct {
	ID       string
	Email    string
	Username string
	Role     UserRole
	BankID   string
}

// JWTManager manages JWT token creation and validation
type JWTManager struct {
	secretKey []byte
}

// NewJWTManager creates a new JWT manager
func NewJWTManager(secret string) *JWTManager {
	return &JWTManager{
		secretKey: []byte(secret),
	}
}

// GenerateTokenPair generates access and refresh token pair
func (j *JWTManager) GenerateTokenPair(user *User) (*TokenPair, error) {
	now := time.Now()
	accessExpiry := now.Add(AccessTokenTTL)
	refreshExpiry := now.Add(RefreshTokenTTL)

	// Get permissions for user role
	permissions := RolePermissions[user.Role]

	// Create access token
	accessClaims := Claims{
		UserID:      user.ID,
		Email:       user.Email,
		Username:    user.Username,
		Role:        user.Role,
		BankID:      user.BankID,
		Permissions: permissions,
		TokenType:   "access",
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(accessExpiry),
			IssuedAt:  jwt.NewNumericDate(now),
			NotBefore: jwt.NewNumericDate(now),
			Issuer:    Issuer,
			Subject:   user.ID,
			ID:        generateTokenID(),
		},
	}

	accessToken := jwt.NewWithClaims(jwt.SigningMethodHS256, accessClaims)
	accessTokenString, err := accessToken.SignedString(j.secretKey)
	if err != nil {
		return nil, fmt.Errorf("failed to sign access token: %w", err)
	}

	// Create refresh token (with minimal claims)
	refreshClaims := Claims{
		UserID:    user.ID,
		TokenType: "refresh",
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(refreshExpiry),
			IssuedAt:  jwt.NewNumericDate(now),
			NotBefore: jwt.NewNumericDate(now),
			Issuer:    Issuer,
			Subject:   user.ID,
			ID:        generateTokenID(),
		},
	}

	refreshToken := jwt.NewWithClaims(jwt.SigningMethodHS256, refreshClaims)
	refreshTokenString, err := refreshToken.SignedString(j.secretKey)
	if err != nil {
		return nil, fmt.Errorf("failed to sign refresh token: %w", err)
	}

	return &TokenPair{
		AccessToken:  accessTokenString,
		RefreshToken: refreshTokenString,
		TokenType:    "Bearer",
		ExpiresIn:    int64(AccessTokenTTL.Seconds()),
		ExpiresAt:    accessExpiry,
	}, nil
}

// ValidateAccessToken validates and parses access token
func (j *JWTManager) ValidateAccessToken(tokenString string) (*Claims, error) {
	token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(token *jwt.Token) (interface{}, error) {
		// Verify signing method
		if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
		}
		return j.secretKey, nil
	})

	if err != nil {
		if errors.Is(err, jwt.ErrTokenExpired) {
			return nil, ErrExpiredToken
		}
		if errors.Is(err, jwt.ErrSignatureInvalid) {
			return nil, ErrInvalidSignature
		}
		return nil, fmt.Errorf("%w: %v", ErrInvalidToken, err)
	}

	claims, ok := token.Claims.(*Claims)
	if !ok || !token.Valid {
		return nil, ErrInvalidToken
	}

	// Verify token type
	if claims.TokenType != "access" {
		return nil, fmt.Errorf("invalid token type: %s", claims.TokenType)
	}

	return claims, nil
}

// ValidateRefreshToken validates and parses refresh token
func (j *JWTManager) ValidateRefreshToken(tokenString string) (*Claims, error) {
	token, err := jwt.ParseWithClaims(tokenString, &Claims{}, func(token *jwt.Token) (interface{}, error) {
		if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
			return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
		}
		return j.secretKey, nil
	})

	if err != nil {
		if errors.Is(err, jwt.ErrTokenExpired) {
			return nil, ErrExpiredToken
		}
		if errors.Is(err, jwt.ErrSignatureInvalid) {
			return nil, ErrInvalidSignature
		}
		return nil, fmt.Errorf("%w: %v", ErrInvalidToken, err)
	}

	claims, ok := token.Claims.(*Claims)
	if !ok || !token.Valid {
		return nil, ErrInvalidToken
	}

	// Verify token type
	if claims.TokenType != "refresh" {
		return nil, fmt.Errorf("invalid token type: %s", claims.TokenType)
	}

	return claims, nil
}

// RefreshAccessToken generates new access token from refresh token
func (j *JWTManager) RefreshAccessToken(refreshTokenString string, user *User) (*TokenPair, error) {
	// Validate refresh token
	_, err := j.ValidateRefreshToken(refreshTokenString)
	if err != nil {
		return nil, err
	}

	// Generate new token pair
	return j.GenerateTokenPair(user)
}

// HasPermission checks if claims have specific permission
func (c *Claims) HasPermission(perm Permission) bool {
	for _, p := range c.Permissions {
		if p == perm {
			return true
		}
	}
	return false
}

// HasAnyPermission checks if claims have any of the specified permissions
func (c *Claims) HasAnyPermission(perms ...Permission) bool {
	for _, perm := range perms {
		if c.HasPermission(perm) {
			return true
		}
	}
	return false
}

// HasAllPermissions checks if claims have all of the specified permissions
func (c *Claims) HasAllPermissions(perms ...Permission) bool {
	for _, perm := range perms {
		if !c.HasPermission(perm) {
			return false
		}
	}
	return true
}

// IsAdmin checks if user is admin
func (c *Claims) IsAdmin() bool {
	return c.Role == RoleAdmin
}

// IsOperator checks if user is operator or admin
func (c *Claims) IsOperator() bool {
	return c.Role == RoleOperator || c.Role == RoleAdmin
}

// IsAuditor checks if user is auditor
func (c *Claims) IsAuditor() bool {
	return c.Role == RoleAuditor
}

// HasRole checks if user has specific role
func (c *Claims) HasRole(role UserRole) bool {
	return c.Role == role
}

// generateTokenID generates random token ID
func generateTokenID() string {
	b := make([]byte, 16)
	if _, err := rand.Read(b); err != nil {
		// Fallback to timestamp-based ID
		return fmt.Sprintf("%d", time.Now().UnixNano())
	}
	return base64.RawURLEncoding.EncodeToString(b)
}

// ExtractBearerToken extracts token from Authorization header
func ExtractBearerToken(authHeader string) (string, error) {
	if authHeader == "" {
		return "", errors.New("authorization header is empty")
	}

	const prefix = "Bearer "
	if len(authHeader) < len(prefix) {
		return "", errors.New("invalid authorization header format")
	}

	if authHeader[:len(prefix)] != prefix {
		return "", errors.New("authorization header must use Bearer scheme")
	}

	return authHeader[len(prefix):], nil
}
