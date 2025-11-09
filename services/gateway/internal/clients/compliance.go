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
type ComplianceCheckRequest struct {
	TransactionID   string  `json:"transaction_id"`
	SenderBank      string  `json:"sender_bank"`
	ReceiverBank    string  `json:"receiver_bank"`
	Amount          float64 `json:"amount"`
	Currency        string  `json:"currency"`
	SenderName      string  `json:"sender_name,omitempty"`
	ReceiverName    string  `json:"receiver_name,omitempty"`
	SenderCountry   string  `json:"sender_country,omitempty"`
	ReceiverCountry string  `json:"receiver_country,omitempty"`
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
