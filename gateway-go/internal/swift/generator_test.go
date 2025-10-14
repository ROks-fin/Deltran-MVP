package swift

import (
	"strings"
	"testing"
	"time"

	"github.com/shopspring/decimal"
)

func TestNewGenerator(t *testing.T) {
	senderBIC := "BANKGB2LXXX"
	gen := NewGenerator(senderBIC)

	if gen == nil {
		t.Fatal("NewGenerator() returned nil")
	}

	if gen.senderBIC != senderBIC {
		t.Errorf("NewGenerator() senderBIC = %v, want %v", gen.senderBIC, senderBIC)
	}
}

func TestGenerator_GenerateMT103(t *testing.T) {
	gen := NewGenerator("BANKGB2LXXX")

	msg := &MT103Message{
		SenderBIC:           "BANKGB2LXXX",
		ReceiverBIC:         "BANKUS33XXX",
		SenderReference:     "REF123456789",
		Currency:            "USD",
		Amount:              decimal.NewFromFloat(1000.00),
		ValueDate:           time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
		OrderingCustomer:    "JOHN DOE\n123 MAIN STREET\nLONDON",
		BeneficiaryCustomer: "JANE SMITH\n456 BROADWAY\nNEW YORK",
		DetailsOfCharges:    "OUR",
		ApplicationID:       "F",
		ServiceID:           "01",
		MessageType:         "103",
	}

	result, err := gen.GenerateMT103(msg)
	if err != nil {
		t.Fatalf("GenerateMT103() error = %v", err)
	}

	// Check that message contains required blocks
	if !strings.Contains(result, "{1:") {
		t.Error("GenerateMT103() missing block 1")
	}
	if !strings.Contains(result, "{2:") {
		t.Error("GenerateMT103() missing block 2")
	}
	if !strings.Contains(result, "{4:") {
		t.Error("GenerateMT103() missing block 4")
	}

	// Check that required fields are present
	if !strings.Contains(result, ":20:REF123456789") {
		t.Error("GenerateMT103() missing field 20 (sender reference)")
	}
	if !strings.Contains(result, ":32A:") {
		t.Error("GenerateMT103() missing field 32A (value date, currency, amount)")
	}
	if !strings.Contains(result, "USD1000,00") {
		t.Error("GenerateMT103() incorrect amount format")
	}
	if !strings.Contains(result, ":50K:") {
		t.Error("GenerateMT103() missing field 50K (ordering customer)")
	}
	if !strings.Contains(result, ":59:") {
		t.Error("GenerateMT103() missing field 59 (beneficiary)")
	}
}

func TestGenerator_GenerateMT103_MissingRequiredFields(t *testing.T) {
	gen := NewGenerator("BANKGB2LXXX")

	tests := []struct {
		name string
		msg  *MT103Message
	}{
		{
			name: "missing sender reference",
			msg: &MT103Message{
				SenderBIC:   "BANKGB2LXXX",
				ReceiverBIC: "BANKUS33XXX",
				Currency:    "USD",
				Amount:      decimal.NewFromFloat(1000.00),
			},
		},
		{
			name: "missing currency",
			msg: &MT103Message{
				SenderBIC:       "BANKGB2LXXX",
				ReceiverBIC:     "BANKUS33XXX",
				SenderReference: "REF123",
				Amount:          decimal.NewFromFloat(1000.00),
			},
		},
		{
			name: "missing amount",
			msg: &MT103Message{
				SenderBIC:       "BANKGB2LXXX",
				ReceiverBIC:     "BANKUS33XXX",
				SenderReference: "REF123",
				Currency:        "USD",
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := gen.GenerateMT103(tt.msg)
			if err == nil {
				t.Error("GenerateMT103() should fail for missing required fields")
			}
		})
	}
}

func TestGenerator_GenerateMT202(t *testing.T) {
	gen := NewGenerator("BANKGB2LXXX")

	msg := &MT202Message{
		SenderBIC:              "BANKGB2LXXX",
		ReceiverBIC:            "BANKDE5AXXX",
		SenderReference:        "FIREF12345",
		RelatedReference:       "RELREF98765",
		ValueDate:              time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
		Currency:               "EUR",
		Amount:                 decimal.NewFromFloat(5000.00),
		OrderingInstitution:    "BANKGB2LXXX",
		BeneficiaryInstitution: "BANKUS33XXX",
		SenderToReceiverInfo:   "PAYMENT FOR SERVICES",
		ApplicationID:          "F",
		ServiceID:              "01",
		MessageType:            "202",
	}

	result, err := gen.GenerateMT202(msg)
	if err != nil {
		t.Fatalf("GenerateMT202() error = %v", err)
	}

	// Check required fields
	if !strings.Contains(result, ":20:FIREF12345") {
		t.Error("GenerateMT202() missing field 20 (transaction reference)")
	}
	if !strings.Contains(result, ":21:RELREF98765") {
		t.Error("GenerateMT202() missing field 21 (related reference)")
	}
	if !strings.Contains(result, ":32A:") {
		t.Error("GenerateMT202() missing field 32A")
	}
	if !strings.Contains(result, "EUR5000,00") {
		t.Error("GenerateMT202() incorrect amount format")
	}
	if !strings.Contains(result, ":52A:") {
		t.Error("GenerateMT202() missing field 52A (ordering institution)")
	}
	if !strings.Contains(result, ":58A:") {
		t.Error("GenerateMT202() missing field 58A (beneficiary institution)")
	}
}

func TestMT103Builder(t *testing.T) {
	builder := NewMT103Builder()

	msg, err := builder.
		SetSender("BANKGB2LXXX").
		SetReceiver("BANKUS33XXX").
		SetReference("REF123456789").
		SetValueDateCurrencyAmount(time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC), "USD", decimal.NewFromFloat(1000.00)).
		SetOrderingCustomer("JOHN DOE\n123 MAIN STREET\nLONDON").
		SetBeneficiary("", "JANE SMITH\n456 BROADWAY\nNEW YORK").
		SetCharges("OUR").
		Build()

	if err != nil {
		t.Fatalf("MT103Builder.Build() error = %v", err)
	}

	if msg.SenderBIC != "BANKGB2LXXX" {
		t.Errorf("SenderBIC = %v, want BANKGB2LXXX", msg.SenderBIC)
	}
	if msg.ReceiverBIC != "BANKUS33XXX" {
		t.Errorf("ReceiverBIC = %v, want BANKUS33XXX", msg.ReceiverBIC)
	}
	if msg.SenderReference != "REF123456789" {
		t.Errorf("SenderReference = %v, want REF123456789", msg.SenderReference)
	}
	if msg.Currency != "USD" {
		t.Errorf("Currency = %v, want USD", msg.Currency)
	}
	expectedAmount := decimal.NewFromFloat(1000.00)
	if !msg.Amount.Equal(expectedAmount) {
		t.Errorf("Amount = %v, want %v", msg.Amount, expectedAmount)
	}
}

func TestMT103Builder_MissingFields(t *testing.T) {
	tests := []struct {
		name    string
		builder func() *MT103Builder
	}{
		{
			name: "missing sender",
			builder: func() *MT103Builder {
				return NewMT103Builder().
					SetReceiver("BANKUS33XXX").
					SetReference("REF123")
			},
		},
		{
			name: "missing receiver",
			builder: func() *MT103Builder {
				return NewMT103Builder().
					SetSender("BANKGB2LXXX").
					SetReference("REF123")
			},
		},
		{
			name: "missing reference",
			builder: func() *MT103Builder {
				return NewMT103Builder().
					SetSender("BANKGB2LXXX").
					SetReceiver("BANKUS33XXX")
			},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := tt.builder().Build()
			if err == nil {
				t.Error("Build() should fail for missing required fields")
			}
		})
	}
}

func TestMT202Builder(t *testing.T) {
	builder := NewMT202Builder()

	msg, err := builder.
		SetSender("BANKGB2LXXX").
		SetReceiver("BANKDE5AXXX").
		SetReference("FIREF12345").
		SetRelatedReference("RELREF98765").
		SetValueDateCurrencyAmount(time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC), "EUR", decimal.NewFromFloat(5000.00)).
		SetOrderingInstitution("BANKGB2LXXX").
		SetBeneficiaryInstitution("BANKUS33XXX").
		Build()

	if err != nil {
		t.Fatalf("MT202Builder.Build() error = %v", err)
	}

	if msg.SenderReference != "FIREF12345" {
		t.Errorf("SenderReference = %v, want FIREF12345", msg.SenderReference)
	}
	if msg.RelatedReference != "RELREF98765" {
		t.Errorf("RelatedReference = %v, want RELREF98765", msg.RelatedReference)
	}
	if msg.Currency != "EUR" {
		t.Errorf("Currency = %v, want EUR", msg.Currency)
	}
}

func TestGenerator_RoundTrip(t *testing.T) {
	// Test that we can generate and then parse a message
	gen := NewGenerator("BANKGB2LXXX")
	parser := NewParser(false)

	originalMsg := &MT103Message{
		SenderBIC:           "BANKGB2LXXX",
		ReceiverBIC:         "BANKUS33XXX",
		SenderReference:     "REF123456789",
		Currency:            "USD",
		Amount:              decimal.NewFromFloat(1000.00),
		ValueDate:           time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
		OrderingCustomer:    "JOHN DOE",
		BeneficiaryCustomer: "JANE SMITH",
		DetailsOfCharges:    "OUR",
		ApplicationID:       "F",
		ServiceID:           "01",
		MessageType:         "103",
	}

	// Generate
	swiftMsg, err := gen.GenerateMT103(originalMsg)
	if err != nil {
		t.Fatalf("GenerateMT103() error = %v", err)
	}

	// Parse back
	parsedMsg, err := parser.ParseMT103(swiftMsg)
	if err != nil {
		t.Fatalf("ParseMT103() error = %v", err)
	}

	// Verify critical fields match
	if parsedMsg.SenderReference != originalMsg.SenderReference {
		t.Errorf("Round trip failed: SenderReference = %v, want %v",
			parsedMsg.SenderReference, originalMsg.SenderReference)
	}
	if parsedMsg.Currency != originalMsg.Currency {
		t.Errorf("Round trip failed: Currency = %v, want %v",
			parsedMsg.Currency, originalMsg.Currency)
	}
	if !parsedMsg.Amount.Equal(originalMsg.Amount) {
		t.Errorf("Round trip failed: Amount = %v, want %v",
			parsedMsg.Amount, originalMsg.Amount)
	}
}

func TestGenerator_BICPadding(t *testing.T) {
	gen := NewGenerator("BANKGB2L")

	msg := &MT103Message{
		SenderBIC:           "BANKGB2L", // 8 chars
		ReceiverBIC:         "BANKUS33", // 8 chars
		SenderReference:     "REF123",
		Currency:            "USD",
		Amount:              decimal.NewFromFloat(100.00),
		ValueDate:           time.Now(),
		OrderingCustomer:    "CUSTOMER",
		BeneficiaryCustomer: "BENEFICIARY",
		ApplicationID:       "F",
		ServiceID:           "01",
		MessageType:         "103",
	}

	result, err := gen.GenerateMT103(msg)
	if err != nil {
		t.Fatalf("GenerateMT103() error = %v", err)
	}

	// Check that BICs are padded to 11 characters
	if !strings.Contains(result, "BANKGB2LXXX") {
		t.Error("GenerateMT103() should pad sender BIC to 11 characters")
	}
	if !strings.Contains(result, "BANKUS33XXX") {
		t.Error("GenerateMT103() should pad receiver BIC to 11 characters")
	}
}

func TestGenerator_AmountFormatting(t *testing.T) {
	gen := NewGenerator("BANKGB2LXXX")

	tests := []struct {
		name   string
		amount decimal.Decimal
		want   string
	}{
		{
			name:   "whole number",
			amount: decimal.NewFromFloat(1000.00),
			want:   "1000,00",
		},
		{
			name:   "with cents",
			amount: decimal.NewFromFloat(1234.56),
			want:   "1234,56",
		},
		{
			name:   "large amount",
			amount: decimal.NewFromFloat(999999.99),
			want:   "999999,99",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg := &MT103Message{
				SenderBIC:           "BANKGB2LXXX",
				ReceiverBIC:         "BANKUS33XXX",
				SenderReference:     "REF123",
				Currency:            "USD",
				Amount:              tt.amount,
				ValueDate:           time.Now(),
				OrderingCustomer:    "CUSTOMER",
				BeneficiaryCustomer: "BENEFICIARY",
				ApplicationID:       "F",
				ServiceID:           "01",
				MessageType:         "103",
			}

			result, err := gen.GenerateMT103(msg)
			if err != nil {
				t.Fatalf("GenerateMT103() error = %v", err)
			}

			if !strings.Contains(result, tt.want) {
				t.Errorf("GenerateMT103() should contain amount %v, got message:\n%v", tt.want, result)
			}
		})
	}
}

func BenchmarkGenerateMT103(b *testing.B) {
	gen := NewGenerator("BANKGB2LXXX")

	msg := &MT103Message{
		SenderBIC:           "BANKGB2LXXX",
		ReceiverBIC:         "BANKUS33XXX",
		SenderReference:     "REF123456789",
		Currency:            "USD",
		Amount:              decimal.NewFromFloat(1000.00),
		ValueDate:           time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
		OrderingCustomer:    "JOHN DOE",
		BeneficiaryCustomer: "JANE SMITH",
		DetailsOfCharges:    "OUR",
		ApplicationID:       "F",
		ServiceID:           "01",
		MessageType:         "103",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = gen.GenerateMT103(msg)
	}
}

func BenchmarkGenerateMT202(b *testing.B) {
	gen := NewGenerator("BANKGB2LXXX")

	msg := &MT202Message{
		SenderBIC:              "BANKGB2LXXX",
		ReceiverBIC:            "BANKDE5AXXX",
		SenderReference:        "FIREF12345",
		RelatedReference:       "RELREF98765",
		ValueDate:              time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
		Currency:               "EUR",
		Amount:                 decimal.NewFromFloat(5000.00),
		OrderingInstitution:    "BANKGB2LXXX",
		BeneficiaryInstitution: "BANKUS33XXX",
		ApplicationID:          "F",
		ServiceID:              "01",
		MessageType:            "202",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = gen.GenerateMT202(msg)
	}
}

func BenchmarkMT103Builder(b *testing.B) {
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = NewMT103Builder().
			SetSender("BANKGB2LXXX").
			SetReceiver("BANKUS33XXX").
			SetReference("REF123456789").
			SetValueDateCurrencyAmount(time.Now(), "USD", decimal.NewFromFloat(1000.00)).
			SetOrderingCustomer("JOHN DOE").
			SetBeneficiary("", "JANE SMITH").
			Build()
	}
}
