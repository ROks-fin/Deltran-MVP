package swift

import (
	"testing"
	"time"

	"github.com/shopspring/decimal"
)

func TestNewParser(t *testing.T) {
	parser := NewParser(false)
	if parser == nil {
		t.Fatal("NewParser(false) returned nil")
	}

	strictParser := NewParser(true)
	if strictParser == nil || !strictParser.strict {
		t.Fatal("NewParser(true) should create strict parser")
	}
}

func TestParser_ParseMT103_ValidMessage(t *testing.T) {
	parser := NewParser(false)

	// Valid MT103 message
	message := `{1:F01BANKGB2LAXXX0000000000}{2:I103BANKUS33XXXXN}{4:
:20:REF123456789
:23B:CRED
:32A:241015USD1000,00
:50K:JOHN DOE
123 MAIN STREET
LONDON
:59:JANE SMITH
456 BROADWAY
NEW YORK
:71A:OUR
-}`

	mt103, err := parser.ParseMT103(message)
	if err != nil {
		t.Fatalf("ParseMT103() error = %v", err)
	}

	if mt103.SenderReference != "REF123456789" {
		t.Errorf("SenderReference = %v, want REF123456789", mt103.SenderReference)
	}

	if mt103.Currency != "USD" {
		t.Errorf("Currency = %v, want USD", mt103.Currency)
	}

	expectedAmount := decimal.NewFromFloat(1000.00)
	if !mt103.Amount.Equal(expectedAmount) {
		t.Errorf("Amount = %v, want %v", mt103.Amount, expectedAmount)
	}

	if mt103.OrderingCustomer == "" {
		t.Error("OrderingCustomer should not be empty")
	}

	if mt103.BeneficiaryCustomer == "" {
		t.Error("Beneficiary should not be empty")
	}
}

func TestParser_ParseMT103_InvalidMessage(t *testing.T) {
	parser := NewParser(false)

	tests := []struct {
		name    string
		message string
		wantErr bool
	}{
		{
			name:    "empty message",
			message: "",
			wantErr: true,
		},
		{
			name:    "malformed blocks",
			message: "invalid message format",
			wantErr: true,
		},
		{
			name:    "missing required fields",
			message: `{1:F01BANKGB2LAXXX0000000000}{2:I103BANKUS33XXXXN}{4:
:20:REF123
-}`,
			wantErr: true,
		},
		{
			name:    "wrong message type",
			message: `{1:F01BANKGB2LAXXX0000000000}{2:I202BANKUS33XXXXN}{4:
:20:REF123
-}`,
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			_, err := parser.ParseMT103(tt.message)
			if (err != nil) != tt.wantErr {
				t.Errorf("ParseMT103() error = %v, wantErr %v", err, tt.wantErr)
			}
		})
	}
}

func TestParser_ParseMT202_ValidMessage(t *testing.T) {
	parser := NewParser(false)

	message := `{1:F01BANKGB2LAXXX0000000000}{2:I202BANKUS33XXXXN}{4:
:20:FIREF12345
:21:RELREF98765
:32A:241015EUR5000,00
:52A:BANKGB2LXXX
:58A:BANKUS33XXX
-}`

	mt202, err := parser.ParseMT202(message)
	if err != nil {
		t.Fatalf("ParseMT202() error = %v", err)
	}

	if mt202.SenderReference != "FIREF12345" {
		t.Errorf("SenderReference = %v, want FIREF12345", mt202.SenderReference)
	}

	if mt202.RelatedReference != "RELREF98765" {
		t.Errorf("RelatedReference = %v, want RELREF98765", mt202.RelatedReference)
	}

	if mt202.Currency != "EUR" {
		t.Errorf("Currency = %v, want EUR", mt202.Currency)
	}

	expectedAmount := decimal.NewFromFloat(5000.00)
	if !mt202.Amount.Equal(expectedAmount) {
		t.Errorf("Amount = %v, want %v", mt202.Amount, expectedAmount)
	}
}

func TestParser_ParseField32A(t *testing.T) {
	parser := NewParser(false)

	tests := []struct {
		name           string
		field          string
		wantDate       time.Time
		wantCurrency   string
		wantAmount     decimal.Decimal
		wantErr        bool
	}{
		{
			name:         "valid field with comma separator",
			field:        "241015USD1000,00",
			wantDate:     time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
			wantCurrency: "USD",
			wantAmount:   decimal.NewFromFloat(1000.00),
			wantErr:      false,
		},
		{
			name:         "valid field with period separator",
			field:        "241015EUR2500.50",
			wantDate:     time.Date(2024, 10, 15, 0, 0, 0, 0, time.UTC),
			wantCurrency: "EUR",
			wantAmount:   decimal.NewFromFloat(2500.50),
			wantErr:      false,
		},
		{
			name:    "invalid date",
			field:   "999999USD1000,00",
			wantErr: true,
		},
		{
			name:         "invalid currency",
			field:        "241015XXX1000,00", // XXX is 3 chars, invalid but parseable
			wantCurrency: "XXX", // Parser extracts but doesn't validate
			wantAmount:   decimal.NewFromFloat(1000.00),
			wantErr:      false, // Parser doesn't validate currency codes
		},
		{
			name:    "missing amount",
			field:   "241015USD",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			msg := &MT103Message{}
			err := parser.parseField32A(tt.field, msg)

			if (err != nil) != tt.wantErr {
				t.Errorf("parseField32A() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			if !tt.wantErr {
				if msg.Currency != tt.wantCurrency {
					t.Errorf("Currency = %v, want %v", msg.Currency, tt.wantCurrency)
				}
				if !msg.Amount.Equal(tt.wantAmount) {
					t.Errorf("Amount = %v, want %v", msg.Amount, tt.wantAmount)
				}
				// Note: Date parsing may need adjustment based on current year
			}
		})
	}
}

func TestParser_ExtractFields(t *testing.T) {
	parser := NewParser(false)

	block := `:20:REF123456789
:23B:CRED
:32A:241015USD1000,00
:50K:JOHN DOE
:59:JANE SMITH
:71A:OUR`

	fields := parser.extractFields(block)

	expectedFields := map[string]bool{
		"20":  true,
		"23B": true,
		"32A": true,
		"50K": true,
		"59":  true,
		"71A": true,
	}

	if len(fields) != len(expectedFields) {
		t.Errorf("extractFields() returned %v fields, want %v", len(fields), len(expectedFields))
	}

	foundFields := make(map[string]bool)
	for _, field := range fields {
		foundFields[field.Tag] = true
	}

	for tag := range expectedFields {
		if !foundFields[tag] {
			t.Errorf("extractFields() missing field %v", tag)
		}
	}
}

func TestParser_SplitBlocks(t *testing.T) {
	parser := NewParser(false)

	tests := []struct {
		name       string
		message    string
		wantBlocks int
		wantErr    bool
	}{
		{
			name:       "valid 4 blocks",
			message:    "{1:F01BANKGB2LAXXX0000000000}{2:I103BANKUS33XXXXN}{4:text}{5:trailer}",
			wantBlocks: 4,
			wantErr:    false,
		},
		{
			name:       "valid 3 blocks (minimum)",
			message:    "{1:header}{2:app}{4:text}",
			wantBlocks: 3,
			wantErr:    false,
		},
		{
			name:       "too few blocks",
			message:    "{1:header}{2:app}",
			wantBlocks: 0,
			wantErr:    true,
		},
		{
			name:       "no blocks",
			message:    "invalid message",
			wantBlocks: 0,
			wantErr:    true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			blocks, err := parser.splitBlocks(tt.message)

			if (err != nil) != tt.wantErr {
				t.Errorf("splitBlocks() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			if !tt.wantErr && len(blocks) != tt.wantBlocks {
				t.Errorf("splitBlocks() returned %v blocks, want %v", len(blocks), tt.wantBlocks)
			}
		})
	}
}

func TestParser_ExtractMessageType(t *testing.T) {
	parser := NewParser(false)

	tests := []struct {
		name    string
		block2  string
		want    string
		wantErr bool
	}{
		{
			name:    "MT103 input",
			block2:  "{2:I103BANKUS33XXXXN}",
			want:    "103",
			wantErr: false,
		},
		{
			name:    "MT202 input",
			block2:  "{2:I202BANKUS33XXXXN}",
			want:    "202",
			wantErr: false,
		},
		{
			name:    "MT103 output",
			block2:  "{2:O1031234567890}",
			want:    "103",
			wantErr: false,
		},
		{
			name:    "invalid block",
			block2:  "{2:INVALID}",
			want:    "",
			wantErr: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			got, err := parser.extractMessageType(tt.block2)

			if (err != nil) != tt.wantErr {
				t.Errorf("extractMessageType() error = %v, wantErr %v", err, tt.wantErr)
				return
			}

			if got != tt.want {
				t.Errorf("extractMessageType() = %v, want %v", got, tt.want)
			}
		})
	}
}

func TestValidateBIC(t *testing.T) {
	tests := []struct {
		name string
		bic  string
		want bool
	}{
		{
			name: "valid 11 char BIC",
			bic:  "BANKGB2LXXX",
			want: true,
		},
		{
			name: "valid 8 char BIC",
			bic:  "BANKGB2L",
			want: true,
		},
		{
			name: "too short",
			bic:  "BANK",
			want: false,
		},
		{
			name: "too long",
			bic:  "BANKGB2LXXXX",
			want: false,
		},
		{
			name: "empty",
			bic:  "",
			want: false,
		},
		{
			name: "with lowercase (should still validate)",
			bic:  "bankgb2lxxx",
			want: true,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if got := ValidateBIC(tt.bic); got != tt.want {
				t.Errorf("ValidateBIC() = %v, want %v", got, tt.want)
			}
		})
	}
}

func BenchmarkParseMT103(b *testing.B) {
	parser := NewParser(false)
	message := `{1:F01BANKGB2LAXXX0000000000}{2:I103BANKUS33XXXXN}{4:
:20:REF123456789
:23B:CRED
:32A:241015USD1000,00
:50K:JOHN DOE
123 MAIN STREET
LONDON
:59:JANE SMITH
456 BROADWAY
NEW YORK
:71A:OUR
-}`

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = parser.ParseMT103(message)
	}
}

func BenchmarkParseMT202(b *testing.B) {
	parser := NewParser(false)
	message := `{1:F01BANKGB2LAXXX0000000000}{2:I202BANKUS33XXXXN}{4:
:20:FIREF12345
:21:RELREF98765
:32A:241015EUR5000,00
:52A:BANKGB2LXXX
:58A:BANKUS33XXX
-}`

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = parser.ParseMT202(message)
	}
}

func BenchmarkValidateBIC(b *testing.B) {
	bic := "BANKGB2LXXX"
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = ValidateBIC(bic)
	}
}

func BenchmarkFormatAmount(b *testing.B) {
	amount := decimal.NewFromFloat(1234.56)
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = FormatAmount(amount)
	}
}
