package auth

import (
	"crypto/hmac"
	"crypto/rand"
	"crypto/sha1"
	"encoding/base32"
	"encoding/binary"
	"errors"
	"fmt"
	"strings"
	"time"
)

const (
	// TOTPPeriod is the time period for TOTP codes (30 seconds)
	TOTPPeriod = 30
	// TOTPDigits is the number of digits in TOTP code
	TOTPDigits = 6
	// TOTPIssuer is the issuer name shown in authenticator apps
	TOTPIssuer = "DelTran"
	// BackupCodesCount is the number of backup codes to generate
	BackupCodesCount = 10
	// BackupCodeLength is the length of each backup code
	BackupCodeLength = 8
)

var (
	ErrInvalidTOTPCode   = errors.New("invalid TOTP code")
	ErrTOTPNotConfigured = errors.New("TOTP not configured for user")
)

// TOTPManager manages TOTP 2FA operations
type TOTPManager struct {
	issuer string
	period int
	digits int
}

// NewTOTPManager creates a new TOTP manager
func NewTOTPManager() *TOTPManager {
	return &TOTPManager{
		issuer: TOTPIssuer,
		period: TOTPPeriod,
		digits: TOTPDigits,
	}
}

// GenerateSecret generates a new TOTP secret
func (t *TOTPManager) GenerateSecret() (string, error) {
	secret := make([]byte, 20) // 160 bits
	_, err := rand.Read(secret)
	if err != nil {
		return "", fmt.Errorf("failed to generate secret: %w", err)
	}

	// Encode to base32
	encoded := base32.StdEncoding.EncodeToString(secret)
	// Remove padding
	encoded = strings.TrimRight(encoded, "=")

	return encoded, nil
}

// GenerateQRCodeURL generates URL for QR code
func (t *TOTPManager) GenerateQRCodeURL(secret, accountName string) string {
	// Format: otpauth://totp/ISSUER:ACCOUNT?secret=SECRET&issuer=ISSUER
	return fmt.Sprintf(
		"otpauth://totp/%s:%s?secret=%s&issuer=%s&algorithm=SHA1&digits=%d&period=%d",
		t.issuer,
		accountName,
		secret,
		t.issuer,
		t.digits,
		t.period,
	)
}

// GenerateCode generates TOTP code for given secret at specific time
func (t *TOTPManager) GenerateCode(secret string, timestamp time.Time) (string, error) {
	// Decode base32 secret
	secret = strings.ToUpper(strings.ReplaceAll(secret, " ", ""))
	// Add padding if needed
	padding := (8 - len(secret)%8) % 8
	secret += strings.Repeat("=", padding)

	key, err := base32.StdEncoding.DecodeString(secret)
	if err != nil {
		return "", fmt.Errorf("invalid secret: %w", err)
	}

	// Calculate counter (time-based)
	counter := uint64(timestamp.Unix()) / uint64(t.period)

	// Generate HMAC-SHA1
	h := hmac.New(sha1.New, key)
	binary.Write(h, binary.BigEndian, counter)
	hash := h.Sum(nil)

	// Dynamic truncation
	offset := hash[len(hash)-1] & 0x0F
	truncated := binary.BigEndian.Uint32(hash[offset:offset+4]) & 0x7FFFFFFF

	// Generate code
	code := truncated % uint32(pow10(t.digits))

	// Format with leading zeros
	format := fmt.Sprintf("%%0%dd", t.digits)
	return fmt.Sprintf(format, code), nil
}

// ValidateCode validates TOTP code with time window
func (t *TOTPManager) ValidateCode(secret, code string) (bool, error) {
	if secret == "" {
		return false, ErrTOTPNotConfigured
	}

	now := time.Now().UTC()

	// Check current time window
	currentCode, err := t.GenerateCode(secret, now)
	if err != nil {
		return false, err
	}
	if currentCode == code {
		return true, nil
	}

	// Check previous time window (handle clock skew)
	prevTime := now.Add(-time.Duration(t.period) * time.Second)
	prevCode, err := t.GenerateCode(secret, prevTime)
	if err != nil {
		return false, err
	}
	if prevCode == code {
		return true, nil
	}

	// Check next time window (handle clock skew)
	nextTime := now.Add(time.Duration(t.period) * time.Second)
	nextCode, err := t.GenerateCode(secret, nextTime)
	if err != nil {
		return false, err
	}
	if nextCode == code {
		return true, nil
	}

	return false, ErrInvalidTOTPCode
}

// ValidateCodeStrict validates TOTP code without time window tolerance
func (t *TOTPManager) ValidateCodeStrict(secret, code string) (bool, error) {
	if secret == "" {
		return false, ErrTOTPNotConfigured
	}

	currentCode, err := t.GenerateCode(secret, time.Now().UTC())
	if err != nil {
		return false, err
	}

	if currentCode == code {
		return true, nil
	}

	return false, ErrInvalidTOTPCode
}

// GenerateBackupCodes generates backup codes for recovery
func (t *TOTPManager) GenerateBackupCodes() ([]string, error) {
	codes := make([]string, BackupCodesCount)

	for i := 0; i < BackupCodesCount; i++ {
		code, err := generateBackupCode()
		if err != nil {
			return nil, fmt.Errorf("failed to generate backup code: %w", err)
		}
		codes[i] = code
	}

	return codes, nil
}

// FormatBackupCode formats backup code with dashes (e.g., ABCD-EFGH)
func FormatBackupCode(code string) string {
	if len(code) != BackupCodeLength {
		return code
	}
	return fmt.Sprintf("%s-%s", code[:4], code[4:])
}

// generateBackupCode generates a single backup code
func generateBackupCode() (string, error) {
	const charset = "ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789"
	b := make([]byte, BackupCodeLength)
	_, err := rand.Read(b)
	if err != nil {
		return "", err
	}

	for i := 0; i < BackupCodeLength; i++ {
		b[i] = charset[int(b[i])%len(charset)]
	}

	return string(b), nil
}

// pow10 calculates 10^n
func pow10(n int) int {
	result := 1
	for i := 0; i < n; i++ {
		result *= 10
	}
	return result
}

// TOTPSetup represents TOTP setup information
type TOTPSetup struct {
	Secret      string   `json:"secret"`
	QRCodeURL   string   `json:"qr_code_url"`
	BackupCodes []string `json:"backup_codes"`
}

// SetupTOTP generates TOTP setup information for a user
func (t *TOTPManager) SetupTOTP(email string) (*TOTPSetup, error) {
	// Generate secret
	secret, err := t.GenerateSecret()
	if err != nil {
		return nil, err
	}

	// Generate QR code URL
	qrURL := t.GenerateQRCodeURL(secret, email)

	// Generate backup codes
	backupCodes, err := t.GenerateBackupCodes()
	if err != nil {
		return nil, err
	}

	// Format backup codes
	formattedCodes := make([]string, len(backupCodes))
	for i, code := range backupCodes {
		formattedCodes[i] = FormatBackupCode(code)
	}

	return &TOTPSetup{
		Secret:      secret,
		QRCodeURL:   qrURL,
		BackupCodes: formattedCodes,
	}, nil
}

// VerifyTOTPSetup verifies TOTP setup by validating the first code
func (t *TOTPManager) VerifyTOTPSetup(secret, code string) error {
	valid, err := t.ValidateCode(secret, code)
	if err != nil {
		return err
	}

	if !valid {
		return ErrInvalidTOTPCode
	}

	return nil
}
