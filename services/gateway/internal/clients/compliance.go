package clients

import (
	"context"
	"time"
)

// ComplianceClient handles communication with Compliance Engine
type ComplianceClient struct {
	*BaseClient
}

// NewComplianceClient creates a new compliance client
func NewComplianceClient(baseURL string) *ComplianceClient {
	return &ComplianceClient{
		BaseClient: NewBaseClient(baseURL, "compliance-engine", 5*time.Second),
	}
}

// ComplianceCheckRequest represents a compliance check request
// This matches the Rust backend ComplianceCheckRequest structure
type ComplianceCheckRequest struct {
	TransactionID    string  `json:"transaction_id"`      // UUID string
	SenderName       string  `json:"sender_name"`
	SenderAccount    string  `json:"sender_account"`
	SenderCountry    string  `json:"sender_country"`
	SenderBankID     string  `json:"sender_bank_id"`      // UUID string
	ReceiverName     string  `json:"receiver_name"`
	ReceiverAccount  string  `json:"receiver_account"`
	ReceiverCountry  string  `json:"receiver_country"`
	ReceiverBankID   string  `json:"receiver_bank_id"`    // UUID string
	Amount           string  `json:"amount"`              // Decimal as string
	Currency         string  `json:"currency"`
	Purpose          string  `json:"purpose,omitempty"`
	Metadata         interface{} `json:"metadata,omitempty"`
}

// ComplianceCheckResponse represents a compliance check response
type ComplianceCheckResponse struct {
	TransactionID  string   `json:"transaction_id"`
	Passed         bool     `json:"passed"`
	SanctionsCheck bool     `json:"sanctions_check"`
	AMLCheck       bool     `json:"aml_check"`
	PEPCheck       bool     `json:"pep_check"`
	Alerts         []string `json:"alerts,omitempty"`
	RiskLevel      string   `json:"risk_level"`
	CheckedAt      string   `json:"checked_at"`
}

// CheckCompliance performs compliance check
func (c *ComplianceClient) CheckCompliance(ctx context.Context, req ComplianceCheckRequest) (*ComplianceCheckResponse, error) {
	var result ComplianceCheckResponse
	err := c.Post(ctx, "/api/v1/compliance/check", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
