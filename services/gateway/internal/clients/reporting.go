package clients

import (
	"context"
	"time"
)

// ReportingClient handles communication with Reporting Engine
type ReportingClient struct {
	*BaseClient
}

// NewReportingClient creates a new reporting client
func NewReportingClient(baseURL string) *ReportingClient {
	return &ReportingClient{
		BaseClient: NewBaseClient(baseURL, "reporting-engine", 10*time.Second),
	}
}

// GenerateReportRequest represents a report generation request
type GenerateReportRequest struct {
	ReportType string                 `json:"report_type"` // transaction, settlement, aml, etc.
	Format     string                 `json:"format"`      // excel, csv, pdf
	StartDate  string                 `json:"start_date"`
	EndDate    string                 `json:"end_date"`
	Filters    map[string]interface{} `json:"filters,omitempty"`
}

// GenerateReportResponse represents a report generation response
type GenerateReportResponse struct {
	ReportID  string `json:"report_id"`
	Status    string `json:"status"`
	DownloadURL string `json:"download_url,omitempty"`
	CreatedAt string `json:"created_at"`
}

// GenerateReport generates a new report
func (c *ReportingClient) GenerateReport(ctx context.Context, req GenerateReportRequest) (*GenerateReportResponse, error) {
	var result GenerateReportResponse
	err := c.Post(ctx, "/api/v1/reports/generate", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
