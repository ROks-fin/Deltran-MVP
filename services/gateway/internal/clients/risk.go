package clients

import (
	"context"
	"time"
)

// RiskClient handles communication with Risk Engine
type RiskClient struct {
	*BaseClient
}

// NewRiskClient creates a new risk client
func NewRiskClient(baseURL string) *RiskClient {
	return &RiskClient{
		BaseClient: NewBaseClient(baseURL, "risk-engine", 5*time.Second),
	}
}

// RiskEvaluationRequest represents a risk evaluation request
type RiskEvaluationRequest struct {
	TransactionID string  `json:"transaction_id"`
	BankID        string  `json:"bank_id"`
	Amount        float64 `json:"amount"`
	Currency      string  `json:"currency"`
	Corridor      string  `json:"corridor"`
	TransactionType string `json:"transaction_type,omitempty"`
}

// RiskEvaluationResponse represents a risk evaluation response
type RiskEvaluationResponse struct {
	TransactionID string   `json:"transaction_id"`
	Score         float64  `json:"score"`
	Level         string   `json:"level"` // LOW, MEDIUM, HIGH
	Approved      bool     `json:"approved"`
	Reasons       []string `json:"reasons,omitempty"`
	Factors       map[string]float64 `json:"factors,omitempty"`
	EvaluatedAt   string   `json:"evaluated_at"`
}

// EvaluateRisk performs risk evaluation
func (c *RiskClient) EvaluateRisk(ctx context.Context, req RiskEvaluationRequest) (*RiskEvaluationResponse, error) {
	var result RiskEvaluationResponse
	err := c.Post(ctx, "/api/v1/risk/evaluate", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
