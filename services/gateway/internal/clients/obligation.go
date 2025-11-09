package clients

import (
	"context"
	"time"
)

// ObligationClient handles communication with Obligation Engine
type ObligationClient struct {
	*BaseClient
}

// NewObligationClient creates a new obligation client
func NewObligationClient(baseURL string) *ObligationClient {
	return &ObligationClient{
		BaseClient: NewBaseClient(baseURL, "obligation-engine", 5*time.Second),
	}
}

// CreateObligationRequest represents an obligation creation request
type CreateObligationRequest struct {
	TransactionID string  `json:"transaction_id"`
	PayerBank     string  `json:"payer_bank"`
	PayeeBank     string  `json:"payee_bank"`
	Amount        float64 `json:"amount"`
	Currency      string  `json:"currency"`
	Reference     string  `json:"reference,omitempty"`
}

// CreateObligationResponse represents an obligation creation response
type CreateObligationResponse struct {
	ObligationID   string  `json:"obligation_id"`
	TransactionID  string  `json:"transaction_id"`
	Status         string  `json:"status"`
	ClearingWindow int64   `json:"clearing_window"`
	NetAmount      float64 `json:"net_amount,omitempty"`
	CreatedAt      string  `json:"created_at"`
}

// CreateObligation creates a new obligation
func (c *ObligationClient) CreateObligation(ctx context.Context, req CreateObligationRequest) (*CreateObligationResponse, error) {
	var result CreateObligationResponse
	err := c.Post(ctx, "/api/v1/obligations/create", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
