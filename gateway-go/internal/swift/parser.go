package swift

import (
	"errors"
	"fmt"
	"regexp"
	"strings"
	"time"

	"github.com/shopspring/decimal"
)

var (
	ErrInvalidMessage      = errors.New("invalid SWIFT message format")
	ErrInvalidBlock        = errors.New("invalid block format")
	ErrMissingField        = errors.New("missing required field")
	ErrInvalidFieldFormat  = errors.New("invalid field format")
	ErrUnsupportedMessage  = errors.New("unsupported message type")
)

// MessageType represents SWIFT message type
type MessageType string

const (
	MT103 MessageType = "103" // Single Customer Credit Transfer
	MT202 MessageType = "202" // Financial Institution Transfer
)

// MT103Message represents a SWIFT MT103 message
type MT103Message struct {
	// Block 1: Basic Header
	ApplicationID     string // F = FIN, A = GPA
	ServiceID         string // 01 = FIN
	SenderBIC         string // 12 characters
	SessionNumber     string // 4 digits
	SequenceNumber    string // 6 digits

	// Block 2: Application Header
	MessagePriority   string // U = Urgent, N = Normal, S = System
	MessageType       string // 103
	ReceiverBIC       string // 12 characters
	DeliveryMonitoring string // 2 = Non-delivery warning, 3 = Delivery notification

	// Block 3: User Header (optional)
	ValidationFlag    string // 119: Validation Flag
	ServiceTypeID     string // 103: Service Type Identifier

	// Block 4: Text Block (Message Body)
	// Mandatory fields
	SenderReference       string          // :20: Transaction Reference
	ValueDate             time.Time       // :32A: Value Date
	Currency              string          // :32A: Currency Code
	Amount                decimal.Decimal // :32A: Amount
	OrderingCustomer      string          // :50K: Ordering Customer
	BeneficiaryCustomer   string          // :59: Beneficiary Customer

	// Optional fields
	OrderingInstitution   string          // :52A/D: Ordering Institution
	SendersCorrespondent  string          // :53A/B/D: Sender's Correspondent
	ReceiversCorrespondent string         // :54A/B/D: Receiver's Correspondent
	IntermediaryInstitution string        // :56A/C/D: Intermediary
	AccountWithInstitution string         // :57A/B/C/D: Account With Institution
	BeneficiaryAccount    string          // :59: Beneficiary Account
	RemittanceInfo        string          // :70: Remittance Information
	DetailsOfCharges      string          // :71A: Details of Charges (OUR/BEN/SHA)
	SenderToReceiverInfo  string          // :72: Sender to Receiver Information

	// Block 5: Trailers (optional)
	Checksum              string // MAC/CHK
	MessageAuthCode       string // MAC
}

// MT202Message represents a SWIFT MT202 message
type MT202Message struct {
	// Block 1: Basic Header
	ApplicationID     string
	ServiceID         string
	SenderBIC         string
	SessionNumber     string
	SequenceNumber    string

	// Block 2: Application Header
	MessagePriority   string
	MessageType       string
	ReceiverBIC       string
	DeliveryMonitoring string

	// Block 3: User Header (optional)
	ValidationFlag    string
	ServiceTypeID     string

	// Block 4: Text Block
	// Mandatory fields
	SenderReference       string          // :20: Transaction Reference
	RelatedReference      string          // :21: Related Reference
	ValueDate             time.Time       // :32A: Value Date
	Currency              string          // :32A: Currency Code
	Amount                decimal.Decimal // :32A: Amount
	OrderingInstitution   string          // :52A/D: Ordering Institution
	BeneficiaryInstitution string         // :58A/D: Beneficiary Institution

	// Optional fields
	SendersCorrespondent  string          // :53A/B/D: Sender's Correspondent
	ReceiversCorrespondent string         // :54A/B/D: Receiver's Correspondent
	IntermediaryInstitution string        // :56A/C/D: Intermediary
	AccountWithInstitution string         // :57A/B/C/D: Account With Institution
	SenderToReceiverInfo  string          // :72: Sender to Receiver Information
}

// Parser parses SWIFT messages
type Parser struct {
	strict bool // Strict mode enforces all validations
}

// NewParser creates a new SWIFT parser
func NewParser(strict bool) *Parser {
	return &Parser{strict: strict}
}

// Parse parses a SWIFT message and returns the appropriate type
func (p *Parser) Parse(message string) (interface{}, error) {
	// Split into blocks
	blocks, err := p.splitBlocks(message)
	if err != nil {
		return nil, err
	}

	// Determine message type from Block 2 (blocks[1])
	messageType, err := p.extractMessageType(blocks[1])
	if err != nil {
		return nil, err
	}

	switch MessageType(messageType) {
	case MT103:
		return p.ParseMT103(message)
	case MT202:
		return p.ParseMT202(message)
	default:
		return nil, fmt.Errorf("%w: %s", ErrUnsupportedMessage, messageType)
	}
}

// ParseMT103 parses a SWIFT MT103 message
func (p *Parser) ParseMT103(message string) (*MT103Message, error) {
	msg := &MT103Message{}

	// Split into blocks
	blocks, err := p.splitBlocks(message)
	if err != nil {
		return nil, err
	}

	// Parse Block 1: Basic Header (blocks[0])
	if err := p.parseBlock1(blocks[0], msg); err != nil {
		return nil, fmt.Errorf("block 1 error: %w", err)
	}

	// Parse Block 2: Application Header (blocks[1])
	if err := p.parseBlock2MT103(blocks[1], msg); err != nil {
		return nil, fmt.Errorf("block 2 error: %w", err)
	}

	// Parse Block 3: User Header (optional) (blocks[2] if present)
	if len(blocks) > 2 && strings.HasPrefix(blocks[2], "{3:") {
		p.parseBlock3(blocks[2], msg)
		// Block 4 will be at index 3
		if len(blocks) <= 3 {
			return nil, ErrInvalidMessage
		}
		if err := p.parseBlock4MT103(blocks[3], msg); err != nil {
			return nil, fmt.Errorf("block 4 error: %w", err)
		}
	} else {
		// No Block 3, Block 4 is at index 2
		if len(blocks) <= 2 {
			return nil, ErrInvalidMessage
		}
		if err := p.parseBlock4MT103(blocks[2], msg); err != nil {
			return nil, fmt.Errorf("block 4 error: %w", err)
		}
	}

	// Validate mandatory fields
	if err := p.validateMT103(msg); err != nil {
		return nil, err
	}

	return msg, nil
}

// ParseMT202 parses a SWIFT MT202 message
func (p *Parser) ParseMT202(message string) (*MT202Message, error) {
	msg := &MT202Message{}

	// Split into blocks
	blocks, err := p.splitBlocks(message)
	if err != nil {
		return nil, err
	}

	// Parse Block 1: Basic Header (blocks[0])
	if err := p.parseBlock1MT202(blocks[0], msg); err != nil {
		return nil, fmt.Errorf("block 1 error: %w", err)
	}

	// Parse Block 2: Application Header (blocks[1])
	if err := p.parseBlock2MT202(blocks[1], msg); err != nil {
		return nil, fmt.Errorf("block 2 error: %w", err)
	}

	// Parse Block 3: User Header (optional) (blocks[2] if present)
	if len(blocks) > 2 && strings.HasPrefix(blocks[2], "{3:") {
		p.parseBlock3MT202(blocks[2], msg)
		// Block 4 will be at index 3
		if len(blocks) <= 3 {
			return nil, ErrInvalidMessage
		}
		if err := p.parseBlock4MT202(blocks[3], msg); err != nil {
			return nil, fmt.Errorf("block 4 error: %w", err)
		}
	} else {
		// No Block 3, Block 4 is at index 2
		if len(blocks) <= 2 {
			return nil, ErrInvalidMessage
		}
		if err := p.parseBlock4MT202(blocks[2], msg); err != nil {
			return nil, fmt.Errorf("block 4 error: %w", err)
		}
	}

	// Validate mandatory fields
	if err := p.validateMT202(msg); err != nil {
		return nil, err
	}

	return msg, nil
}

// splitBlocks splits a SWIFT message into blocks
func (p *Parser) splitBlocks(message string) ([]string, error) {
	// SWIFT message format:
	// {1:...}{2:...}{3:...}{4:...}{5:...}

	blockRegex := regexp.MustCompile(`\{[1-5]:[^\}]*\}`)
	matches := blockRegex.FindAllString(message, -1)

	if len(matches) < 3 {
		return nil, ErrInvalidMessage
	}

	return matches, nil
}

// extractMessageType extracts message type from Block 2
func (p *Parser) extractMessageType(block2 string) (string, error) {
	// Block 2 format: {2:I103...} or {2:O103...}
	re := regexp.MustCompile(`\{2:[IO](\d{3})`)
	matches := re.FindStringSubmatch(block2)
	if len(matches) < 2 {
		return "", ErrInvalidBlock
	}
	return matches[1], nil
}

// parseBlock1 parses Block 1 (Basic Header) for MT103
func (p *Parser) parseBlock1(block string, msg *MT103Message) error {
	// Format: {1:F01BANKBEBBAXXX1234123456}
	// F = Application ID (1 char)
	// 01 = Service ID (2 chars)
	// BANKBEBBAXXX = Sender BIC (12 chars)
	// 1234 = Session Number (4 chars)
	// 123456 = Sequence Number (6 chars)
	// Total: 1 + 2 + 12 + 4 + 6 = 25 chars

	block = strings.TrimPrefix(block, "{1:")
	block = strings.TrimSuffix(block, "}")

	if len(block) < 25 {
		return fmt.Errorf("%w: block 1 length %d, expected 25", ErrInvalidBlock, len(block))
	}

	msg.ApplicationID = block[0:1]
	msg.ServiceID = block[1:3]
	msg.SenderBIC = block[3:15]
	msg.SessionNumber = block[15:19]
	msg.SequenceNumber = block[19:25]

	return nil
}

// parseBlock2MT103 parses Block 2 (Application Header) for MT103
func (p *Parser) parseBlock2MT103(block string, msg *MT103Message) error {
	// Input format: {2:I103BANKDEFFXXXXN}
	// I/O = Input/Output
	// 103 = Message Type
	// BANKDEFFXXXX = Receiver BIC
	// N = Priority (U/N/S)

	block = strings.TrimPrefix(block, "{2:")
	block = strings.TrimSuffix(block, "}")

	if len(block) < 16 {
		return ErrInvalidBlock
	}

	msg.MessageType = block[1:4]
	msg.ReceiverBIC = block[4:16]
	if len(block) > 16 {
		msg.MessagePriority = block[16:17]
	}

	return nil
}

// parseBlock3 parses Block 3 (User Header) for MT103
func (p *Parser) parseBlock3(block string, msg *MT103Message) {
	// Format: {3:{119:STP}{103:EBA}}
	block = strings.TrimPrefix(block, "{3:")
	block = strings.TrimSuffix(block, "}")

	// Extract fields
	if strings.Contains(block, "{119:") {
		re := regexp.MustCompile(`\{119:([^\}]+)\}`)
		if matches := re.FindStringSubmatch(block); len(matches) > 1 {
			msg.ValidationFlag = matches[1]
		}
	}
	if strings.Contains(block, "{103:") {
		re := regexp.MustCompile(`\{103:([^\}]+)\}`)
		if matches := re.FindStringSubmatch(block); len(matches) > 1 {
			msg.ServiceTypeID = matches[1]
		}
	}
}

// parseBlock4MT103 parses Block 4 (Text Block) for MT103
func (p *Parser) parseBlock4MT103(block string, msg *MT103Message) error {
	// Format: {4:\n:20:REF123\n:32A:250112USD1000,00\n:50K:...}

	block = strings.TrimPrefix(block, "{4:")
	block = strings.TrimSuffix(block, "}")

	// Split into fields
	fields := p.extractFields(block)

	for _, field := range fields {
		tag := field.Tag
		value := field.Value

		switch tag {
		case "20":
			msg.SenderReference = value
		case "32A":
			if err := p.parseField32A(value, msg); err != nil {
				return err
			}
		case "50K", "50F":
			msg.OrderingCustomer = value
		case "52A", "52D":
			msg.OrderingInstitution = value
		case "53A", "53B", "53D":
			msg.SendersCorrespondent = value
		case "54A", "54B", "54D":
			msg.ReceiversCorrespondent = value
		case "56A", "56C", "56D":
			msg.IntermediaryInstitution = value
		case "57A", "57B", "57C", "57D":
			msg.AccountWithInstitution = value
		case "59", "59A":
			p.parseField59(value, msg)
		case "70":
			msg.RemittanceInfo = value
		case "71A":
			msg.DetailsOfCharges = value
		case "72":
			msg.SenderToReceiverInfo = value
		}
	}

	return nil
}

// Field represents a SWIFT field
type Field struct {
	Tag   string
	Value string
}

// extractFields extracts fields from text block
func (p *Parser) extractFields(block string) []Field {
	var fields []Field

	// Pattern: :TAG:VALUE
	re := regexp.MustCompile(`:(\d{2}[A-Z]?):((?:[^\n:]|:[^\d])+)`)
	matches := re.FindAllStringSubmatch(block, -1)

	for _, match := range matches {
		if len(match) >= 3 {
			fields = append(fields, Field{
				Tag:   match[1],
				Value: strings.TrimSpace(match[2]),
			})
		}
	}

	return fields
}

// parseField32A parses field :32A: (Value Date, Currency, Amount)
func (p *Parser) parseField32A(value string, msg *MT103Message) error {
	// Format: YYMMDDCCCAMOUNT
	// Example: 250112USD1000,00

	if len(value) < 9 {
		return ErrInvalidFieldFormat
	}

	// Parse date (YYMMDD)
	dateStr := value[0:6]
	date, err := time.Parse("060102", dateStr)
	if err != nil {
		return fmt.Errorf("invalid date format: %w", err)
	}
	msg.ValueDate = date

	// Parse currency (3 chars)
	msg.Currency = value[6:9]

	// Parse amount (rest)
	amountStr := value[9:]
	amountStr = strings.ReplaceAll(amountStr, ",", ".")
	amount, err := decimal.NewFromString(amountStr)
	if err != nil {
		return fmt.Errorf("invalid amount format: %w", err)
	}
	msg.Amount = amount

	return nil
}

// parseField59 parses field :59: (Beneficiary)
func (p *Parser) parseField59(value string, msg *MT103Message) {
	// Format: Can contain account number and name/address
	// First line might be account, rest is name/address

	lines := strings.Split(value, "\n")
	if len(lines) > 0 {
		// Check if first line is account number (starts with /)
		if strings.HasPrefix(lines[0], "/") {
			msg.BeneficiaryAccount = strings.TrimPrefix(lines[0], "/")
			if len(lines) > 1 {
				msg.BeneficiaryCustomer = strings.Join(lines[1:], "\n")
			}
		} else {
			msg.BeneficiaryCustomer = value
		}
	}
}

// validateMT103 validates MT103 mandatory fields
func (p *Parser) validateMT103(msg *MT103Message) error {
	if msg.SenderReference == "" {
		return fmt.Errorf("%w: field 20 (Sender's Reference)", ErrMissingField)
	}
	if msg.ValueDate.IsZero() {
		return fmt.Errorf("%w: field 32A (Value Date)", ErrMissingField)
	}
	if msg.Currency == "" {
		return fmt.Errorf("%w: field 32A (Currency)", ErrMissingField)
	}
	if msg.Amount.IsZero() {
		return fmt.Errorf("%w: field 32A (Amount)", ErrMissingField)
	}
	if msg.OrderingCustomer == "" {
		return fmt.Errorf("%w: field 50 (Ordering Customer)", ErrMissingField)
	}
	if msg.BeneficiaryCustomer == "" && msg.BeneficiaryAccount == "" {
		return fmt.Errorf("%w: field 59 (Beneficiary)", ErrMissingField)
	}

	return nil
}

// MT202 parsing methods

func (p *Parser) parseBlock1MT202(block string, msg *MT202Message) error {
	block = strings.TrimPrefix(block, "{1:")
	block = strings.TrimSuffix(block, "}")

	if len(block) < 25 {
		return fmt.Errorf("%w: block 1 length %d, expected 25", ErrInvalidBlock, len(block))
	}

	msg.ApplicationID = block[0:1]
	msg.ServiceID = block[1:3]
	msg.SenderBIC = block[3:15]
	msg.SessionNumber = block[15:19]
	msg.SequenceNumber = block[19:25]

	return nil
}

func (p *Parser) parseBlock2MT202(block string, msg *MT202Message) error {
	block = strings.TrimPrefix(block, "{2:")
	block = strings.TrimSuffix(block, "}")

	if len(block) < 16 {
		return ErrInvalidBlock
	}

	msg.MessageType = block[1:4]
	msg.ReceiverBIC = block[4:16]
	if len(block) > 16 {
		msg.MessagePriority = block[16:17]
	}

	return nil
}

func (p *Parser) parseBlock3MT202(block string, msg *MT202Message) {
	block = strings.TrimPrefix(block, "{3:")
	block = strings.TrimSuffix(block, "}")

	if strings.Contains(block, "{119:") {
		re := regexp.MustCompile(`\{119:([^\}]+)\}`)
		if matches := re.FindStringSubmatch(block); len(matches) > 1 {
			msg.ValidationFlag = matches[1]
		}
	}
	if strings.Contains(block, "{103:") {
		re := regexp.MustCompile(`\{103:([^\}]+)\}`)
		if matches := re.FindStringSubmatch(block); len(matches) > 1 {
			msg.ServiceTypeID = matches[1]
		}
	}
}

func (p *Parser) parseBlock4MT202(block string, msg *MT202Message) error {
	block = strings.TrimPrefix(block, "{4:")
	block = strings.TrimSuffix(block, "}")

	fields := p.extractFields(block)

	for _, field := range fields {
		tag := field.Tag
		value := field.Value

		switch tag {
		case "20":
			msg.SenderReference = value
		case "21":
			msg.RelatedReference = value
		case "32A":
			if err := p.parseField32AMT202(value, msg); err != nil {
				return err
			}
		case "52A", "52D":
			msg.OrderingInstitution = value
		case "53A", "53B", "53D":
			msg.SendersCorrespondent = value
		case "54A", "54B", "54D":
			msg.ReceiversCorrespondent = value
		case "56A", "56C", "56D":
			msg.IntermediaryInstitution = value
		case "57A", "57B", "57C", "57D":
			msg.AccountWithInstitution = value
		case "58A", "58D":
			msg.BeneficiaryInstitution = value
		case "72":
			msg.SenderToReceiverInfo = value
		}
	}

	return nil
}

func (p *Parser) parseField32AMT202(value string, msg *MT202Message) error {
	if len(value) < 9 {
		return ErrInvalidFieldFormat
	}

	dateStr := value[0:6]
	date, err := time.Parse("060102", dateStr)
	if err != nil {
		return fmt.Errorf("invalid date format: %w", err)
	}
	msg.ValueDate = date

	msg.Currency = value[6:9]

	amountStr := value[9:]
	amountStr = strings.ReplaceAll(amountStr, ",", ".")
	amount, err := decimal.NewFromString(amountStr)
	if err != nil {
		return fmt.Errorf("invalid amount format: %w", err)
	}
	msg.Amount = amount

	return nil
}

func (p *Parser) validateMT202(msg *MT202Message) error {
	if msg.SenderReference == "" {
		return fmt.Errorf("%w: field 20 (Sender's Reference)", ErrMissingField)
	}
	if msg.ValueDate.IsZero() {
		return fmt.Errorf("%w: field 32A (Value Date)", ErrMissingField)
	}
	if msg.Currency == "" {
		return fmt.Errorf("%w: field 32A (Currency)", ErrMissingField)
	}
	if msg.Amount.IsZero() {
		return fmt.Errorf("%w: field 32A (Amount)", ErrMissingField)
	}
	if msg.OrderingInstitution == "" {
		return fmt.Errorf("%w: field 52 (Ordering Institution)", ErrMissingField)
	}
	if msg.BeneficiaryInstitution == "" {
		return fmt.Errorf("%w: field 58 (Beneficiary Institution)", ErrMissingField)
	}

	return nil
}

// FormatAmount formats a decimal amount for SWIFT (12,2 format with comma)
func FormatAmount(amount decimal.Decimal) string {
	return strings.ReplaceAll(amount.StringFixed(2), ".", ",")
}

// FormatDate formats a date for SWIFT (YYMMDD)
func FormatDate(date time.Time) string {
	return date.Format("060102")
}

// ValidateBIC validates a BIC code
func ValidateBIC(bic string) bool {
	// BIC format: 4 chars (bank) + 2 chars (country) + 2 chars (location) + [3 chars (branch)]
	if len(bic) != 8 && len(bic) != 11 {
		return false
	}

	// Convert to uppercase for validation
	bic = strings.ToUpper(bic)
	bicRegex := regexp.MustCompile(`^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$`)
	return bicRegex.MatchString(bic)
}
