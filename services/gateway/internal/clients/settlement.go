package clients

import (
	"context"
	"time"
)

// SettlementClient handles communication with Settlement Engine
// For MVP, using HTTP instead of gRPC
type SettlementClient struct {
	*BaseClient
}

// NewSettlementClient creates a new settlement client
func NewSettlementClient(baseURL string) *SettlementClient {
	return &SettlementClient{
		BaseClient: NewBaseClient(baseURL, "settlement-engine", 10*time.Second),
	}
}

// ExecuteSettlementRequest represents a settlement execution request
type ExecuteSettlementRequest struct {
	SettlementID string  `json:"settlement_id"`
	PayerBank    string  `json:"payer_bank"`
	PayeeBank    string  `json:"payee_bank"`
	Amount       float64 `json:"amount"`
	Currency     string  `json:"currency"`
	Reference    string  `json:"reference,omitempty"`
}

// ExecuteSettlementResponse represents a settlement execution response
type ExecuteSettlementResponse struct {
	SettlementID   string `json:"settlement_id"`
	Status         string `json:"status"`
	TransactionRef string `json:"transaction_ref"`
	CompletedAt    string `json:"completed_at,omitempty"`
}

// SettlementStatusRequest represents a settlement status request
type SettlementStatusRequest struct {
	SettlementID string `json:"settlement_id"`
}

// SettlementStatusResponse represents a settlement status response
type SettlementStatusResponse struct {
	SettlementID string  `json:"settlement_id"`
	Status       string  `json:"status"`
	Amount       float64 `json:"amount"`
	Currency     string  `json:"currency"`
	PayerBank    string  `json:"payer_bank"`
	PayeeBank    string  `json:"payee_bank"`
	CreatedAt    string  `json:"created_at"`
	CompletedAt  string  `json:"completed_at,omitempty"`
}

// ExecuteSettlement executes a settlement
func (c *SettlementClient) ExecuteSettlement(ctx context.Context, req ExecuteSettlementRequest) (*ExecuteSettlementResponse, error) {
	var result ExecuteSettlementResponse
	err := c.Post(ctx, "/api/v1/settlement/execute", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}

// GetSettlementStatus retrieves settlement status
func (c *SettlementClient) GetSettlementStatus(ctx context.Context, settlementID string) (*SettlementStatusResponse, error) {
	var result SettlementStatusResponse
	err := c.Post(ctx, "/api/v1/settlement/status", SettlementStatusRequest{SettlementID: settlementID}, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
