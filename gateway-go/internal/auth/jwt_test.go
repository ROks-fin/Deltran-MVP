package auth

import (
	"testing"
	"time"

	"github.com/golang-jwt/jwt/v5"
)

func TestNewJWTManager(t *testing.T) {
	secret := "test-secret-key"
	manager := NewJWTManager(secret)

	if manager == nil {
		t.Fatal("NewJWTManager() returned nil")
	}
	if string(manager.secretKey) != secret {
		t.Errorf("NewJWTManager() secretKey = %v, want %v", string(manager.secretKey), secret)
	}
}

func TestJWTManager_GenerateTokenPair(t *testing.T) {
	manager := NewJWTManager("test-secret")

	user := &User{
		ID:       "user-123",
		Email:    "test@example.com",
		Username: "testuser",
		Role:     RoleOperator,
	}

	tokenPair, err := manager.GenerateTokenPair(user)
	if err != nil {
		t.Fatalf("GenerateTokenPair() error = %v", err)
	}

	if tokenPair == nil {
		t.Fatal("GenerateTokenPair() returned nil")
	}

	if tokenPair.AccessToken == "" {
		t.Error("GenerateTokenPair() AccessToken is empty")
	}

	if tokenPair.RefreshToken == "" {
		t.Error("GenerateTokenPair() RefreshToken is empty")
	}

	if tokenPair.TokenType != "Bearer" {
		t.Errorf("GenerateTokenPair() TokenType = %v, want %v", tokenPair.TokenType, "Bearer")
	}

	if tokenPair.ExpiresIn <= 0 {
		t.Error("GenerateTokenPair() ExpiresIn should be positive")
	}

	// Verify access token can be validated
	claims, err := manager.ValidateAccessToken(tokenPair.AccessToken)
	if err != nil {
		t.Errorf("ValidateAccessToken() error = %v", err)
	}

	if claims.UserID != user.ID {
		t.Errorf("ValidateAccessToken() UserID = %v, want %v", claims.UserID, user.ID)
	}

	if claims.Role != user.Role {
		t.Errorf("ValidateAccessToken() Role = %v, want %v", claims.Role, user.Role)
	}
}

func TestJWTManager_ValidateAccessToken(t *testing.T) {
	manager := NewJWTManager("test-secret")

	user := &User{
		ID:       "user-456",
		Email:    "test2@example.com",
		Username: "testuser2",
		Role:     RoleAdmin,
	}

	tokenPair, _ := manager.GenerateTokenPair(user)

	tests := []struct {
		name      string
		token     string
		wantErr   bool
		checkFunc func(*Claims) error
	}{
		{
			name:    "valid access token",
			token:   tokenPair.AccessToken,
			wantErr: false,
			checkFunc: func(c *Claims) error {
				if c.UserID != user.ID {
					t.Errorf("UserID = %v, want %v", c.UserID, user.ID)
				}
				if c.Role != user.Role {
					t.Errorf("Role = %v, want %v", c.Role, user.Role)
				}
				if c.TokenType != "access" {
					t.Errorf("TokenType = %v, want %v", c.TokenType, "access")
				}
				return nil
			},
		},
		{
			name:    "refresh token should fail",
			token:   tokenPair.RefreshToken,
			wantErr: true,
		},
		{
			name:    "invalid token",
			token:   "invalid.token.here",
			wantErr: true,
		},
		{
			name:    "empty token",
			token:   "",
			wantErr: true,
		},
		{
			name:    "malformed token",
			token:   "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.invalid",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			claims, err := manager.ValidateAccessToken(tt.token)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateAccessToken() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && tt.checkFunc != nil {
				if err := tt.checkFunc(claims); err != nil {
					t.Error(err)
				}
			}
		})
	}
}

func TestJWTManager_ValidateRefreshToken(t *testing.T) {
	manager := NewJWTManager("test-secret")

	user := &User{
		ID:       "user-789",
		Email:    "test3@example.com",
		Username: "testuser3",
		Role:     RoleAuditor,
	}

	tokenPair, _ := manager.GenerateTokenPair(user)

	tests := []struct {
		name    string
		token   string
		wantErr bool
	}{
		{
			name:    "valid refresh token",
			token:   tokenPair.RefreshToken,
			wantErr: false,
		},
		{
			name:    "access token should fail",
			token:   tokenPair.AccessToken,
			wantErr: true,
		},
		{
			name:    "invalid token",
			token:   "invalid.token.here",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			claims, err := manager.ValidateRefreshToken(tt.token)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateRefreshToken() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr {
				if claims.TokenType != "refresh" {
					t.Errorf("TokenType = %v, want %v", claims.TokenType, "refresh")
				}
			}
		})
	}
}

func TestJWTManager_ExpiredToken(t *testing.T) {
	manager := NewJWTManager("test-secret")

	// Create an expired token
	now := time.Now()
	expiredClaims := &Claims{
		UserID:   "expired-user",
		Role:     RoleViewer,
		TokenType: "access",
		RegisteredClaims: jwt.RegisteredClaims{
			ExpiresAt: jwt.NewNumericDate(now.Add(-1 * time.Hour)),
			IssuedAt:  jwt.NewNumericDate(now.Add(-2 * time.Hour)),
			NotBefore: jwt.NewNumericDate(now.Add(-2 * time.Hour)),
			Subject:   "expired-user",
		},
	}

	token := jwt.NewWithClaims(jwt.SigningMethodHS256, expiredClaims)
	expiredToken, _ := token.SignedString([]byte(manager.secretKey))

	_, err := manager.ValidateAccessToken(expiredToken)
	if err == nil {
		t.Error("ValidateAccessToken() should fail for expired token")
	}
}

func TestJWTManager_WrongSecret(t *testing.T) {
	manager1 := NewJWTManager("secret-1")
	manager2 := NewJWTManager("secret-2")

	user := &User{
		ID:       "user-wrong-secret",
		Email:    "test@example.com",
		Username: "testuser",
		Role:     RoleOperator,
	}

	tokenPair, _ := manager1.GenerateTokenPair(user)

	// Try to validate with different secret
	_, err := manager2.ValidateAccessToken(tokenPair.AccessToken)
	if err == nil {
		t.Error("ValidateAccessToken() should fail when using wrong secret")
	}
}

func TestClaims_HasPermission(t *testing.T) {
	tests := []struct {
		name       string
		role       UserRole
		permission Permission
		want       bool
	}{
		{
			name:       "admin has payment create",
			role:       RoleAdmin,
			permission: PermPaymentCreate,
			want:       true,
		},
		{
			name:       "operator has payment create",
			role:       RoleOperator,
			permission: PermPaymentCreate,
			want:       true,
		},
		{
			name:       "auditor does not have payment create",
			role:       RoleAuditor,
			permission: PermPaymentCreate,
			want:       false,
		},
		{
			name:       "viewer does not have payment create",
			role:       RoleViewer,
			permission: PermPaymentCreate,
			want:       false,
		},
		{
			name:       "admin has user delete",
			role:       RoleAdmin,
			permission: PermUserDelete,
			want:       true,
		},
		{
			name:       "operator does not have user delete",
			role:       RoleOperator,
			permission: PermUserDelete,
			want:       false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			claims := &Claims{
				Role:        tt.role,
				Permissions: RolePermissions[tt.role],
			}

			if got := claims.HasPermission(tt.permission); got != tt.want {
				t.Errorf("HasPermission() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestClaims_HasRole(t *testing.T) {
	tests := []struct {
		name      string
		claimRole UserRole
		checkRole UserRole
		want      bool
	}{
		{
			name:      "admin is admin",
			claimRole: RoleAdmin,
			checkRole: RoleAdmin,
			want:      true,
		},
		{
			name:      "admin is not operator",
			claimRole: RoleAdmin,
			checkRole: RoleOperator,
			want:      false,
		},
		{
			name:      "operator is operator",
			claimRole: RoleOperator,
			checkRole: RoleOperator,
			want:      true,
		},
		{
			name:      "viewer is not admin",
			claimRole: RoleViewer,
			checkRole: RoleAdmin,
			want:      false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			claims := &Claims{
				Role: tt.claimRole,
			}

			if got := claims.HasRole(tt.checkRole); got != tt.want {
				t.Errorf("HasRole() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestRolePermissions(t *testing.T) {
	// Test that all roles have permissions defined
	roles := []UserRole{RoleAdmin, RoleOperator, RoleAuditor, RoleViewer}

	for _, role := range roles {
		t.Run(string(role), func(t *testing.T) {
			perms, exists := RolePermissions[role]
			if !exists {
				t.Errorf("RolePermissions missing entry for role %v", role)
			}
			if len(perms) == 0 {
				t.Errorf("RolePermissions[%v] has no permissions", role)
			}
		})
	}

	// Test admin has all permissions
	t.Run("admin has most permissions", func(t *testing.T) {
		adminPerms := RolePermissions[RoleAdmin]
		if len(adminPerms) < 15 {
			t.Errorf("Admin should have at least 15 permissions, got %v", len(adminPerms))
		}
	})

	// Test viewer has least permissions
	t.Run("viewer has limited permissions", func(t *testing.T) {
		viewerPerms := RolePermissions[RoleViewer]
		if len(viewerPerms) > 5 {
			t.Errorf("Viewer should have limited permissions, got %v", len(viewerPerms))
		}
	})
}

func TestUserRole_String(t *testing.T) {
	tests := []struct {
		role UserRole
		want string
	}{
		{RoleAdmin, "admin"},
		{RoleOperator, "operator"},
		{RoleAuditor, "auditor"},
		{RoleViewer, "viewer"},
	}

	for _, tt := range tests {
		t.Run(string(tt.role), func(t *testing.T) {
			if got := string(tt.role); got != tt.want {
				t.Errorf("UserRole = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestTokenTTL(t *testing.T) {
	// Verify token TTL constants are reasonable
	if AccessTokenTTL < 5*time.Minute {
		t.Error("AccessTokenTTL is too short (< 5 minutes)")
	}
	if AccessTokenTTL > 1*time.Hour {
		t.Error("AccessTokenTTL is too long (> 1 hour)")
	}

	if RefreshTokenTTL < 1*time.Hour {
		t.Error("RefreshTokenTTL is too short (< 1 hour)")
	}
	if RefreshTokenTTL > 30*24*time.Hour {
		t.Error("RefreshTokenTTL is too long (> 30 days)")
	}

	if AccessTokenTTL >= RefreshTokenTTL {
		t.Error("AccessTokenTTL should be shorter than RefreshTokenTTL")
	}
}

func BenchmarkGenerateTokenPair(b *testing.B) {
	manager := NewJWTManager("benchmark-secret")
	user := &User{
		ID:       "bench-user",
		Email:    "bench@example.com",
		Username: "benchuser",
		Role:     RoleOperator,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = manager.GenerateTokenPair(user)
	}
}

func BenchmarkValidateAccessToken(b *testing.B) {
	manager := NewJWTManager("benchmark-secret")
	user := &User{
		ID:       "bench-user",
		Email:    "bench@example.com",
		Username: "benchuser",
		Role:     RoleOperator,
	}

	tokenPair, _ := manager.GenerateTokenPair(user)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = manager.ValidateAccessToken(tokenPair.AccessToken)
	}
}

func BenchmarkHasPermission(b *testing.B) {
	claims := &Claims{
		Role:        RoleOperator,
		Permissions: RolePermissions[RoleOperator],
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = claims.HasPermission(PermPaymentCreate)
	}
}
