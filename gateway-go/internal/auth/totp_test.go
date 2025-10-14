package auth

import (
	"strings"
	"testing"
	"time"
)

func TestNewTOTPManager(t *testing.T) {
	manager := NewTOTPManager()

	if manager == nil {
		t.Fatal("NewTOTPManager() returned nil")
	}
	if manager.issuer != "DelTran" {
		t.Errorf("NewTOTPManager() issuer = %v, want %v", manager.issuer, "DelTran")
	}
	if manager.period != 30 {
		t.Errorf("NewTOTPManager() period = %v, want %v", manager.period, 30)
	}
	if manager.digits != 6 {
		t.Errorf("NewTOTPManager() digits = %v, want %v", manager.digits, 6)
	}
}

func TestTOTPManager_GenerateSecret(t *testing.T) {
	manager := NewTOTPManager()

	secret, err := manager.GenerateSecret()
	if err != nil {
		t.Fatalf("GenerateSecret() error = %v", err)
	}

	if secret == "" {
		t.Error("GenerateSecret() returned empty secret")
	}

	// Secret should be base32 encoded
	if len(secret) < 16 {
		t.Error("GenerateSecret() returned secret too short")
	}

	// Generate multiple secrets and ensure they're unique
	secrets := make(map[string]bool)
	for i := 0; i < 10; i++ {
		s, _ := manager.GenerateSecret()
		if secrets[s] {
			t.Error("GenerateSecret() generated duplicate secret")
		}
		secrets[s] = true
	}
}

func TestTOTPManager_GenerateCode(t *testing.T) {
	manager := NewTOTPManager()
	secret, _ := manager.GenerateSecret()
	timestamp := time.Now()

	code, err := manager.GenerateCode(secret, timestamp)
	if err != nil {
		t.Fatalf("GenerateCode() error = %v", err)
	}

	if len(code) != 6 {
		t.Errorf("GenerateCode() code length = %v, want 6", len(code))
	}

	// Code should be numeric
	for _, char := range code {
		if char < '0' || char > '9' {
			t.Errorf("GenerateCode() returned non-numeric code: %v", code)
			break
		}
	}

	// Same timestamp should generate same code
	code2, _ := manager.GenerateCode(secret, timestamp)
	if code != code2 {
		t.Error("GenerateCode() generated different codes for same timestamp")
	}

	// Different timestamp should generate different code (usually)
	code3, _ := manager.GenerateCode(secret, timestamp.Add(time.Minute))
	if code == code3 {
		// This might occasionally happen due to time windows, but unlikely
		t.Log("Warning: GenerateCode() generated same code for different timestamps")
	}
}

func TestTOTPManager_ValidateCode(t *testing.T) {
	manager := NewTOTPManager()
	secret, _ := manager.GenerateSecret()

	// Generate current code
	now := time.Now()
	validCode, _ := manager.GenerateCode(secret, now)

	tests := []struct {
		name    string
		secret  string
		code    string
		wantErr bool
	}{
		{
			name:    "valid current code",
			secret:  secret,
			code:    validCode,
			wantErr: false,
		},
		{
			name:    "invalid code",
			secret:  secret,
			code:    "000000",
			wantErr: true,
		},
		{
			name:    "wrong length code",
			secret:  secret,
			code:    "123",
			wantErr: true,
		},
		{
			name:    "empty code",
			secret:  secret,
			code:    "",
			wantErr: true,
		},
		{
			name:    "non-numeric code",
			secret:  secret,
			code:    "abc123",
			wantErr: true,
		},
		{
			name:    "invalid secret",
			secret:  "invalid!!!",
			code:    validCode,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			valid, err := manager.ValidateCode(tt.secret, tt.code)
			if (err != nil) != tt.wantErr {
				t.Errorf("ValidateCode() error = %v, wantErr %v", err, tt.wantErr)
				return
			}
			if !tt.wantErr && !valid {
				t.Error("ValidateCode() returned false for valid code")
			}
			if tt.wantErr && valid {
				t.Error("ValidateCode() returned true for invalid code")
			}
		})
	}
}

func TestTOTPManager_ValidateCode_TimeWindow(t *testing.T) {
	manager := NewTOTPManager()
	secret, _ := manager.GenerateSecret()

	// Test codes from past, present, and future time windows
	now := time.Now()
	pastCode, _ := manager.GenerateCode(secret, now.Add(-30*time.Second))   // 1 window back
	currentCode, _ := manager.GenerateCode(secret, now)
	futureCode, _ := manager.GenerateCode(secret, now.Add(30*time.Second))  // 1 window ahead

	tests := []struct {
		name string
		code string
		want bool
	}{
		{
			name: "current code valid",
			code: currentCode,
			want: true,
		},
		{
			name: "past code valid (within skew)",
			code: pastCode,
			want: true,
		},
		{
			name: "future code valid (within skew)",
			code: futureCode,
			want: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			valid, err := manager.ValidateCode(secret, tt.code)
			if err != nil {
				t.Fatalf("ValidateCode() error = %v", err)
			}
			if valid != tt.want {
				t.Errorf("ValidateCode() = %v, want %v", valid, tt.want)
			}
		})
	}

	// Code from 2 windows ago should be invalid
	veryOldCode, _ := manager.GenerateCode(secret, now.Add(-60*time.Second))
	valid, _ := manager.ValidateCode(secret, veryOldCode)
	if valid {
		t.Error("ValidateCode() accepted code from 2 windows ago (should be rejected)")
	}
}

func TestTOTPManager_GenerateQRCodeURL(t *testing.T) {
	manager := NewTOTPManager()
	secret, _ := manager.GenerateSecret()

	tests := []struct {
		name        string
		accountName string
		secret      string
		checkFunc   func(string) error
	}{
		{
			name:        "valid QR code URL",
			accountName: "user@example.com",
			secret:      secret,
			checkFunc: func(uri string) error {
				if !strings.HasPrefix(uri, "otpauth://totp/") {
					t.Error("URI should start with otpauth://totp/")
				}
				if !strings.Contains(uri, "user@example.com") {
					t.Error("URI should contain account name")
				}
				if !strings.Contains(uri, "issuer=DelTran") {
					t.Error("URI should contain issuer")
				}
				if !strings.Contains(uri, "secret=") {
					t.Error("URI should contain secret")
				}
				if !strings.Contains(uri, "digits=6") {
					t.Error("URI should specify 6 digits")
				}
				if !strings.Contains(uri, "period=30") {
					t.Error("URI should specify 30 second period")
				}
				return nil
			},
		},
		{
			name:        "empty account name",
			accountName: "",
			secret:      secret,
		},
		{
			name:        "special characters in account name",
			accountName: "user+test@example.com",
			secret:      secret,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			url := manager.GenerateQRCodeURL(tt.secret, tt.accountName)
			if tt.checkFunc != nil {
				if err := tt.checkFunc(url); err != nil {
					t.Error(err)
				}
			}
		})
	}
}

func TestTOTPManager_GenerateBackupCodes(t *testing.T) {
	manager := NewTOTPManager()

	codes, err := manager.GenerateBackupCodes()
	if err != nil {
		t.Fatalf("GenerateBackupCodes() error = %v", err)
	}

	if len(codes) != BackupCodesCount {
		t.Errorf("GenerateBackupCodes() returned %v codes, want %v", len(codes), BackupCodesCount)
	}

	// Check all codes are unique
	codeMap := make(map[string]bool)
	for _, code := range codes {
		if len(code) != BackupCodeLength {
			t.Errorf("Backup code length = %v, want %v", len(code), BackupCodeLength)
		}

		// Check uniqueness
		if codeMap[code] {
			t.Errorf("Duplicate backup code: %v", code)
		}
		codeMap[code] = true

		// Check characters are alphanumeric
		for _, char := range code {
			if !((char >= 'A' && char <= 'Z') || (char >= '0' && char <= '9')) {
				t.Errorf("Backup code contains invalid character: %v", code)
				break
			}
		}
	}
}

func TestTOTPManager_CodeGeneration_Consistency(t *testing.T) {
	// Test that two managers with same config generate same codes
	manager1 := NewTOTPManager()
	manager2 := NewTOTPManager()

	secret, _ := manager1.GenerateSecret()
	timestamp := time.Now()

	code1, _ := manager1.GenerateCode(secret, timestamp)
	code2, _ := manager2.GenerateCode(secret, timestamp)

	if code1 != code2 {
		t.Error("Two TOTPManagers with same config generated different codes")
	}

	// Both should validate the same code
	valid1, _ := manager1.ValidateCode(secret, code1)
	valid2, _ := manager2.ValidateCode(secret, code1)

	if !valid1 || !valid2 {
		t.Error("Code should be valid for both managers")
	}
}

func BenchmarkGenerateSecret(b *testing.B) {
	manager := NewTOTPManager()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = manager.GenerateSecret()
	}
}

func BenchmarkGenerateCode(b *testing.B) {
	manager := NewTOTPManager()
	secret, _ := manager.GenerateSecret()
	timestamp := time.Now()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = manager.GenerateCode(secret, timestamp)
	}
}

func BenchmarkValidateCode(b *testing.B) {
	manager := NewTOTPManager()
	secret, _ := manager.GenerateSecret()
	code, _ := manager.GenerateCode(secret, time.Now())
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = manager.ValidateCode(secret, code)
	}
}

func BenchmarkGenerateBackupCodes(b *testing.B) {
	manager := NewTOTPManager()
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = manager.GenerateBackupCodes()
	}
}
