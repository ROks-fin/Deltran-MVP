package auth

import (
	"crypto/rand"
	"errors"
	"fmt"
	"regexp"
	"unicode"

	"golang.org/x/crypto/bcrypt"
)

const (
	// BcryptCost is the cost factor for bcrypt hashing
	BcryptCost = 12

	// Password complexity requirements
	MinPasswordLength = 8
	MaxPasswordLength = 128
)

var (
	ErrPasswordTooShort  = fmt.Errorf("password must be at least %d characters", MinPasswordLength)
	ErrPasswordTooLong   = fmt.Errorf("password must be at most %d characters", MaxPasswordLength)
	ErrPasswordTooWeak   = errors.New("password does not meet complexity requirements")
	ErrPasswordIncorrect = errors.New("incorrect password")
)

// PasswordStrength represents password strength level
type PasswordStrength int

const (
	PasswordWeak PasswordStrength = iota
	PasswordFair
	PasswordGood
	PasswordStrong
	PasswordVeryStrong
)

func (ps PasswordStrength) String() string {
	switch ps {
	case PasswordWeak:
		return "weak"
	case PasswordFair:
		return "fair"
	case PasswordGood:
		return "good"
	case PasswordStrong:
		return "strong"
	case PasswordVeryStrong:
		return "very_strong"
	default:
		return "unknown"
	}
}

// PasswordValidator validates and hashes passwords
type PasswordValidator struct {
	minLength         int
	maxLength         int
	requireUppercase  bool
	requireLowercase  bool
	requireDigit      bool
	requireSpecial    bool
	commonPasswords   map[string]bool
}

// NewPasswordValidator creates a new password validator with default settings
func NewPasswordValidator() *PasswordValidator {
	return &PasswordValidator{
		minLength:        MinPasswordLength,
		maxLength:        MaxPasswordLength,
		requireUppercase: true,
		requireLowercase: true,
		requireDigit:     true,
		requireSpecial:   true,
		commonPasswords:  getCommonPasswords(),
	}
}

// HashPassword hashes a password using bcrypt
func HashPassword(password string) (string, error) {
	hash, err := bcrypt.GenerateFromPassword([]byte(password), BcryptCost)
	if err != nil {
		return "", fmt.Errorf("failed to hash password: %w", err)
	}
	return string(hash), nil
}

// VerifyPassword verifies a password against a hash
func VerifyPassword(password, hash string) error {
	err := bcrypt.CompareHashAndPassword([]byte(hash), []byte(password))
	if err != nil {
		if errors.Is(err, bcrypt.ErrMismatchedHashAndPassword) {
			return ErrPasswordIncorrect
		}
		return fmt.Errorf("failed to verify password: %w", err)
	}
	return nil
}

// ValidatePassword validates password complexity
func (pv *PasswordValidator) ValidatePassword(password string) error {
	// Check length
	if len(password) < pv.minLength {
		return ErrPasswordTooShort
	}
	if len(password) > pv.maxLength {
		return ErrPasswordTooLong
	}

	// Check complexity requirements
	var (
		hasUpper   bool
		hasLower   bool
		hasDigit   bool
		hasSpecial bool
	)

	for _, char := range password {
		switch {
		case unicode.IsUpper(char):
			hasUpper = true
		case unicode.IsLower(char):
			hasLower = true
		case unicode.IsDigit(char):
			hasDigit = true
		case unicode.IsPunct(char) || unicode.IsSymbol(char):
			hasSpecial = true
		}
	}

	if pv.requireUppercase && !hasUpper {
		return fmt.Errorf("%w: must contain at least one uppercase letter", ErrPasswordTooWeak)
	}
	if pv.requireLowercase && !hasLower {
		return fmt.Errorf("%w: must contain at least one lowercase letter", ErrPasswordTooWeak)
	}
	if pv.requireDigit && !hasDigit {
		return fmt.Errorf("%w: must contain at least one digit", ErrPasswordTooWeak)
	}
	if pv.requireSpecial && !hasSpecial {
		return fmt.Errorf("%w: must contain at least one special character", ErrPasswordTooWeak)
	}

	// Check against common passwords
	if pv.commonPasswords[password] {
		return fmt.Errorf("%w: password is too common", ErrPasswordTooWeak)
	}

	return nil
}

// CheckPasswordStrength evaluates password strength
func (pv *PasswordValidator) CheckPasswordStrength(password string) PasswordStrength {
	score := 0

	// Length score
	length := len(password)
	if length >= 8 {
		score++
	}
	if length >= 12 {
		score++
	}
	if length >= 16 {
		score++
	}

	// Character diversity score
	var (
		hasUpper   bool
		hasLower   bool
		hasDigit   bool
		hasSpecial bool
	)

	for _, char := range password {
		switch {
		case unicode.IsUpper(char):
			hasUpper = true
		case unicode.IsLower(char):
			hasLower = true
		case unicode.IsDigit(char):
			hasDigit = true
		case unicode.IsPunct(char) || unicode.IsSymbol(char):
			hasSpecial = true
		}
	}

	if hasUpper && hasLower {
		score++
	}
	if hasDigit {
		score++
	}
	if hasSpecial {
		score++
	}

	// Pattern detection (penalize common patterns)
	if hasCommonPattern(password) {
		score--
	}

	// Common password check
	if pv.commonPasswords[password] {
		return PasswordWeak
	}

	// Determine strength
	switch {
	case score <= 2:
		return PasswordWeak
	case score == 3:
		return PasswordFair
	case score == 4:
		return PasswordGood
	case score == 5:
		return PasswordStrong
	default:
		return PasswordVeryStrong
	}
}

// hasCommonPattern checks for common password patterns
func hasCommonPattern(password string) bool {
	patterns := []string{
		`^(.)\1+$`,          // All same character
		`^(01|12|23|34|45|56|67|78|89|90)+$`, // Sequential numbers
		`^(abc|bcd|cde|def|efg|fgh|ghi|hij|ijk|jkl|klm|lmn|mno|nop|opq|pqr|qrs|rst|stu|tuv|uvw|vwx|wxy|xyz)+$`, // Sequential letters
		`^(qwerty|asdfgh|zxcvbn)+$`, // Keyboard patterns
	}

	for _, pattern := range patterns {
		matched, _ := regexp.MatchString(pattern, password)
		if matched {
			return true
		}
	}

	return false
}

// getCommonPasswords returns a set of common passwords
func getCommonPasswords() map[string]bool {
	// Top 100 most common passwords (subset for demonstration)
	passwords := []string{
		"password", "123456", "12345678", "12345", "qwerty",
		"abc123", "monkey", "1234567", "letmein", "trustno1",
		"dragon", "baseball", "iloveyou", "master", "sunshine",
		"ashley", "bailey", "passw0rd", "shadow", "123123",
		"654321", "superman", "qazwsx", "michael", "football",
		"password1", "Password1", "welcome", "admin", "root",
		"test", "guest", "changeme", "test123", "admin123",
	}

	common := make(map[string]bool, len(passwords))
	for _, pwd := range passwords {
		common[pwd] = true
	}

	return common
}

// PasswordPolicy represents password policy configuration
type PasswordPolicy struct {
	MinLength              int  `json:"min_length"`
	MaxLength              int  `json:"max_length"`
	RequireUppercase       bool `json:"require_uppercase"`
	RequireLowercase       bool `json:"require_lowercase"`
	RequireDigit           bool `json:"require_digit"`
	RequireSpecial         bool `json:"require_special"`
	MaxPasswordAge         int  `json:"max_password_age"`          // days
	PasswordHistoryCount   int  `json:"password_history_count"`    // remember last N passwords
	PreventReuseCount      int  `json:"prevent_reuse_count"`       // can't reuse last N passwords
	AccountLockoutThreshold int `json:"account_lockout_threshold"` // failed attempts before lockout
	AccountLockoutDuration int  `json:"account_lockout_duration"`  // minutes
}

// DefaultPasswordPolicy returns default password policy
func DefaultPasswordPolicy() *PasswordPolicy {
	return &PasswordPolicy{
		MinLength:              MinPasswordLength,
		MaxLength:              MaxPasswordLength,
		RequireUppercase:       true,
		RequireLowercase:       true,
		RequireDigit:           true,
		RequireSpecial:         true,
		MaxPasswordAge:         90,  // 90 days
		PasswordHistoryCount:   5,   // remember last 5 passwords
		PreventReuseCount:      3,   // can't reuse last 3 passwords
		AccountLockoutThreshold: 5,  // lock after 5 failed attempts
		AccountLockoutDuration: 30,  // lock for 30 minutes
	}
}

// ValidateWithPolicy validates password against a policy
func ValidateWithPolicy(password string, policy *PasswordPolicy) error {
	pv := &PasswordValidator{
		minLength:        policy.MinLength,
		maxLength:        policy.MaxLength,
		requireUppercase: policy.RequireUppercase,
		requireLowercase: policy.RequireLowercase,
		requireDigit:     policy.RequireDigit,
		requireSpecial:   policy.RequireSpecial,
		commonPasswords:  getCommonPasswords(),
	}

	return pv.ValidatePassword(password)
}

// CheckPasswordHistory checks if password was used recently
func CheckPasswordHistory(newPasswordHash string, previousHashes []string, preventReuseCount int) error {
	if preventReuseCount <= 0 {
		return nil
	}

	// Check against last N passwords
	checkCount := preventReuseCount
	if checkCount > len(previousHashes) {
		checkCount = len(previousHashes)
	}

	for i := 0; i < checkCount; i++ {
		// In real implementation, we'd need to hash newPassword and compare
		// This is a simplified version
		if newPasswordHash == previousHashes[i] {
			return fmt.Errorf("cannot reuse one of your last %d passwords", preventReuseCount)
		}
	}

	return nil
}

// GenerateRandomPassword generates a random password meeting complexity requirements
func GenerateRandomPassword(length int) (string, error) {
	if length < MinPasswordLength {
		length = MinPasswordLength
	}
	if length > MaxPasswordLength {
		length = MaxPasswordLength
	}

	const (
		lowercase = "abcdefghijklmnopqrstuvwxyz"
		uppercase = "ABCDEFGHIJKLMNOPQRSTUVWXYZ"
		digits    = "0123456789"
		special   = "!@#$%^&*()-_=+[]{}|;:,.<>?"
	)

	allChars := lowercase + uppercase + digits + special

	password := make([]byte, length)
	randomBytes := make([]byte, length)

	_, err := rand.Read(randomBytes)
	if err != nil {
		return "", fmt.Errorf("failed to generate random bytes: %w", err)
	}

	// Ensure at least one of each required character type
	password[0] = lowercase[int(randomBytes[0])%len(lowercase)]
	password[1] = uppercase[int(randomBytes[1])%len(uppercase)]
	password[2] = digits[int(randomBytes[2])%len(digits)]
	password[3] = special[int(randomBytes[3])%len(special)]

	// Fill rest with random characters
	for i := 4; i < length; i++ {
		password[i] = allChars[int(randomBytes[i])%len(allChars)]
	}

	// Shuffle password
	for i := length - 1; i > 0; i-- {
		j := int(randomBytes[i]) % (i + 1)
		password[i], password[j] = password[j], password[i]
	}

	return string(password), nil
}
