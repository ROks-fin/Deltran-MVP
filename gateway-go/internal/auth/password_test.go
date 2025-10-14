package auth

import (
	"strings"
	"testing"

	"golang.org/x/crypto/bcrypt"
)

func TestHashPassword(t *testing.T) {
	tests := []struct {
		name     string
		password string
		wantErr  bool
	}{
		{
			name:     "valid password",
			password: "SecurePassword123!",
			wantErr:  false,
		},
		{
			name:     "short password",
			password: "short",
			wantErr:  false, // hashing should work regardless
		},
		{
			name:     "empty password",
			password: "",
			wantErr:  false, // bcrypt can hash empty string
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			hash, err := HashPassword(tt.password)
			if (err != nil) != tt.wantErr {
				t.Errorf("HashPassword() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr {
				// Verify hash starts with bcrypt prefix
				if !strings.HasPrefix(hash, "$2a$") && !strings.HasPrefix(hash, "$2b$") {
					t.Errorf("HashPassword() returned invalid bcrypt hash")
				}
				// Verify we can compare
				err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(tt.password))
				if err != nil {
					t.Errorf("HashPassword() produced hash that doesn't match original password")
				}
			}
		})
	}
}

func TestVerifyPassword(t *testing.T) {
	correctPassword := "CorrectPassword123!"
	correctHash, _ := HashPassword(correctPassword)

	tests := []struct {
		name     string
		password string
		hash     string
		wantErr  bool
	}{
		{
			name:     "correct password",
			password: correctPassword,
			hash:     correctHash,
			wantErr:  false,
		},
		{
			name:     "incorrect password",
			password: "WrongPassword123!",
			hash:     correctHash,
			wantErr:  true,
		},
		{
			name:     "empty password",
			password: "",
			hash:     correctHash,
			wantErr:  true,
		},
		{
			name:     "invalid hash",
			password: correctPassword,
			hash:     "invalid-hash",
			wantErr:  true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := VerifyPassword(tt.password, tt.hash)
			if (err != nil) != tt.wantErr {
				t.Errorf("VerifyPassword() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestPasswordValidator_ValidatePassword(t *testing.T) {
	validator := NewPasswordValidator()

	tests := []struct {
		name     string
		password string
		wantErr  bool
	}{
		{
			name:     "valid strong password",
			password: "SecurePass123!",
			wantErr:  false,
		},
		{
			name:     "too short",
			password: "Abc1!",
			wantErr:  true,
		},
		{
			name:     "too long",
			password: strings.Repeat("A", 129) + "bc1!",
			wantErr:  true,
		},
		{
			name:     "no uppercase",
			password: "password123!",
			wantErr:  true,
		},
		{
			name:     "no lowercase",
			password: "PASSWORD123!",
			wantErr:  true,
		},
		{
			name:     "no digit",
			password: "PasswordABC!",
			wantErr:  true,
		},
		{
			name:     "no special character",
			password: "Password123",
			wantErr:  true,
		},
		{
			name:     "common password",
			password: "password",
			wantErr:  true,
		},
		{
			name:     "common password 2",
			password: "123456",
			wantErr:  true,
		},
		{
			name:     "minimum length valid",
			password: "Abcd123!",
			wantErr:  false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := validator.ValidatePassword(tt.password)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidatePassword() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestPasswordValidator_CheckPasswordStrength(t *testing.T) {
	validator := NewPasswordValidator()

	tests := []struct {
		name     string
		password string
		minLevel PasswordStrength
		maxLevel PasswordStrength
	}{
		{
			name:     "weak password",
			password: "abc",
			minLevel: PasswordWeak,
			maxLevel: PasswordWeak,
		},
		{
			name:     "fair password",
			password: "Abcd1234",
			minLevel: PasswordFair,
			maxLevel: PasswordGood,
		},
		{
			name:     "good password",
			password: "Abcd1234!",
			minLevel: PasswordGood,
			maxLevel: PasswordStrong,
		},
		{
			name:     "strong password",
			password: "SecurePass123!",
			minLevel: PasswordGood,
			maxLevel: PasswordStrong,
		},
		{
			name:     "very strong password",
			password: "V3ry$ecur3P@ssw0rd!2024",
			minLevel: PasswordStrong,
			maxLevel: PasswordVeryStrong,
		},
		{
			name:     "common password is weak",
			password: "password",
			minLevel: PasswordWeak,
			maxLevel: PasswordWeak,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			strength := validator.CheckPasswordStrength(tt.password)
			if strength < tt.minLevel || strength > tt.maxLevel {
				t.Errorf("CheckPasswordStrength() = %v, want between %v and %v",
					strength, tt.minLevel, tt.maxLevel)
			}
		})
	}
}

func TestHasCommonPattern(t *testing.T) {
	tests := []struct {
		name     string
		password string
		want     bool
	}{
		{
			name:     "sequential numbers",
			password: "12345678",
			want:     true,
		},
		{
			name:     "keyboard pattern qwerty",
			password: "qwertyqwerty",
			want:     true,
		},
		{
			name:     "keyboard pattern asdfgh",
			password: "asdfghasdfgh",
			want:     true,
		},
		{
			name:     "no common pattern",
			password: "SecurePass123!",
			want:     false,
		},
		{
			name:     "random string",
			password: "xJ8k#mP2qL9",
			want:     false,
		},
		{
			name:     "complex secure password",
			password: "G7#pQm2$kL9x",
			want:     false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := hasCommonPattern(tt.password); got != tt.want {
				t.Errorf("hasCommonPattern() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestGenerateRandomPassword(t *testing.T) {
	tests := []struct {
		name   string
		length int
	}{
		{
			name:   "minimum length",
			length: MinPasswordLength,
		},
		{
			name:   "medium length",
			length: 16,
		},
		{
			name:   "maximum length",
			length: MaxPasswordLength,
		},
		{
			name:   "below minimum (should use minimum)",
			length: 4,
		},
		{
			name:   "above maximum (should use maximum)",
			length: 200,
		},
	}

	validator := NewPasswordValidator()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			password, err := GenerateRandomPassword(tt.length)
			if err != nil {
				t.Fatalf("GenerateRandomPassword() error = %v", err)
			}

			// Check length is within bounds
			if len(password) < MinPasswordLength || len(password) > MaxPasswordLength {
				t.Errorf("GenerateRandomPassword() length = %v, want between %v and %v",
					len(password), MinPasswordLength, MaxPasswordLength)
			}

			// Validate that generated password meets complexity requirements
			if err := validator.ValidatePassword(password); err != nil {
				t.Errorf("GenerateRandomPassword() produced invalid password: %v", err)
			}

			// Check that password contains required character types
			hasUpper := false
			hasLower := false
			hasDigit := false
			hasSpecial := false

			for _, char := range password {
				if char >= 'A' && char <= 'Z' {
					hasUpper = true
				} else if char >= 'a' && char <= 'z' {
					hasLower = true
				} else if char >= '0' && char <= '9' {
					hasDigit = true
				} else {
					hasSpecial = true
				}
			}

			if !hasUpper || !hasLower || !hasDigit || !hasSpecial {
				t.Errorf("GenerateRandomPassword() missing required character types: upper=%v, lower=%v, digit=%v, special=%v",
					hasUpper, hasLower, hasDigit, hasSpecial)
			}
		})
	}
}

func TestGenerateRandomPassword_Uniqueness(t *testing.T) {
	// Generate 100 passwords and ensure they're all unique
	passwords := make(map[string]bool)
	count := 100

	for i := 0; i < count; i++ {
		password, err := GenerateRandomPassword(16)
		if err != nil {
			t.Fatalf("GenerateRandomPassword() error = %v", err)
		}
		if passwords[password] {
			t.Errorf("GenerateRandomPassword() generated duplicate password")
		}
		passwords[password] = true
	}

	if len(passwords) != count {
		t.Errorf("GenerateRandomPassword() generated %v unique passwords, want %v", len(passwords), count)
	}
}

func TestValidateWithPolicy(t *testing.T) {
	tests := []struct {
		name     string
		password string
		policy   *PasswordPolicy
		wantErr  bool
	}{
		{
			name:     "valid with default policy",
			password: "SecurePass123!",
			policy:   DefaultPasswordPolicy(),
			wantErr:  false,
		},
		{
			name:     "fails min length",
			password: "Abc1!",
			policy:   DefaultPasswordPolicy(),
			wantErr:  true,
		},
		{
			name:     "valid with lenient policy",
			password: "simple",
			policy: &PasswordPolicy{
				MinLength:        6,
				MaxLength:        20,
				RequireUppercase: false,
				RequireLowercase: true,
				RequireDigit:     false,
				RequireSpecial:   false,
			},
			wantErr: false,
		},
		{
			name:     "fails strict policy",
			password: "SimplePass123",
			policy: &PasswordPolicy{
				MinLength:        15,
				MaxLength:        128,
				RequireUppercase: true,
				RequireLowercase: true,
				RequireDigit:     true,
				RequireSpecial:   true,
			},
			wantErr: true, // too short for strict policy
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := ValidateWithPolicy(tt.password, tt.policy)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateWithPolicy() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestCheckPasswordHistory(t *testing.T) {
	hash1, _ := HashPassword("OldPassword1!")
	hash2, _ := HashPassword("OldPassword2!")
	hash3, _ := HashPassword("OldPassword3!")
	previousHashes := []string{hash1, hash2, hash3}

	tests := []struct {
		name              string
		newPasswordHash   string
		previousHashes    []string
		preventReuseCount int
		wantErr           bool
	}{
		{
			name:              "new password not in history",
			newPasswordHash:   "new-hash",
			previousHashes:    previousHashes,
			preventReuseCount: 3,
			wantErr:           false,
		},
		{
			name:              "reusing recent password",
			newPasswordHash:   hash1,
			previousHashes:    previousHashes,
			preventReuseCount: 3,
			wantErr:           true,
		},
		{
			name:              "reusing old password beyond limit",
			newPasswordHash:   hash1,
			previousHashes:    previousHashes,
			preventReuseCount: 2,
			wantErr:           true, // hash1 is at index 0, still within last 2
		},
		{
			name:              "prevent reuse disabled",
			newPasswordHash:   hash1,
			previousHashes:    previousHashes,
			preventReuseCount: 0,
			wantErr:           false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			err := CheckPasswordHistory(tt.newPasswordHash, tt.previousHashes, tt.preventReuseCount)
			if (err != nil) != tt.wantErr {
				t.Errorf("CheckPasswordHistory() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestPasswordStrength_String(t *testing.T) {
	tests := []struct {
		strength PasswordStrength
		want     string
	}{
		{PasswordWeak, "weak"},
		{PasswordFair, "fair"},
		{PasswordGood, "good"},
		{PasswordStrong, "strong"},
		{PasswordVeryStrong, "very_strong"},
		{PasswordStrength(999), "unknown"},
	}

	for _, tt := range tests {
		t.Run(tt.want, func(t *testing.T) {
			if got := tt.strength.String(); got != tt.want {
				t.Errorf("PasswordStrength.String() = %v, want %v", got, tt.want)
			}
		})
	}
}

func BenchmarkHashPassword(b *testing.B) {
	password := "BenchmarkPassword123!"
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = HashPassword(password)
	}
}

func BenchmarkVerifyPassword(b *testing.B) {
	password := "BenchmarkPassword123!"
	hash, _ := HashPassword(password)
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = VerifyPassword(password, hash)
	}
}

func BenchmarkValidatePassword(b *testing.B) {
	validator := NewPasswordValidator()
	password := "BenchmarkPassword123!"
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = validator.ValidatePassword(password)
	}
}

func BenchmarkCheckPasswordStrength(b *testing.B) {
	validator := NewPasswordValidator()
	password := "BenchmarkPassword123!"
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = validator.CheckPasswordStrength(password)
	}
}

func BenchmarkGenerateRandomPassword(b *testing.B) {
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = GenerateRandomPassword(16)
	}
}
