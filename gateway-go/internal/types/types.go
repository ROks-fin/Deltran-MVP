// Domain types for gateway
package types

import (
	"time"

	"github.com/google/uuid"
	"github.com/shopspring/decimal"
)

// Payment represents a payment request
type Payment struct {
	PaymentID       uuid.UUID       `json:"payment_id"`
	UETR            string          `json:"uetr"` // Unique End-to-end Transaction Reference
	Amount          decimal.Decimal `json:"amount"`
	Currency        string          `json:"currency"`
	DebtorBank      string          `json:"debtor_bank"`
	CreditorBank    string          `json:"creditor_bank"`
	DebtorAccount   string          `json:"debtor_account"`
	CreditorAccount string          `json:"creditor_account"`
	DebtorName      string          `json:"debtor_name"`
	CreditorName    string          `json:"creditor_name"`
	Reference       string          `json:"reference"`
	Status          PaymentStatus   `json:"status"`
	CreatedAt       time.Time       `json:"created_at"`
	UpdatedAt       time.Time       `json:"updated_at"`
}

// PaymentStatus represents payment lifecycle status
type PaymentStatus string

const (
	PaymentStatusInitiated           PaymentStatus = "INITIATED"
	PaymentStatusValidated           PaymentStatus = "VALIDATED"
	PaymentStatusScreened            PaymentStatus = "SCREENED"
	PaymentStatusApproved            PaymentStatus = "APPROVED"
	PaymentStatusQueued              PaymentStatus = "QUEUED"
	PaymentStatusSettling            PaymentStatus = "SETTLING"
	PaymentStatusSettled             PaymentStatus = "SETTLED"
	PaymentStatusCompleted           PaymentStatus = "COMPLETED"
	PaymentStatusRejected            PaymentStatus = "REJECTED"
	PaymentStatusFailed              PaymentStatus = "FAILED"
)

// EventType represents ledger event types
type EventType string

const (
	EventTypePaymentInitiated      EventType = "PAYMENT_INITIATED"
	EventTypeValidationPassed      EventType = "VALIDATION_PASSED"
	EventTypeValidationFailed      EventType = "VALIDATION_FAILED"
	EventTypeSanctionsCleared      EventType = "SANCTIONS_CLEARED"
	EventTypeSanctionsHit          EventType = "SANCTIONS_HIT"
	EventTypeRiskApproved          EventType = "RISK_APPROVED"
	EventTypeRiskRejected          EventType = "RISK_REJECTED"
	EventTypeQueuedForSettlement   EventType = "QUEUED_FOR_SETTLEMENT"
	EventTypeSettlementStarted     EventType = "SETTLEMENT_STARTED"
	EventTypeSettlementCompleted   EventType = "SETTLEMENT_COMPLETED"
	EventTypePaymentCompleted      EventType = "PAYMENT_COMPLETED"
	EventTypePaymentRejected       EventType = "PAYMENT_REJECTED"
	EventTypePaymentFailed         EventType = "PAYMENT_FAILED"
)

// ValidationResult represents validation check result
type ValidationResult struct {
	Valid   bool     `json:"valid"`
	Errors  []string `json:"errors,omitempty"`
	Warnings []string `json:"warnings,omitempty"`
}

// RiskAssessment represents risk check result
type RiskAssessment struct {
	Approved   bool            `json:"approved"`
	RiskScore  float64         `json:"risk_score"`
	RiskLevel  string          `json:"risk_level"` // LOW, MEDIUM, HIGH, CRITICAL
	Reasons    []string        `json:"reasons,omitempty"`
	Mitigations []string       `json:"mitigations,omitempty"`
}

// SanctionsCheck represents sanctions screening result
type SanctionsCheck struct {
	Cleared bool     `json:"cleared"`
	Hits    []string `json:"hits,omitempty"`
	Lists   []string `json:"lists,omitempty"` // OFAC, UN, EU, etc.
}

// ISO20022Message represents an ISO 20022 message
type ISO20022Message struct {
	MessageType string    `json:"message_type"` // pacs.008, pacs.002, etc.
	Content     string    `json:"content"`      // XML content
	Direction   string    `json:"direction"`    // INBOUND, OUTBOUND
	Timestamp   time.Time `json:"timestamp"`
}

// BankConnector represents a bank integration
type BankConnector interface {
	// SendPayment sends a payment to the bank
	SendPayment(payment *Payment) error

	// GetPaymentStatus queries payment status
	GetPaymentStatus(paymentID uuid.UUID) (PaymentStatus, error)

	// ParseIncoming parses incoming bank message
	ParseIncoming(message []byte) (*Payment, error)
}

// WorkerPool manages concurrent payment processing
type WorkerPool struct {
	size    int
	queue   chan *Payment
	workers []*Worker
}

// Worker processes payments
type Worker struct {
	id       int
	queue    chan *Payment
	shutdown chan struct{}
}

// Metrics represents gateway metrics
type Metrics struct {
	PaymentsTotal       int64   `json:"payments_total"`
	PaymentsSucceeded   int64   `json:"payments_succeeded"`
	PaymentsFailed      int64   `json:"payments_failed"`
	PaymentsRejected    int64   `json:"payments_rejected"`
	AverageLatencyMs    float64 `json:"average_latency_ms"`
	CurrentQueueDepth   int     `json:"current_queue_depth"`
	WorkersActive       int     `json:"workers_active"`
}

// Error types
type ErrorCode string

const (
	ErrorCodeValidation       ErrorCode = "VALIDATION_ERROR"
	ErrorCodeInsufficientFunds ErrorCode = "INSUFFICIENT_FUNDS"
	ErrorCodeSanctionsHit     ErrorCode = "SANCTIONS_HIT"
	ErrorCodeRiskRejected     ErrorCode = "RISK_REJECTED"
	ErrorCodeBankUnavailable  ErrorCode = "BANK_UNAVAILABLE"
	ErrorCodeTimeout          ErrorCode = "TIMEOUT"
	ErrorCodeInternal         ErrorCode = "INTERNAL_ERROR"
)

// GatewayError represents a gateway error
type GatewayError struct {
	Code    ErrorCode `json:"code"`
	Message string    `json:"message"`
	Details string    `json:"details,omitempty"`
}

func (e *GatewayError) Error() string {
	if e.Details != "" {
		return string(e.Code) + ": " + e.Message + " (" + e.Details + ")"
	}
	return string(e.Code) + ": " + e.Message
}