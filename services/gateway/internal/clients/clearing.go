package clients

import (
	"context"
	"time"
)

// ClearingClient handles communication with Clearing Engine
// For MVP, using HTTP instead of gRPC
type ClearingClient struct {
	*BaseClient
}

// NewClearingClient creates a new clearing client
func NewClearingClient(baseURL string) *ClearingClient {
	return &ClearingClient{
		BaseClient: NewBaseClient(baseURL, "clearing-engine", 5*time.Second),
	}
}

// WindowStatusRequest represents a window status request
type WindowStatusRequest struct {
	WindowID int64 `json:"window_id"`
}

// WindowStatusResponse represents a window status response
type WindowStatusResponse struct {
	WindowID         int64   `json:"window_id"`
	Status           string  `json:"status"`
	ObligationsCount int     `json:"obligations_count"`
	TotalAmount      float64 `json:"total_amount"`
	OpenedAt         string  `json:"opened_at"`
	ClosedAt         string  `json:"closed_at,omitempty"`
}

// GetWindowStatus retrieves current clearing window status
func (c *ClearingClient) GetWindowStatus(ctx context.Context, windowID int64) (*WindowStatusResponse, error) {
	var result WindowStatusResponse
	err := c.Post(ctx, "/api/v1/clearing/window/status", WindowStatusRequest{WindowID: windowID}, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}

// GetCurrentWindow retrieves current clearing window
func (c *ClearingClient) GetCurrentWindow(ctx context.Context) (*WindowStatusResponse, error) {
	var result WindowStatusResponse
	err := c.Get(ctx, "/api/v1/clearing/window/current", &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
