package swift

import (
	"testing"
	"time"

	"github.com/shopspring/decimal"
)

// TestMT103RoundTrip tests that we can generate and parse MT103 messages
func TestMT103RoundTrip(t *testing.T) {
	// Build MT103 message
	builder := NewMT103Builder()
	original, err := builder.
		SetSender("BANKGB2LXXX").
		SetReceiver("BANKUS33XXX").
		SetReference("REF20251012001").
		SetValueDateCurrencyAmount(time.Date(2025, 10, 12, 0, 0, 0, 0, time.UTC), "USD", decimal.NewFromFloat(1000.50)).
		SetOrderingCustomer("John Doe\n123 Main Street\nLondon").
		SetBeneficiary("US1234567890", "Jane Smith\n456 Broadway\nNew York").
		SetRemittanceInfo("Invoice INV-2025-001").
		SetCharges("SHA").
		Build()

	if err != nil {
		t.Fatalf("Failed to build MT103: %v", err)
	}

	// Generate SWIFT message
	generator := NewGenerator("BANKGB2LXXX")
	swiftMessage, err := generator.GenerateMT103(original)
	if err != nil {
		t.Fatalf("Failed to generate MT103: %v", err)
	}

	t.Logf("Generated SWIFT MT103:\n%s", swiftMessage)

	// Parse it back
	parser := NewParser(false)
	parsed, err := parser.ParseMT103(swiftMessage)
	if err != nil {
		t.Fatalf("Failed to parse MT103: %v", err)
	}

	// Verify critical fields match
	if parsed.SenderReference != original.SenderReference {
		t.Errorf("Reference mismatch: got %s, want %s", parsed.SenderReference, original.SenderReference)
	}

	if parsed.Currency != original.Currency {
		t.Errorf("Currency mismatch: got %s, want %s", parsed.Currency, original.Currency)
	}

	if !parsed.Amount.Equal(original.Amount) {
		t.Errorf("Amount mismatch: got %s, want %s", parsed.Amount, original.Amount)
	}

	if parsed.ValueDate.Format("2006-01-02") != original.ValueDate.Format("2006-01-02") {
		t.Errorf("Value date mismatch: got %s, want %s",
			parsed.ValueDate.Format("2006-01-02"),
			original.ValueDate.Format("2006-01-02"))
	}

	if parsed.DetailsOfCharges != original.DetailsOfCharges {
		t.Errorf("Charges mismatch: got %s, want %s", parsed.DetailsOfCharges, original.DetailsOfCharges)
	}
}

// TestMT202RoundTrip tests MT202 generation and parsing
func TestMT202RoundTrip(t *testing.T) {
	// Build MT202 message
	builder := NewMT202Builder()
	original, err := builder.
		SetSender("BANKGB2LXXX").
		SetReceiver("BANKDE5AXXX").
		SetReference("REF20251012002").
		SetRelatedReference("REF20251012001").
		SetValueDateCurrencyAmount(time.Date(2025, 10, 12, 0, 0, 0, 0, time.UTC), "EUR", decimal.NewFromFloat(5000.00)).
		SetOrderingInstitution("BANKGB2LXXX").
		SetBeneficiaryInstitution("BANKDE5AXXX").
		Build()

	if err != nil {
		t.Fatalf("Failed to build MT202: %v", err)
	}

	// Generate SWIFT message
	generator := NewGenerator("BANKGB2LXXX")
	swiftMessage, err := generator.GenerateMT202(original)
	if err != nil {
		t.Fatalf("Failed to generate MT202: %v", err)
	}

	t.Logf("Generated SWIFT MT202:\n%s", swiftMessage)

	// Parse it back
	parser := NewParser(false)
	parsed, err := parser.ParseMT202(swiftMessage)
	if err != nil {
		t.Fatalf("Failed to parse MT202: %v", err)
	}

	// Verify critical fields
	if parsed.SenderReference != original.SenderReference {
		t.Errorf("Reference mismatch: got %s, want %s", parsed.SenderReference, original.SenderReference)
	}

	if parsed.RelatedReference != original.RelatedReference {
		t.Errorf("Related reference mismatch: got %s, want %s", parsed.RelatedReference, original.RelatedReference)
	}

	if parsed.Currency != original.Currency {
		t.Errorf("Currency mismatch: got %s, want %s", parsed.Currency, original.Currency)
	}

	if !parsed.Amount.Equal(original.Amount) {
		t.Errorf("Amount mismatch: got %s, want %s", parsed.Amount, original.Amount)
	}
}

// TestBICValidation tests BIC code validation
func TestBICValidation(t *testing.T) {
	tests := []struct {
		bic   string
		valid bool
	}{
		{"BANKGB2LXXX", true},   // 11 chars
		{"BANKGB2L", true},       // 8 chars
		{"DEUTDEFFXXX", true},    // 11 chars
		{"CHASUS33", true},       // 8 chars
		{"INVALID", false},       // Too short
		{"TOOLONGBICCODE", false}, // Too long
		{"bank gb2l", false},     // Lowercase/spaces
		{"BANK123XXX", false},    // Invalid format
	}

	for _, tt := range tests {
		t.Run(tt.bic, func(t *testing.T) {
			valid := ValidateBIC(tt.bic)
			if valid != tt.valid {
				t.Errorf("ValidateBIC(%s) = %v, want %v", tt.bic, valid, tt.valid)
			}
		})
	}
}

// TestFormatAmount tests amount formatting
func TestFormatAmount(t *testing.T) {
	tests := []struct {
		amount   string
		expected string
	}{
		{"1000.00", "1000,00"},
		{"1234.56", "1234,56"},
		{"0.01", "0,01"},
		{"999999.99", "999999,99"},
	}

	for _, tt := range tests {
		t.Run(tt.amount, func(t *testing.T) {
			amount, _ := decimal.NewFromString(tt.amount)
			formatted := FormatAmount(amount)
			if formatted != tt.expected {
				t.Errorf("FormatAmount(%s) = %s, want %s", tt.amount, formatted, tt.expected)
			}
		})
	}
}

// TestFormatDate tests date formatting
func TestFormatDate(t *testing.T) {
	date := time.Date(2025, 10, 12, 0, 0, 0, 0, time.UTC)
	formatted := FormatDate(date)
	expected := "251012"

	if formatted != expected {
		t.Errorf("FormatDate() = %s, want %s", formatted, expected)
	}
}

// TestGenerateReference tests reference generation
func TestGenerateReference(t *testing.T) {
	ref1 := GenerateReference("PAY")
	ref2 := GenerateReference("PAY")

	if ref1 == ref2 {
		t.Error("GenerateReference() should produce unique references")
	}

	if len(ref1) < 4 {
		t.Error("GenerateReference() should produce non-empty reference")
	}

	if ref1[:3] != "PAY" {
		t.Errorf("GenerateReference() should start with prefix, got %s", ref1)
	}
}

// TestMT103MandatoryFields tests that mandatory fields are validated
func TestMT103MandatoryFields(t *testing.T) {
	tests := []struct {
		name      string
		buildFunc func(*MT103Builder) *MT103Builder
		wantError bool
	}{
		{
			name: "missing sender",
			buildFunc: func(b *MT103Builder) *MT103Builder {
				return b.SetReceiver("BANKUS33XXX").
					SetReference("REF001").
					SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(100))
			},
			wantError: true,
		},
		{
			name: "missing receiver",
			buildFunc: func(b *MT103Builder) *MT103Builder {
				return b.SetSender("BANKGB2LXXX").
					SetReference("REF001").
					SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(100))
			},
			wantError: true,
		},
		{
			name: "missing reference",
			buildFunc: func(b *MT103Builder) *MT103Builder {
				return b.SetSender("BANKGB2LXXX").
					SetReceiver("BANKUS33XXX").
					SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(100))
			},
			wantError: true,
		},
		{
			name: "valid minimum",
			buildFunc: func(b *MT103Builder) *MT103Builder {
				return b.SetSender("BANKGB2LXXX").
					SetReceiver("BANKUS33XXX").
					SetReference("REF001").
					SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(100)).
					SetOrderingCustomer("Customer").
					SetBeneficiary("ACC123", "Beneficiary")
			},
			wantError: false,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			builder := NewMT103Builder()
			builder = tt.buildFunc(builder)
			_, err := builder.Build()

			if (err != nil) != tt.wantError {
				t.Errorf("Build() error = %v, wantError %v", err, tt.wantError)
			}
		})
	}
}

// BenchmarkMT103Generation benchmarks MT103 message generation
func BenchmarkMT103Generation(b *testing.B) {
	builder := NewMT103Builder()
	msg, _ := builder.
		SetSender("BANKGB2LXXX").
		SetReceiver("BANKUS33XXX").
		SetReference("REF001").
		SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(1000)).
		SetOrderingCustomer("Customer").
		SetBeneficiary("ACC123", "Beneficiary").
		Build()

	generator := NewGenerator("BANKGB2LXXX")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = generator.GenerateMT103(msg)
	}
}

// BenchmarkMT103Parsing benchmarks MT103 message parsing
func BenchmarkMT103Parsing(b *testing.B) {
	// Pre-generate message
	builder := NewMT103Builder()
	msg, _ := builder.
		SetSender("BANKGB2LXXX").
		SetReceiver("BANKUS33XXX").
		SetReference("REF001").
		SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(1000)).
		SetOrderingCustomer("Customer").
		SetBeneficiary("ACC123", "Beneficiary").
		Build()

	generator := NewGenerator("BANKGB2LXXX")
	swiftMsg, _ := generator.GenerateMT103(msg)

	parser := NewParser(false)

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = parser.ParseMT103(swiftMsg)
	}
}
