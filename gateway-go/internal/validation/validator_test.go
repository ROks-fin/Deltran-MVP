package validation

import (
	"testing"

	"github.com/deltran/gateway/internal/config"
	"github.com/deltran/gateway/internal/types"
	"github.com/google/uuid"
	"github.com/shopspring/decimal"
	"go.uber.org/zap"
)

func newTestValidator() *Validator {
	logger, _ := zap.NewDevelopment()
	cfg := &config.Config{
		Limits: config.LimitsConfig{
			MinPaymentAmount: "0.01",
			MaxPaymentAmount: "1000000.00",
		},
	}
	return New(cfg, logger)
}

func newValidPayment() *types.Payment {
	return &types.Payment{
		PaymentID:       uuid.New(),
		Amount:          decimal.NewFromFloat(1000.00),
		Currency:        "USD",
		DebtorBank:      "BANKGB2LXXX",
		CreditorBank:    "BANKUS33XXX",
		DebtorAccount:   "GB29NWBK60161331926819",
		CreditorAccount: "US1234567890",
		DebtorName:      "John Doe",
		CreditorName:    "Jane Smith",
		Reference:       "INV-2024-001",
	}
}

func TestValidatePayment_Valid(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()

	result := validator.ValidatePayment(payment)

	if !result.Valid {
		t.Errorf("Valid payment should pass validation. Errors: %v", result.Errors)
	}

	if len(result.Errors) > 0 {
		t.Errorf("Expected no errors, got %d: %v", len(result.Errors), result.Errors)
	}
}

func TestValidatePayment_NegativeAmount(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(-100.00)

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment with negative amount should not be valid")
	}

	if !containsError(result.Errors, "Amount must be positive") {
		t.Error("Expected 'Amount must be positive' error")
	}
}

func TestValidatePayment_ZeroAmount(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.Zero

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment with zero amount should not be valid")
	}
}

func TestValidatePayment_AmountBelowMinimum(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(0.001) // Below 0.01 minimum

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment below minimum amount should not be valid")
	}

	if !containsError(result.Errors, "Amount below minimum") {
		t.Error("Expected 'Amount below minimum' error")
	}
}

func TestValidatePayment_AmountAboveMaximum(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(2000000.00) // Above 1M maximum

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment above maximum amount should not be valid")
	}

	if !containsError(result.Errors, "Amount exceeds maximum") {
		t.Error("Expected 'Amount exceeds maximum' error")
	}
}

func TestValidatePayment_InvalidCurrency(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Currency = "XXX"

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment with invalid currency should not be valid")
	}

	if !containsError(result.Errors, "Invalid currency code") {
		t.Error("Expected 'Invalid currency code' error")
	}
}

func TestValidatePayment_InvalidDebtorBIC(t *testing.T) {
	tests := []struct {
		name string
		bic  string
	}{
		{"too short", "BANK"},
		{"lowercase", "bankgb2lxxx"},
		{"invalid format", "1234567890"},
		{"empty", ""},
	}

	validator := newTestValidator()

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			payment := newValidPayment()
			payment.DebtorBank = tt.bic

			result := validator.ValidatePayment(payment)

			if result.Valid {
				t.Errorf("Payment with invalid debtor BIC '%s' should not be valid", tt.bic)
			}

			if !containsError(result.Errors, "Invalid debtor BIC") {
				t.Error("Expected 'Invalid debtor BIC' error")
			}
		})
	}
}

func TestValidatePayment_InvalidCreditorBIC(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.CreditorBank = "INVALID"

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment with invalid creditor BIC should not be valid")
	}

	if !containsError(result.Errors, "Invalid creditor BIC") {
		t.Error("Expected 'Invalid creditor BIC' error")
	}
}

func TestValidatePayment_MissingDebtorAccount(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.DebtorAccount = ""

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment without debtor account should not be valid")
	}

	if !containsError(result.Errors, "Debtor account required") {
		t.Error("Expected 'Debtor account required' error")
	}
}

func TestValidatePayment_MissingCreditorAccount(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.CreditorAccount = ""

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment without creditor account should not be valid")
	}

	if !containsError(result.Errors, "Creditor account required") {
		t.Error("Expected 'Creditor account required' error")
	}
}

func TestValidatePayment_MissingDebtorName(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.DebtorName = ""

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment without debtor name should not be valid")
	}

	if !containsError(result.Errors, "Debtor name required") {
		t.Error("Expected 'Debtor name required' error")
	}
}

func TestValidatePayment_MissingCreditorName(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.CreditorName = ""

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment without creditor name should not be valid")
	}

	if !containsError(result.Errors, "Creditor name required") {
		t.Error("Expected 'Creditor name required' error")
	}
}

func TestValidatePayment_MissingReference(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Reference = ""

	result := validator.ValidatePayment(payment)

	// Should still be valid but have a warning
	if !result.Valid {
		t.Error("Payment without reference should still be valid")
	}

	if !containsWarning(result.Warnings, "Payment reference missing") {
		t.Error("Expected warning about missing reference")
	}
}

func TestValidatePayment_SameBankTransfer(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.DebtorBank = "BANKGB2LXXX"
	payment.CreditorBank = "BANKGB2LXXX"

	result := validator.ValidatePayment(payment)

	// Should be valid but have a warning
	if !result.Valid {
		t.Error("Same bank transfer should be valid")
	}

	if !containsWarning(result.Warnings, "Same bank transfer") {
		t.Error("Expected warning about same bank transfer")
	}
}

func TestValidatePayment_MultipleErrors(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(-100.00)
	payment.Currency = "XXX"
	payment.DebtorBank = "INVALID"
	payment.CreditorBank = ""
	payment.DebtorAccount = ""
	payment.CreditorAccount = ""
	payment.DebtorName = ""
	payment.CreditorName = ""

	result := validator.ValidatePayment(payment)

	if result.Valid {
		t.Error("Payment with multiple errors should not be valid")
	}

	if len(result.Errors) < 5 {
		t.Errorf("Expected at least 5 errors, got %d: %v", len(result.Errors), result.Errors)
	}
}

func TestCheckSanctions_Clean(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()

	check := validator.CheckSanctions(payment)

	if !check.Cleared {
		t.Error("Clean payment should pass sanctions check")
	}

	if len(check.Hits) > 0 {
		t.Errorf("Expected no sanctions hits, got %d: %v", len(check.Hits), check.Hits)
	}
}

func TestCheckSanctions_SanctionedDebtor(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.DebtorName = "SANCTIONED_BANK LLC"

	check := validator.CheckSanctions(payment)

	if check.Cleared {
		t.Error("Payment with sanctioned debtor should not clear")
	}

	if len(check.Hits) == 0 {
		t.Error("Expected sanctions hit for debtor")
	}
}

func TestCheckSanctions_SanctionedCreditor(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.CreditorName = "BLOCKED_ENTITY Corp"

	check := validator.CheckSanctions(payment)

	if check.Cleared {
		t.Error("Payment with sanctioned creditor should not clear")
	}

	if len(check.Hits) == 0 {
		t.Error("Expected sanctions hit for creditor")
	}
}

func TestAssessRisk_LowRisk(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(1000.00) // Low amount
	payment.Currency = "USD"                        // Common currency
	payment.DebtorBank = "BANKUS33XXX"              // Same country
	payment.CreditorBank = "BANKUS44XXX"

	assessment := validator.AssessRisk(payment)

	if !assessment.Approved {
		t.Error("Low risk payment should be approved")
	}

	if assessment.RiskLevel != "LOW" {
		t.Errorf("Risk level = %s, want LOW", assessment.RiskLevel)
	}

	if assessment.RiskScore >= 0.3 {
		t.Errorf("Risk score = %f, should be < 0.3 for low risk", assessment.RiskScore)
	}
}

func TestAssessRisk_HighValueTransaction(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(200000.00) // High value

	assessment := validator.AssessRisk(payment)

	if !containsReason(assessment.Reasons, "High value transaction") {
		t.Error("Expected 'High value transaction' in risk reasons")
	}

	if assessment.RiskScore < 0.3 {
		t.Errorf("High value transaction should increase risk score, got %f", assessment.RiskScore)
	}
}

func TestAssessRisk_HighRiskCountry(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.DebtorBank = "BANKKPXXX" // North Korea (KP)

	assessment := validator.AssessRisk(payment)

	if !containsReason(assessment.Reasons, "High-risk country") {
		t.Error("Expected 'High-risk country' in risk reasons")
	}

	if assessment.RiskScore < 0.4 {
		t.Errorf("High-risk country should significantly increase risk score, got %f", assessment.RiskScore)
	}
}

func TestAssessRisk_CrossBorder(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.DebtorBank = "BANKGB2LXXX"  // UK
	payment.CreditorBank = "BANKUS33XXX" // US

	assessment := validator.AssessRisk(payment)

	if !containsReason(assessment.Reasons, "Cross-border transaction") {
		t.Error("Expected 'Cross-border transaction' in risk reasons")
	}
}

func TestAssessRisk_UncommonCurrency(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Currency = "INR" // Less common for international transfers

	assessment := validator.AssessRisk(payment)

	if !containsReason(assessment.Reasons, "Uncommon currency") {
		t.Error("Expected 'Uncommon currency' in risk reasons")
	}
}

func TestAssessRisk_CriticalLevel(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(500000.00) // High value
	payment.Currency = "INR"                         // Uncommon
	payment.DebtorBank = "BANKKPXXX"                 // High-risk country
	payment.CreditorBank = "BANKIR33XXX"             // Another high-risk country

	assessment := validator.AssessRisk(payment)

	if assessment.RiskLevel != "CRITICAL" {
		t.Errorf("Risk level = %s, want CRITICAL", assessment.RiskLevel)
	}

	if assessment.Approved {
		t.Error("Critical risk payment should not be approved")
	}

	if assessment.RiskScore < 0.8 {
		t.Errorf("Critical risk score should be >= 0.8, got %f", assessment.RiskScore)
	}
}

func TestAssessRisk_HighLevel(t *testing.T) {
	validator := newTestValidator()
	payment := newValidPayment()
	payment.Amount = decimal.NewFromFloat(200000.00) // High value
	payment.Currency = "INR"                         // Uncommon
	payment.DebtorBank = "BANKGB2LXXX"               // UK
	payment.CreditorBank = "BANKIN33XXX"             // India (cross-border)

	assessment := validator.AssessRisk(payment)

	if assessment.RiskLevel != "HIGH" && assessment.RiskLevel != "MEDIUM" {
		t.Errorf("Risk level = %s, want HIGH or MEDIUM", assessment.RiskLevel)
	}

	if len(assessment.Mitigations) == 0 {
		t.Error("High risk should have mitigations")
	}
}

func TestIsValidCurrency(t *testing.T) {
	tests := []struct {
		currency string
		valid    bool
	}{
		{"USD", true},
		{"EUR", true},
		{"GBP", true},
		{"JPY", true},
		{"CHF", true},
		{"XXX", false},
		{"", false},
		{"usd", false}, // lowercase
	}

	for _, tt := range tests {
		t.Run(tt.currency, func(t *testing.T) {
			result := isValidCurrency(tt.currency)
			if result != tt.valid {
				t.Errorf("isValidCurrency(%s) = %v, want %v", tt.currency, result, tt.valid)
			}
		})
	}
}

func TestBICRegex(t *testing.T) {
	tests := []struct {
		bic   string
		valid bool
	}{
		{"BANKGB2LXXX", true},
		{"BANKGB2L", true},
		{"CHASUS33", true},
		{"DEUTDEFFXXX", true},
		{"INVALID", false},
		{"bankgb2l", false},
		{"BANK123XXX", false},
		{"TOOLONGBIC", false},
		{"", false},
	}

	for _, tt := range tests {
		t.Run(tt.bic, func(t *testing.T) {
			result := bicRegex.MatchString(tt.bic)
			if result != tt.valid {
				t.Errorf("BIC regex match(%s) = %v, want %v", tt.bic, result, tt.valid)
			}
		})
	}
}

func TestIsHighRiskCountry(t *testing.T) {
	tests := []struct {
		country string
		isHigh  bool
	}{
		{"KP", true},  // North Korea
		{"IR", true},  // Iran
		{"SY", true},  // Syria
		{"US", false}, // USA
		{"GB", false}, // UK
		{"", false},
	}

	for _, tt := range tests {
		t.Run(tt.country, func(t *testing.T) {
			result := isHighRiskCountry(tt.country)
			if result != tt.isHigh {
				t.Errorf("isHighRiskCountry(%s) = %v, want %v", tt.country, result, tt.isHigh)
			}
		})
	}
}

func TestIsCommonCurrency(t *testing.T) {
	tests := []struct {
		currency string
		isCommon bool
	}{
		{"USD", true},
		{"EUR", true},
		{"GBP", true},
		{"JPY", false},
		{"INR", false},
		{"XXX", false},
	}

	for _, tt := range tests {
		t.Run(tt.currency, func(t *testing.T) {
			result := isCommonCurrency(tt.currency)
			if result != tt.isCommon {
				t.Errorf("isCommonCurrency(%s) = %v, want %v", tt.currency, result, tt.isCommon)
			}
		})
	}
}

// Helper functions

func containsError(errors []string, target string) bool {
	for _, err := range errors {
		if contains(err, target) {
			return true
		}
	}
	return false
}

func containsWarning(warnings []string, target string) bool {
	for _, warn := range warnings {
		if contains(warn, target) {
			return true
		}
	}
	return false
}

func containsReason(reasons []string, target string) bool {
	for _, reason := range reasons {
		if contains(reason, target) {
			return true
		}
	}
	return false
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) &&
		(s == substr ||
		 len(s) > len(substr) &&
		 (s[:len(substr)] == substr || s[len(s)-len(substr):] == substr))
}

// Benchmarks

func BenchmarkValidatePayment(b *testing.B) {
	validator := newTestValidator()
	payment := newValidPayment()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = validator.ValidatePayment(payment)
	}
}

func BenchmarkCheckSanctions(b *testing.B) {
	validator := newTestValidator()
	payment := newValidPayment()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = validator.CheckSanctions(payment)
	}
}

func BenchmarkAssessRisk(b *testing.B) {
	validator := newTestValidator()
	payment := newValidPayment()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = validator.AssessRisk(payment)
	}
}

func BenchmarkBICRegex(b *testing.B) {
	bic := "BANKGB2LXXX"

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = bicRegex.MatchString(bic)
	}
}
