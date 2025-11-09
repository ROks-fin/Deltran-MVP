package types

import (
	"time"

	"github.com/shopspring/decimal"
)

// Report types
const (
	ReportTypeAML            = "aml"
	ReportTypeSettlement     = "settlement"
	ReportTypeReconciliation = "reconciliation"
	ReportTypeOperational    = "operational"
	ReportTypeAuditTrail     = "audit_trail"
)

// Report formats
const (
	FormatExcel = "excel"
	FormatCSV   = "csv"
	FormatPDF   = "pdf"
)

// Report status
const (
	StatusPending   = "pending"
	StatusProcessing = "processing"
	StatusCompleted = "completed"
	StatusFailed    = "failed"
)

// Report represents a generated report
type Report struct {
	ID           string            `json:"id"`
	Type         string            `json:"type"`
	Name         string            `json:"name"`
	Description  string            `json:"description"`
	PeriodStart  time.Time         `json:"period_start"`
	PeriodEnd    time.Time         `json:"period_end"`
	GeneratedAt  time.Time         `json:"generated_at"`
	GeneratedBy  string            `json:"generated_by"`
	Status       string            `json:"status"`
	StoragePath  string            `json:"storage_path"`
	FileSize     int64             `json:"file_size"`
	Format       string            `json:"format"`
	Metadata     map[string]string `json:"metadata"`
	ErrorMessage string            `json:"error_message,omitempty"`
}

// ReportRequest represents a request to generate a report
type ReportRequest struct {
	Type        string            `json:"type"`
	Format      string            `json:"format"`
	PeriodStart time.Time         `json:"period_start"`
	PeriodEnd   time.Time         `json:"period_end"`
	Parameters  map[string]string `json:"parameters"`
	RequestedBy string            `json:"requested_by"`
}

// ReportPeriod represents a time period for a report
type ReportPeriod struct {
	Start time.Time
	End   time.Time
}

// AML Report Data Structures

type AMLReportData struct {
	PeriodStart         time.Time
	PeriodEnd           time.Time
	TotalTransactions   int64
	TotalVolume         decimal.Decimal
	HighRiskCount       int64
	MediumRiskCount     int64
	LowRiskCount        int64
	CriticalRiskCount   int64
	SuspiciousCount     int64
	SanctionsHits       int64
	FalsePositives      int64
	ComplianceActions   int64
	GeneratedBy         string
	Transactions        []AMLTransaction
	RiskDistribution    []RiskCategory
	SuspiciousActivities []SuspiciousActivity
}

type AMLTransaction struct {
	ID             string
	Timestamp      time.Time
	SenderBank     string
	ReceiverBank   string
	Amount         decimal.Decimal
	Currency       string
	RiskScore      float64
	RiskLevel      string
	SanctionsHit   bool
	ComplianceFlag string
}

type RiskCategory struct {
	Level string
	Count int64
	Percentage float64
}

type SuspiciousActivity struct {
	TransactionID string
	DetectedAt    time.Time
	ActivityType  string
	RiskScore     float64
	Indicators    []string
	Status        string
	ReviewedBy    string
}

// Settlement Report Data Structures

type SettlementReportData struct {
	Date              time.Time
	WindowID          int64
	GrossPositions    []GrossPosition
	NettingResults    []NettingResult
	NetObligations    []NetObligation
	LiquidityAnalysis LiquidityAnalysis
	EfficiencyMetrics EfficiencyMetrics
}

type GrossPosition struct {
	SenderBank       string
	SenderName       string
	ReceiverBank     string
	ReceiverName     string
	Currency         string
	GrossAmount      decimal.Decimal
	TransactionCount int64
}

type NettingResult struct {
	BankPair        string
	Currency        string
	GrossOutflow    decimal.Decimal
	GrossInflow     decimal.Decimal
	NetAmount       decimal.Decimal
	Direction       string
	TransactionsSaved int64
}

type NetObligation struct {
	Bank            string
	Currency        string
	NetPosition     decimal.Decimal
	PositionType    string
	SettlementAmount decimal.Decimal
	Status          string
}

type LiquidityAnalysis struct {
	TotalLiquidity   decimal.Decimal
	RequiredLiquidity decimal.Decimal
	AvailableLiquidity decimal.Decimal
	LiquidityGaps    []LiquidityGap
}

type LiquidityGap struct {
	Bank     string
	Currency string
	Gap      decimal.Decimal
	Severity string
}

type EfficiencyMetrics struct {
	GrossVolume         decimal.Decimal
	NetVolume          decimal.Decimal
	VolumeReduction    decimal.Decimal
	ReductionPercent   float64
	TransactionsSaved  int
	EstimatedCostSaved decimal.Decimal
}

// Reconciliation Report Data Structures

type ReconciliationReportData struct {
	Date              time.Time
	MatchedCount      int64
	UnmatchedCount    int64
	DiscrepancyCount  int64
	TotalRecords      int64
	MatchedItems      []ReconciliationItem
	Discrepancies     []Discrepancy
	UnmatchedItems    []UnmatchedItem
}

type ReconciliationItem struct {
	TransactionID string
	BankReference string
	Amount        decimal.Decimal
	Status        string
	MatchedAt     time.Time
}

type Discrepancy struct {
	TransactionID string
	ExpectedAmount decimal.Decimal
	ActualAmount  decimal.Decimal
	Difference    decimal.Decimal
	Type          string
	DetectedAt    time.Time
}

type UnmatchedItem struct {
	Reference  string
	Amount     decimal.Decimal
	Source     string
	Age        int // days
	Status     string
}

// Operational Report Data Structures

type OperationalReportData struct {
	Date              time.Time
	SystemMetrics     SystemMetrics
	PerformanceMetrics PerformanceMetrics
	ErrorAnalysis     ErrorAnalysis
	HealthChecks      []HealthCheck
}

type SystemMetrics struct {
	Uptime            time.Duration
	TotalRequests     int64
	SuccessfulRequests int64
	FailedRequests    int64
	AvgResponseTime   time.Duration
	P95ResponseTime   time.Duration
	P99ResponseTime   time.Duration
}

type PerformanceMetrics struct {
	Throughput       int64 // TPS
	DatabaseLatency  time.Duration
	CacheHitRatio    float64
	QueueDepth       int64
	ActiveConnections int64
}

type ErrorAnalysis struct {
	TotalErrors     int64
	ErrorsByType    map[string]int64
	TopErrors       []ErrorSummary
	ErrorRate       float64
}

type ErrorSummary struct {
	ErrorType   string
	Count       int64
	LastOccurred time.Time
	Message     string
}

type HealthCheck struct {
	Service   string
	Status    string
	Latency   time.Duration
	CheckedAt time.Time
	Details   string
}

// Report Schedule

type ReportSchedule struct {
	ID         string
	Name       string
	ReportType string
	Cron       string
	Recipients []string
	Formats    []string
	Parameters map[string]string
	Enabled    bool
	LastRun    *time.Time
	NextRun    *time.Time
}

// Report Template

type ReportTemplate struct {
	ID      string
	Name    string
	Type    string
	Version int
	Layout  TemplateLayout
	Styles  TemplateStyles
	Active  bool
}

type TemplateLayout struct {
	Sheets []string `json:"sheets"`
}

type TemplateStyles struct {
	HeaderBG   string `json:"header_bg"`
	HeaderFont string `json:"header_font"`
	DataFont   string `json:"data_font"`
}

// Report Generation Result

type GenerationResult struct {
	ReportID    string
	Data        []byte
	Format      string
	FileSize    int64
	StoragePath string
	GeneratedAt time.Time
	Error       error
}
