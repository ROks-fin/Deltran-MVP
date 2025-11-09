package clients

import (
	"context"
	"time"
)

// LiquidityClient handles communication with Liquidity Router
type LiquidityClient struct {
	*BaseClient
}

// NewLiquidityClient creates a new liquidity client
func NewLiquidityClient(baseURL string) *LiquidityClient {
	return &LiquidityClient{
		BaseClient: NewBaseClient(baseURL, "liquidity-router", 5*time.Second),
	}
}

// LiquidityPredictionRequest represents a liquidity prediction request
type LiquidityPredictionRequest struct {
	Corridor     string  `json:"corridor"`
	Amount       float64 `json:"amount"`
	FromCurrency string  `json:"from_currency"`
	ToCurrency   string  `json:"to_currency"`
	TimeHorizon  int64   `json:"time_horizon,omitempty"` // seconds
}

// LiquidityPredictionResponse represents a liquidity prediction response
type LiquidityPredictionResponse struct {
	Available         bool    `json:"available"`
	InstantSettlement bool    `json:"instant_settlement"`
	Confidence        float64 `json:"confidence"`
	EstimatedTime     string  `json:"estimated_time"`
	RecommendedPath   string  `json:"recommended_path,omitempty"`
	CheckedAt         string  `json:"checked_at"`
}

// PredictLiquidity performs liquidity prediction
func (c *LiquidityClient) PredictLiquidity(ctx context.Context, req LiquidityPredictionRequest) (*LiquidityPredictionResponse, error) {
	var result LiquidityPredictionResponse
	err := c.Post(ctx, "/api/v1/liquidity/predict", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
