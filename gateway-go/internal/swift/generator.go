package swift

import (
	"crypto/rand"
	"encoding/hex"
	"fmt"
	"strings"
	"time"

	"github.com/shopspring/decimal"
)

// Generator generates SWIFT messages
type Generator struct {
	senderBIC string
}

// NewGenerator creates a new SWIFT message generator
func NewGenerator(senderBIC string) *Generator {
	return &Generator{
		senderBIC: senderBIC,
	}
}

// MT103Builder builds MT103 messages
type MT103Builder struct {
	msg *MT103Message
}

// NewMT103Builder creates a new MT103 builder
func NewMT103Builder() *MT103Builder {
	return &MT103Builder{
		msg: &MT103Message{
			MessageType:     "103",
			MessagePriority: "N", // Normal priority by default
		},
	}
}

// SetSender sets sender information
func (b *MT103Builder) SetSender(bic string) *MT103Builder {
	b.msg.SenderBIC = bic
	return b
}

// SetReceiver sets receiver information
func (b *MT103Builder) SetReceiver(bic string) *MT103Builder {
	b.msg.ReceiverBIC = bic
	return b
}

// SetReference sets transaction reference (:20:)
func (b *MT103Builder) SetReference(ref string) *MT103Builder {
	b.msg.SenderReference = ref
	return b
}

// SetValueDateCurrencyAmount sets :32A: field
func (b *MT103Builder) SetValueDateCurrencyAmount(date time.Time, currency string, amount decimal.Decimal) *MT103Builder {
	b.msg.ValueDate = date
	b.msg.Currency = currency
	b.msg.Amount = amount
	return b
}

// SetOrderingCustomer sets :50K: field
func (b *MT103Builder) SetOrderingCustomer(customer string) *MT103Builder {
	b.msg.OrderingCustomer = customer
	return b
}

// SetBeneficiary sets :59: field
func (b *MT103Builder) SetBeneficiary(account, name string) *MT103Builder {
	b.msg.BeneficiaryAccount = account
	b.msg.BeneficiaryCustomer = name
	return b
}

// SetRemittanceInfo sets :70: field
func (b *MT103Builder) SetRemittanceInfo(info string) *MT103Builder {
	b.msg.RemittanceInfo = info
	return b
}

// SetCharges sets :71A: field (OUR/BEN/SHA)
func (b *MT103Builder) SetCharges(charges string) *MT103Builder {
	b.msg.DetailsOfCharges = charges
	return b
}

// SetOrderingInstitution sets :52A/D: field
func (b *MT103Builder) SetOrderingInstitution(institution string) *MT103Builder {
	b.msg.OrderingInstitution = institution
	return b
}

// SetAccountWithInstitution sets :57A: field
func (b *MT103Builder) SetAccountWithInstitution(institution string) *MT103Builder {
	b.msg.AccountWithInstitution = institution
	return b
}

// SetPriority sets message priority (U/N/S)
func (b *MT103Builder) SetPriority(priority string) *MT103Builder {
	b.msg.MessagePriority = priority
	return b
}

// Build builds the MT103 message
func (b *MT103Builder) Build() (*MT103Message, error) {
	// Validate mandatory fields
	if b.msg.SenderBIC == "" {
		return nil, fmt.Errorf("sender BIC is required")
	}
	if b.msg.ReceiverBIC == "" {
		return nil, fmt.Errorf("receiver BIC is required")
	}
	if b.msg.SenderReference == "" {
		return nil, fmt.Errorf("sender reference is required")
	}
	if b.msg.ValueDate.IsZero() {
		return nil, fmt.Errorf("value date is required")
	}
	if b.msg.Currency == "" {
		return nil, fmt.Errorf("currency is required")
	}
	if b.msg.Amount.IsZero() {
		return nil, fmt.Errorf("amount is required")
	}
	if b.msg.OrderingCustomer == "" {
		return nil, fmt.Errorf("ordering customer is required")
	}
	if b.msg.BeneficiaryCustomer == "" && b.msg.BeneficiaryAccount == "" {
		return nil, fmt.Errorf("beneficiary is required")
	}

	// Set defaults
	if b.msg.ApplicationID == "" {
		b.msg.ApplicationID = "F"
	}
	if b.msg.ServiceID == "" {
		b.msg.ServiceID = "01"
	}
	if b.msg.DetailsOfCharges == "" {
		b.msg.DetailsOfCharges = "SHA" // Shared charges by default
	}

	return b.msg, nil
}

// MT202Builder builds MT202 messages
type MT202Builder struct {
	msg *MT202Message
}

// NewMT202Builder creates a new MT202 builder
func NewMT202Builder() *MT202Builder {
	return &MT202Builder{
		msg: &MT202Message{
			MessageType:     "202",
			MessagePriority: "N",
		},
	}
}

// SetSender sets sender information
func (b *MT202Builder) SetSender(bic string) *MT202Builder {
	b.msg.SenderBIC = bic
	return b
}

// SetReceiver sets receiver information
func (b *MT202Builder) SetReceiver(bic string) *MT202Builder {
	b.msg.ReceiverBIC = bic
	return b
}

// SetReference sets transaction reference (:20:)
func (b *MT202Builder) SetReference(ref string) *MT202Builder {
	b.msg.SenderReference = ref
	return b
}

// SetRelatedReference sets related reference (:21:)
func (b *MT202Builder) SetRelatedReference(ref string) *MT202Builder {
	b.msg.RelatedReference = ref
	return b
}

// SetValueDateCurrencyAmount sets :32A: field
func (b *MT202Builder) SetValueDateCurrencyAmount(date time.Time, currency string, amount decimal.Decimal) *MT202Builder {
	b.msg.ValueDate = date
	b.msg.Currency = currency
	b.msg.Amount = amount
	return b
}

// SetOrderingInstitution sets :52A/D: field
func (b *MT202Builder) SetOrderingInstitution(institution string) *MT202Builder {
	b.msg.OrderingInstitution = institution
	return b
}

// SetBeneficiaryInstitution sets :58A/D: field
func (b *MT202Builder) SetBeneficiaryInstitution(institution string) *MT202Builder {
	b.msg.BeneficiaryInstitution = institution
	return b
}

// SetSendersCorrespondent sets :53A/B/D: field
func (b *MT202Builder) SetSendersCorrespondent(correspondent string) *MT202Builder {
	b.msg.SendersCorrespondent = correspondent
	return b
}

// SetAccountWithInstitution sets :57A: field
func (b *MT202Builder) SetAccountWithInstitution(institution string) *MT202Builder {
	b.msg.AccountWithInstitution = institution
	return b
}

// Build builds the MT202 message
func (b *MT202Builder) Build() (*MT202Message, error) {
	// Validate mandatory fields
	if b.msg.SenderBIC == "" {
		return nil, fmt.Errorf("sender BIC is required")
	}
	if b.msg.ReceiverBIC == "" {
		return nil, fmt.Errorf("receiver BIC is required")
	}
	if b.msg.SenderReference == "" {
		return nil, fmt.Errorf("sender reference is required")
	}
	if b.msg.ValueDate.IsZero() {
		return nil, fmt.Errorf("value date is required")
	}
	if b.msg.Currency == "" {
		return nil, fmt.Errorf("currency is required")
	}
	if b.msg.Amount.IsZero() {
		return nil, fmt.Errorf("amount is required")
	}
	if b.msg.OrderingInstitution == "" {
		return nil, fmt.Errorf("ordering institution is required")
	}
	if b.msg.BeneficiaryInstitution == "" {
		return nil, fmt.Errorf("beneficiary institution is required")
	}

	// Set defaults
	if b.msg.ApplicationID == "" {
		b.msg.ApplicationID = "F"
	}
	if b.msg.ServiceID == "" {
		b.msg.ServiceID = "01"
	}

	return b.msg, nil
}

// GenerateMT103 generates a SWIFT MT103 message string
func (g *Generator) GenerateMT103(msg *MT103Message) (string, error) {
	// Validate mandatory fields
	if msg.SenderBIC == "" {
		return "", fmt.Errorf("sender BIC is required")
	}
	if msg.ReceiverBIC == "" {
		return "", fmt.Errorf("receiver BIC is required")
	}
	if msg.SenderReference == "" {
		return "", fmt.Errorf("sender reference (:20:) is required")
	}
	if msg.ValueDate.IsZero() {
		return "", fmt.Errorf("value date (:32A:) is required")
	}
	if msg.Currency == "" {
		return "", fmt.Errorf("currency (:32A:) is required")
	}
	if msg.Amount.IsZero() {
		return "", fmt.Errorf("amount (:32A:) is required")
	}
	if msg.OrderingCustomer == "" {
		return "", fmt.Errorf("ordering customer (:50K:) is required")
	}
	if msg.BeneficiaryCustomer == "" && msg.BeneficiaryAccount == "" {
		return "", fmt.Errorf("beneficiary (:59:) is required")
	}

	var sb strings.Builder

	// Block 1: Basic Header
	// Set session and sequence numbers if not provided
	sessionNum := msg.SessionNumber
	if sessionNum == "" {
		sessionNum = "0000"
	}
	seqNum := msg.SequenceNumber
	if seqNum == "" {
		seqNum = "000000"
	}

	// Set defaults
	appID := msg.ApplicationID
	if appID == "" {
		appID = "F"
	}
	svcID := msg.ServiceID
	if svcID == "" {
		svcID = "01"
	}

	sb.WriteString(fmt.Sprintf("{1:%s%s%s%s%s}",
		appID,
		svcID,
		padBIC(msg.SenderBIC),
		sessionNum,
		seqNum,
	))

	// Set message type default
	msgType := msg.MessageType
	if msgType == "" {
		msgType = "103"
	}

	// Block 2: Application Header
	sb.WriteString(fmt.Sprintf("{2:I%s%sN}",
		msgType,
		padBIC(msg.ReceiverBIC),
	))

	// Block 3: User Header (optional)
	if msg.ValidationFlag != "" || msg.ServiceTypeID != "" {
		sb.WriteString("{3:")
		if msg.ValidationFlag != "" {
			sb.WriteString(fmt.Sprintf("{119:%s}", msg.ValidationFlag))
		}
		if msg.ServiceTypeID != "" {
			sb.WriteString(fmt.Sprintf("{103:%s}", msg.ServiceTypeID))
		}
		sb.WriteString("}")
	}

	// Block 4: Text Block
	sb.WriteString("{4:\n")

	// :20: Sender's Reference (mandatory)
	sb.WriteString(fmt.Sprintf(":20:%s\n", msg.SenderReference))

	// :32A: Value Date, Currency, Amount (mandatory)
	sb.WriteString(fmt.Sprintf(":32A:%s%s%s\n",
		FormatDate(msg.ValueDate),
		msg.Currency,
		FormatAmount(msg.Amount),
	))

	// :50K: Ordering Customer (mandatory)
	sb.WriteString(fmt.Sprintf(":50K:%s\n", formatMultiline(msg.OrderingCustomer)))

	// :52A/D: Ordering Institution (optional)
	if msg.OrderingInstitution != "" {
		sb.WriteString(fmt.Sprintf(":52A:%s\n", msg.OrderingInstitution))
	}

	// :53A/B/D: Sender's Correspondent (optional)
	if msg.SendersCorrespondent != "" {
		sb.WriteString(fmt.Sprintf(":53A:%s\n", msg.SendersCorrespondent))
	}

	// :54A/B/D: Receiver's Correspondent (optional)
	if msg.ReceiversCorrespondent != "" {
		sb.WriteString(fmt.Sprintf(":54A:%s\n", msg.ReceiversCorrespondent))
	}

	// :56A/C/D: Intermediary (optional)
	if msg.IntermediaryInstitution != "" {
		sb.WriteString(fmt.Sprintf(":56A:%s\n", msg.IntermediaryInstitution))
	}

	// :57A/B/C/D: Account With Institution (optional)
	if msg.AccountWithInstitution != "" {
		sb.WriteString(fmt.Sprintf(":57A:%s\n", msg.AccountWithInstitution))
	}

	// :59: Beneficiary (mandatory)
	sb.WriteString(":59:")
	if msg.BeneficiaryAccount != "" {
		sb.WriteString(fmt.Sprintf("/%s\n", msg.BeneficiaryAccount))
	}
	sb.WriteString(fmt.Sprintf("%s\n", formatMultiline(msg.BeneficiaryCustomer)))

	// :70: Remittance Information (optional)
	if msg.RemittanceInfo != "" {
		sb.WriteString(fmt.Sprintf(":70:%s\n", formatMultiline(msg.RemittanceInfo)))
	}

	// :71A: Details of Charges (optional)
	if msg.DetailsOfCharges != "" {
		sb.WriteString(fmt.Sprintf(":71A:%s\n", msg.DetailsOfCharges))
	}

	// :72: Sender to Receiver Information (optional)
	if msg.SenderToReceiverInfo != "" {
		sb.WriteString(fmt.Sprintf(":72:%s\n", formatMultiline(msg.SenderToReceiverInfo)))
	}

	sb.WriteString("-}")

	// Block 5: Trailers (optional)
	// Not implemented for now

	return sb.String(), nil
}

// GenerateMT202 generates a SWIFT MT202 message string
func (g *Generator) GenerateMT202(msg *MT202Message) (string, error) {
	// Validate mandatory fields
	if msg.SenderBIC == "" {
		return "", fmt.Errorf("sender BIC is required")
	}
	if msg.ReceiverBIC == "" {
		return "", fmt.Errorf("receiver BIC is required")
	}
	if msg.SenderReference == "" {
		return "", fmt.Errorf("sender reference (:20:) is required")
	}
	if msg.ValueDate.IsZero() {
		return "", fmt.Errorf("value date (:32A:) is required")
	}
	if msg.Currency == "" {
		return "", fmt.Errorf("currency (:32A:) is required")
	}
	if msg.Amount.IsZero() {
		return "", fmt.Errorf("amount (:32A:) is required")
	}
	if msg.OrderingInstitution == "" {
		return "", fmt.Errorf("ordering institution (:52A:) is required")
	}
	if msg.BeneficiaryInstitution == "" {
		return "", fmt.Errorf("beneficiary institution (:58A:) is required")
	}

	var sb strings.Builder

	// Block 1: Basic Header
	// Set session and sequence numbers if not provided
	sessionNum := msg.SessionNumber
	if sessionNum == "" {
		sessionNum = "0000"
	}
	seqNum := msg.SequenceNumber
	if seqNum == "" {
		seqNum = "000000"
	}

	// Set defaults
	appID := msg.ApplicationID
	if appID == "" {
		appID = "F"
	}
	svcID := msg.ServiceID
	if svcID == "" {
		svcID = "01"
	}

	sb.WriteString(fmt.Sprintf("{1:%s%s%s%s%s}",
		appID,
		svcID,
		padBIC(msg.SenderBIC),
		sessionNum,
		seqNum,
	))

	// Set message type default
	msgType := msg.MessageType
	if msgType == "" {
		msgType = "202"
	}

	// Block 2: Application Header
	sb.WriteString(fmt.Sprintf("{2:I%s%sN}",
		msgType,
		padBIC(msg.ReceiverBIC),
	))

	// Block 3: User Header (optional)
	if msg.ValidationFlag != "" || msg.ServiceTypeID != "" {
		sb.WriteString("{3:")
		if msg.ValidationFlag != "" {
			sb.WriteString(fmt.Sprintf("{119:%s}", msg.ValidationFlag))
		}
		if msg.ServiceTypeID != "" {
			sb.WriteString(fmt.Sprintf("{103:%s}", msg.ServiceTypeID))
		}
		sb.WriteString("}")
	}

	// Block 4: Text Block
	sb.WriteString("{4:\n")

	// :20: Sender's Reference (mandatory)
	sb.WriteString(fmt.Sprintf(":20:%s\n", msg.SenderReference))

	// :21: Related Reference (mandatory)
	sb.WriteString(fmt.Sprintf(":21:%s\n", msg.RelatedReference))

	// :32A: Value Date, Currency, Amount (mandatory)
	sb.WriteString(fmt.Sprintf(":32A:%s%s%s\n",
		FormatDate(msg.ValueDate),
		msg.Currency,
		FormatAmount(msg.Amount),
	))

	// :52A/D: Ordering Institution (mandatory)
	sb.WriteString(fmt.Sprintf(":52A:%s\n", msg.OrderingInstitution))

	// :53A/B/D: Sender's Correspondent (optional)
	if msg.SendersCorrespondent != "" {
		sb.WriteString(fmt.Sprintf(":53A:%s\n", msg.SendersCorrespondent))
	}

	// :54A/B/D: Receiver's Correspondent (optional)
	if msg.ReceiversCorrespondent != "" {
		sb.WriteString(fmt.Sprintf(":54A:%s\n", msg.ReceiversCorrespondent))
	}

	// :56A/C/D: Intermediary (optional)
	if msg.IntermediaryInstitution != "" {
		sb.WriteString(fmt.Sprintf(":56A:%s\n", msg.IntermediaryInstitution))
	}

	// :57A/B/C/D: Account With Institution (optional)
	if msg.AccountWithInstitution != "" {
		sb.WriteString(fmt.Sprintf(":57A:%s\n", msg.AccountWithInstitution))
	}

	// :58A/D: Beneficiary Institution (mandatory)
	sb.WriteString(fmt.Sprintf(":58A:%s\n", msg.BeneficiaryInstitution))

	// :72: Sender to Receiver Information (optional)
	if msg.SenderToReceiverInfo != "" {
		sb.WriteString(fmt.Sprintf(":72:%s\n", formatMultiline(msg.SenderToReceiverInfo)))
	}

	sb.WriteString("-}")

	return sb.String(), nil
}

// padBIC pads BIC to 12 characters with X
func padBIC(bic string) string {
	if len(bic) == 8 {
		return bic + "XXXX" // Pad to 12 chars
	}
	if len(bic) == 11 {
		return bic + "X" // Pad to 12 chars
	}
	if len(bic) == 12 {
		return bic
	}
	// If already 12 or longer, just return as is
	return bic
}

// formatMultiline formats multiline text for SWIFT (max 35 chars per line, max 4 lines)
func formatMultiline(text string) string {
	if text == "" {
		return ""
	}

	// Split into lines if newlines present
	lines := strings.Split(text, "\n")
	result := make([]string, 0, len(lines))

	for _, line := range lines {
		// Trim and limit to 35 characters
		line = strings.TrimSpace(line)
		if len(line) > 35 {
			line = line[:35]
		}
		if line != "" {
			result = append(result, line)
		}
		// Limit to 4 lines
		if len(result) >= 4 {
			break
		}
	}

	return strings.Join(result, "\n")
}

// GenerateReference generates a unique transaction reference
func GenerateReference(prefix string) string {
	// Use combination of timestamp and random bytes for uniqueness
	timestamp := time.Now().UnixNano()

	// Add 4 random bytes for additional uniqueness
	randomBytes := make([]byte, 2)
	rand.Read(randomBytes)
	randomHex := hex.EncodeToString(randomBytes)

	return fmt.Sprintf("%s%d%s", prefix, timestamp, randomHex)
}
