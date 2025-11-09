package models

import "time"

// TransferRequest represents incoming transfer request
type TransferRequest struct {
	SenderBank      string  `json:"sender_bank" binding:"required"`
	ReceiverBank    string  `json:"receiver_bank" binding:"required"`
	Amount          float64 `json:"amount" binding:"required,gt=0"`
	FromCurrency    string  `json:"from_currency" binding:"required"`
	ToCurrency      string  `json:"to_currency" binding:"required"`
	Reference       string  `json:"reference"`
	SenderAccount   string  `json:"sender_account" binding:"required"`
	ReceiverAccount string  `json:"receiver_account" binding:"required"`
	IdempotencyKey  string  `json:"idempotency_key"`
}

// TransferResponse represents the response for a transfer request
type TransferResponse struct {
	TransactionID     string    `json:"transaction_id"`
	Status            string    `json:"status"`
	Message           string    `json:"message"`
	InstantSettlement bool      `json:"instant_settlement"`
	EstimatedTime     string    `json:"estimated_time"`
	CreatedAt         time.Time `json:"created_at"`
	ComplianceCheck   *ComplianceResult `json:"compliance_check,omitempty"`
	RiskScore         *RiskResult       `json:"risk_score,omitempty"`
}

// TransactionStatus represents transaction status
type TransactionStatus string

const (
	StatusPending    TransactionStatus = "PENDING"
	StatusProcessing TransactionStatus = "PROCESSING"
	StatusCompleted  TransactionStatus = "COMPLETED"
	StatusFailed     TransactionStatus = "FAILED"
	StatusBlocked    TransactionStatus = "BLOCKED"
)

// ComplianceResult represents compliance check result
type ComplianceResult struct {
	Passed         bool      `json:"passed"`
	SanctionsCheck bool      `json:"sanctions_check"`
	AMLCheck       bool      `json:"aml_check"`
	Alerts         []string  `json:"alerts,omitempty"`
	CheckedAt      time.Time `json:"checked_at"`
}

// RiskResult represents risk evaluation result
type RiskResult struct {
	Score      float64 `json:"score"`
	Level      string  `json:"level"` // LOW, MEDIUM, HIGH
	Approved   bool    `json:"approved"`
	Reasons    []string `json:"reasons,omitempty"`
	EvaluatedAt time.Time `json:"evaluated_at"`
}

// LiquidityResult represents liquidity check result
type LiquidityResult struct {
	Available         bool      `json:"available"`
	InstantSettlement bool      `json:"instant_settlement"`
	Confidence        float64   `json:"confidence"`
	EstimatedTime     string    `json:"estimated_time"`
	CheckedAt         time.Time `json:"checked_at"`
}

// ObligationResult represents obligation creation result
type ObligationResult struct {
	ObligationID string    `json:"obligation_id"`
	Status       string    `json:"status"`
	ClearingWindow int64   `json:"clearing_window"`
	CreatedAt    time.Time `json:"created_at"`
}

// TokenResult represents token minting result
type TokenResult struct {
	TokenID   string    `json:"token_id"`
	Amount    float64   `json:"amount"`
	Currency  string    `json:"currency"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
}

// ErrorResponse represents error response
type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
	Code    int    `json:"code"`
}

// HealthResponse represents health check response
type HealthResponse struct {
	Status  string `json:"status"`
	Service string `json:"service"`
	Version string `json:"version"`
	Uptime  string `json:"uptime"`
}

// Bank represents a bank entity
type Bank struct {
	ID      string `json:"id"`
	Code    string `json:"code"`
	Name    string `json:"name"`
	Country string `json:"country"`
	Active  bool   `json:"active"`
}

// Corridor represents a trading corridor
type Corridor struct {
	ID                string  `json:"id"`
	Name              string  `json:"name"`
	FromCurrency      string  `json:"from_currency"`
	ToCurrency        string  `json:"to_currency"`
	Active            bool    `json:"active"`
	InstantSettlement bool    `json:"instant_settlement"`
	MaxAmount         float64 `json:"max_amount"`
}

// FXRate represents foreign exchange rate
type FXRate struct {
	Corridor  string    `json:"corridor"`
	Rate      float64   `json:"rate"`
	Spread    float64   `json:"spread"`
	UpdatedAt time.Time `json:"updated_at"`
}

// Transaction represents a transaction
type Transaction struct {
	ID                string            `json:"id"`
	SenderBank        string            `json:"sender_bank"`
	ReceiverBank      string            `json:"receiver_bank"`
	Amount            float64           `json:"amount"`
	FromCurrency      string            `json:"from_currency"`
	ToCurrency        string            `json:"to_currency"`
	FXRate            float64           `json:"fx_rate"`
	Status            TransactionStatus `json:"status"`
	InstantSettlement bool              `json:"instant_settlement"`
	CreatedAt         time.Time         `json:"created_at"`
	CompletedAt       *time.Time        `json:"completed_at,omitempty"`
}
