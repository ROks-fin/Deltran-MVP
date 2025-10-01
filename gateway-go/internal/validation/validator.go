// Payment validation logic
package validation

import (
	"regexp"
	"strings"

	"github.com/deltran/gateway/internal/config"
	"github.com/deltran/gateway/internal/types"
	"github.com/shopspring/decimal"
	"go.uber.org/zap"
)

var (
	// BIC format: 4 letters (bank) + 2 letters (country) + 2 alphanumeric (location) + optional 3 alphanumeric (branch)
	bicRegex = regexp.MustCompile(`^[A-Z]{4}[A-Z]{2}[A-Z0-9]{2}([A-Z0-9]{3})?$`)

	// IBAN format (basic validation)
	ibanRegex = regexp.MustCompile(`^[A-Z]{2}[0-9]{2}[A-Z0-9]+$`)

	// Sanctions lists (simplified - in production, use proper OFAC/UN/EU lists)
	sanctionedEntities = []string{
		"SANCTIONED_BANK",
		"BLOCKED_ENTITY",
		// Add real entities from OFAC/UN/EU lists
	}
)

// Validator handles payment validation
type Validator struct {
	config *config.Config
	logger *zap.Logger

	minAmount decimal.Decimal
	maxAmount decimal.Decimal
}

// New creates a new validator
func New(cfg *config.Config, logger *zap.Logger) *Validator {
	minAmount, _ := decimal.NewFromString(cfg.Limits.MinPaymentAmount)
	maxAmount, _ := decimal.NewFromString(cfg.Limits.MaxPaymentAmount)

	return &Validator{
		config:    cfg,
		logger:    logger,
		minAmount: minAmount,
		maxAmount: maxAmount,
	}
}

// ValidatePayment validates a payment
func (v *Validator) ValidatePayment(payment *types.Payment) *types.ValidationResult {
	result := &types.ValidationResult{
		Valid:    true,
		Errors:   []string{},
		Warnings: []string{},
	}

	// 1. Validate amount
	if payment.Amount.LessThanOrEqual(decimal.Zero) {
		result.Valid = false
		result.Errors = append(result.Errors, "Amount must be positive")
	}

	if payment.Amount.LessThan(v.minAmount) {
		result.Valid = false
		result.Errors = append(result.Errors, "Amount below minimum")
	}

	if payment.Amount.GreaterThan(v.maxAmount) {
		result.Valid = false
		result.Errors = append(result.Errors, "Amount exceeds maximum")
	}

	// 2. Validate currency
	if !isValidCurrency(payment.Currency) {
		result.Valid = false
		result.Errors = append(result.Errors, "Invalid currency code")
	}

	// 3. Validate BICs
	if !bicRegex.MatchString(payment.DebtorBank) {
		result.Valid = false
		result.Errors = append(result.Errors, "Invalid debtor BIC")
	}

	if !bicRegex.MatchString(payment.CreditorBank) {
		result.Valid = false
		result.Errors = append(result.Errors, "Invalid creditor BIC")
	}

	// 4. Validate accounts (basic check)
	if payment.DebtorAccount == "" {
		result.Valid = false
		result.Errors = append(result.Errors, "Debtor account required")
	}

	if payment.CreditorAccount == "" {
		result.Valid = false
		result.Errors = append(result.Errors, "Creditor account required")
	}

	// 5. Validate names
	if payment.DebtorName == "" {
		result.Valid = false
		result.Errors = append(result.Errors, "Debtor name required")
	}

	if payment.CreditorName == "" {
		result.Valid = false
		result.Errors = append(result.Errors, "Creditor name required")
	}

	// 6. Validate reference
	if payment.Reference == "" {
		result.Warnings = append(result.Warnings, "Payment reference missing")
	}

	// 7. Check for same bank (warning)
	if payment.DebtorBank == payment.CreditorBank {
		result.Warnings = append(result.Warnings, "Same bank transfer - consider direct processing")
	}

	return result
}

// CheckSanctions checks payment against sanctions lists
func (v *Validator) CheckSanctions(payment *types.Payment) *types.SanctionsCheck {
	check := &types.SanctionsCheck{
		Cleared: true,
		Hits:    []string{},
		Lists:   []string{},
	}

	// Check debtor
	if isSanctioned(payment.DebtorName) || isSanctioned(payment.DebtorBank) {
		check.Cleared = false
		check.Hits = append(check.Hits, "Debtor: "+payment.DebtorName)
		check.Lists = append(check.Lists, "OFAC")
	}

	// Check creditor
	if isSanctioned(payment.CreditorName) || isSanctioned(payment.CreditorBank) {
		check.Cleared = false
		check.Hits = append(check.Hits, "Creditor: "+payment.CreditorName)
		check.Lists = append(check.Lists, "OFAC")
	}

	if !check.Cleared {
		v.logger.Warn("Sanctions hit detected",
			zap.String("payment_id", payment.PaymentID.String()),
			zap.Strings("hits", check.Hits),
		)
	}

	return check
}

// AssessRisk performs risk assessment
func (v *Validator) AssessRisk(payment *types.Payment) *types.RiskAssessment {
	assessment := &types.RiskAssessment{
		Approved:    true,
		RiskScore:   0.0,
		RiskLevel:   "LOW",
		Reasons:     []string{},
		Mitigations: []string{},
	}

	// 1. Amount-based risk
	if payment.Amount.GreaterThan(decimal.NewFromInt(100000)) {
		assessment.RiskScore += 0.3
		assessment.Reasons = append(assessment.Reasons, "High value transaction")
	}

	// 2. Country-based risk
	debtorCountry := payment.DebtorBank[:2]
	creditorCountry := payment.CreditorBank[:2]

	if isHighRiskCountry(debtorCountry) || isHighRiskCountry(creditorCountry) {
		assessment.RiskScore += 0.4
		assessment.Reasons = append(assessment.Reasons, "High-risk country")
	}

	// 3. Cross-border risk
	if debtorCountry != creditorCountry {
		assessment.RiskScore += 0.1
		assessment.Reasons = append(assessment.Reasons, "Cross-border transaction")
	}

	// 4. Currency risk
	if !isCommonCurrency(payment.Currency) {
		assessment.RiskScore += 0.2
		assessment.Reasons = append(assessment.Reasons, "Uncommon currency")
	}

	// Determine risk level
	if assessment.RiskScore >= 0.8 {
		assessment.RiskLevel = "CRITICAL"
		assessment.Approved = false
	} else if assessment.RiskScore >= 0.6 {
		assessment.RiskLevel = "HIGH"
		assessment.Mitigations = append(assessment.Mitigations, "Enhanced due diligence required")
	} else if assessment.RiskScore >= 0.3 {
		assessment.RiskLevel = "MEDIUM"
		assessment.Mitigations = append(assessment.Mitigations, "Additional verification recommended")
	}

	if !assessment.Approved {
		v.logger.Warn("Risk assessment rejected",
			zap.String("payment_id", payment.PaymentID.String()),
			zap.Float64("risk_score", assessment.RiskScore),
			zap.String("risk_level", assessment.RiskLevel),
		)
	}

	return assessment
}

// Helper functions

func isValidCurrency(currency string) bool {
	validCurrencies := []string{"USD", "EUR", "GBP", "AED", "INR", "CHF", "JPY", "CNY"}
	for _, c := range validCurrencies {
		if currency == c {
			return true
		}
	}
	return false
}

func isSanctioned(name string) bool {
	nameUpper := strings.ToUpper(name)
	for _, entity := range sanctionedEntities {
		if strings.Contains(nameUpper, entity) {
			return true
		}
	}
	return false
}

func isHighRiskCountry(countryCode string) bool {
	// Simplified list - in production, use FATF high-risk jurisdictions
	highRiskCountries := []string{"KP", "IR", "SY"}
	for _, c := range highRiskCountries {
		if countryCode == c {
			return true
		}
	}
	return false
}

func isCommonCurrency(currency string) bool {
	commonCurrencies := []string{"USD", "EUR", "GBP"}
	for _, c := range commonCurrencies {
		if currency == c {
			return true
		}
	}
	return false
}