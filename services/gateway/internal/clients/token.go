package clients

import (
	"context"
	"time"
)

// TokenClient handles communication with Token Engine
type TokenClient struct {
	*BaseClient
}

// NewTokenClient creates a new token client
func NewTokenClient(baseURL string) *TokenClient {
	return &TokenClient{
		BaseClient: NewBaseClient(baseURL, "token-engine", 5*time.Second),
	}
}

// MintTokenRequest represents a token minting request
type MintTokenRequest struct {
	BankID   string  `json:"bank_id"`
	Currency string  `json:"currency"`
	Amount   float64 `json:"amount"`
	Reference string `json:"reference,omitempty"`
}

// MintTokenResponse represents a token minting response
type MintTokenResponse struct {
	TokenID   string  `json:"token_id"`
	BankID    string  `json:"bank_id"`
	Currency  string  `json:"currency"`
	Amount    float64 `json:"amount"`
	Status    string  `json:"status"`
	CreatedAt string  `json:"created_at"`
}

// TransferTokenRequest represents a token transfer request
type TransferTokenRequest struct {
	FromBank  string  `json:"from_bank"`
	ToBank    string  `json:"to_bank"`
	Currency  string  `json:"currency"`
	Amount    float64 `json:"amount"`
	Reference string  `json:"reference,omitempty"`
}

// TransferTokenResponse represents a token transfer response
type TransferTokenResponse struct {
	TransferID string  `json:"transfer_id"`
	FromBank   string  `json:"from_bank"`
	ToBank     string  `json:"to_bank"`
	Currency   string  `json:"currency"`
	Amount     float64 `json:"amount"`
	Status     string  `json:"status"`
	CreatedAt  string  `json:"created_at"`
}

// MintToken mints new tokens
func (c *TokenClient) MintToken(ctx context.Context, req MintTokenRequest) (*MintTokenResponse, error) {
	var result MintTokenResponse
	err := c.Post(ctx, "/api/v1/tokens/mint", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}

// TransferToken transfers tokens between banks
func (c *TokenClient) TransferToken(ctx context.Context, req TransferTokenRequest) (*TransferTokenResponse, error) {
	var result TransferTokenResponse
	err := c.Post(ctx, "/api/v1/tokens/transfer", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
