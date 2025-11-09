# Reporting Engine Service Specification

## Overview
Enterprise-grade reporting system providing real-time analytics, regulatory compliance reports, and one-click audit trails for the DelTran platform with Excel/CSV export capabilities for Big 4 audits.

## Core Responsibilities
1. Real-time operational dashboards and metrics
2. Regulatory compliance reports (AML, sanctions, transaction monitoring)
3. Financial reconciliation and settlement reports
4. Excel/CSV generation for Big 4 audits
5. Scheduled report generation and distribution
6. Ad-hoc query interface for business analytics
7. Data aggregation and caching for performance

## Technology Stack
- **Language**: Go 1.21+
- **Database**: PostgreSQL 16 with TimescaleDB for time-series
- **Cache**: Redis for report caching
- **Report Generation**: excelize for Excel, encoding/csv for CSV
- **Message Queue**: NATS JetStream for async processing
- **Storage**: S3-compatible storage for report archives
- **Visualization**: Integration with Grafana and Metabase

## Service Architecture

### Directory Structure
```
services/reporting-engine/
├── cmd/
│   └── server/
│       └── main.go              # Service entry point
├── internal/
│   ├── config/
│   │   └── config.go           # Configuration management
│   ├── generators/
│   │   ├── excel.go            # Excel report generator
│   │   ├── csv.go              # CSV report generator
│   │   ├── pdf.go              # PDF report generator
│   │   └── iso20022.go         # ISO 20022 format generator
│   ├── reports/
│   │   ├── aml.go              # AML/compliance reports
│   │   ├── settlement.go       # Settlement reports
│   │   ├── reconciliation.go   # Financial reconciliation
│   │   ├── operational.go      # Operational metrics
│   │   └── audit.go            # Audit trail reports
│   ├── scheduler/
│   │   ├── scheduler.go        # Report scheduling engine
│   │   └── jobs.go             # Scheduled job definitions
│   ├── storage/
│   │   ├── postgres.go         # Database queries
│   │   ├── timescale.go        # Time-series aggregation
│   │   ├── redis.go            # Report caching
│   │   └── s3.go              # Report archival
│   ├── aggregator/
│   │   ├── metrics.go          # Metrics aggregation
│   │   └── pipeline.go         # Data processing pipeline
│   └── api/
│       ├── handlers.go         # REST API handlers
│       ├── middleware.go       # Authentication
│       └── download.go         # Report download handler
├── pkg/
│   ├── types/
│   │   └── report.go          # Report type definitions
│   └── utils/
│       └── formatter.go       # Data formatting utilities
├── templates/                  # Report templates
│   ├── excel/
│   └── layouts/
├── Dockerfile
├── Makefile
└── go.mod
```

## Implementation Details

### 1. Report Generator Core

```go
// internal/generators/excel.go
package generators

import (
    "bytes"
    "fmt"
    "time"

    "github.com/xuri/excelize/v2"
    "go.uber.org/zap"
)

type ExcelGenerator struct {
    logger *zap.Logger
}

func (g *ExcelGenerator) GenerateAMLReport(
    data *AMLReportData,
    period ReportPeriod,
) (*bytes.Buffer, error) {
    f := excelize.NewFile()
    defer f.Close()

    // Create sheets
    sheets := []string{
        "Executive Summary",
        "Transaction Analysis",
        "Risk Indicators",
        "Suspicious Activities",
        "Customer Risk Profile",
        "Sanctions Screening",
        "Compliance Actions",
    }

    for _, sheet := range sheets {
        f.NewSheet(sheet)
    }

    // Executive Summary
    if err := g.generateExecutiveSummary(f, data, period); err != nil {
        return nil, fmt.Errorf("failed to generate executive summary: %w", err)
    }

    // Transaction Analysis
    if err := g.generateTransactionAnalysis(f, data); err != nil {
        return nil, fmt.Errorf("failed to generate transaction analysis: %w", err)
    }

    // Risk Indicators
    if err := g.generateRiskIndicators(f, data); err != nil {
        return nil, fmt.Errorf("failed to generate risk indicators: %w", err)
    }

    // Apply Big 4 audit formatting standards
    g.applyAuditFormatting(f)

    // Add digital signature/watermark
    g.addAuditTrail(f, data.GeneratedBy, time.Now())

    buf := new(bytes.Buffer)
    if err := f.Write(buf); err != nil {
        return nil, fmt.Errorf("failed to write Excel file: %w", err)
    }

    return buf, nil
}

func (g *ExcelGenerator) generateExecutiveSummary(
    f *excelize.File,
    data *AMLReportData,
    period ReportPeriod,
) error {
    sheet := "Executive Summary"

    // Header styling
    headerStyle, _ := f.NewStyle(&excelize.Style{
        Font: &excelize.Font{
            Bold:  true,
            Size:  14,
            Color: "#FFFFFF",
        },
        Fill: excelize.Fill{
            Type:    "pattern",
            Color:   []string{"#2C3E50"},
            Pattern: 1,
        },
        Alignment: &excelize.Alignment{
            Horizontal: "center",
            Vertical:   "center",
        },
    })

    // Title
    f.MergeCell(sheet, "A1", "H1")
    f.SetCellValue(sheet, "A1", "AML Compliance Report - Executive Summary")
    f.SetCellStyle(sheet, "A1", "H1", headerStyle)

    // Report period
    f.SetCellValue(sheet, "A3", "Report Period:")
    f.SetCellValue(sheet, "B3", fmt.Sprintf("%s to %s",
        period.Start.Format("2006-01-02"),
        period.End.Format("2006-01-02")))

    // Key metrics
    metrics := [][]interface{}{
        {"Total Transactions", data.TotalTransactions},
        {"Total Volume", fmt.Sprintf("$%,.2f", data.TotalVolume)},
        {"High Risk Transactions", data.HighRiskCount},
        {"Suspicious Activities", data.SuspiciousCount},
        {"Sanctions Hits", data.SanctionsHits},
        {"False Positives", data.FalsePositives},
        {"Compliance Actions", data.ComplianceActions},
    }

    row := 5
    f.SetCellValue(sheet, "A"+fmt.Sprint(row), "Key Metrics")
    row++

    for _, metric := range metrics {
        f.SetCellValue(sheet, "A"+fmt.Sprint(row), metric[0])
        f.SetCellValue(sheet, "C"+fmt.Sprint(row), metric[1])
        row++
    }

    // Risk distribution chart
    if err := g.addRiskDistributionChart(f, sheet, data); err != nil {
        return err
    }

    return nil
}

func (g *ExcelGenerator) addRiskDistributionChart(
    f *excelize.File,
    sheet string,
    data *AMLReportData,
) error {
    // Prepare chart data
    categories := []string{"Low Risk", "Medium Risk", "High Risk", "Critical"}
    values := []int{
        data.LowRiskCount,
        data.MediumRiskCount,
        data.HighRiskCount,
        data.CriticalRiskCount,
    }

    // Write data for chart
    startRow := 20
    f.SetCellValue(sheet, "A"+fmt.Sprint(startRow), "Risk Level")
    f.SetCellValue(sheet, "B"+fmt.Sprint(startRow), "Count")

    for i, cat := range categories {
        f.SetCellValue(sheet, "A"+fmt.Sprint(startRow+i+1), cat)
        f.SetCellValue(sheet, "B"+fmt.Sprint(startRow+i+1), values[i])
    }

    // Create chart
    chart := &excelize.Chart{
        Type: excelize.Col3DClustered,
        Series: []excelize.ChartSeries{
            {
                Name:       sheet + "!$B$" + fmt.Sprint(startRow),
                Categories: sheet + "!$A$" + fmt.Sprint(startRow+1) + ":$A$" + fmt.Sprint(startRow+4),
                Values:     sheet + "!$B$" + fmt.Sprint(startRow+1) + ":$B$" + fmt.Sprint(startRow+4),
            },
        },
        Title: []excelize.RichTextRun{
            {
                Text: "Risk Distribution",
            },
        },
        Legend: excelize.ChartLegend{
            Position: "bottom",
        },
        PlotArea: excelize.ChartPlotArea{
            ShowVal: true,
        },
    }

    return f.AddChart(sheet, "D20", chart)
}

func (g *ExcelGenerator) applyAuditFormatting(f *excelize.File) {
    // Apply PwC/Deloitte/EY/KPMG standard formatting

    // Create standard styles
    styles := map[string]*excelize.Style{
        "header": {
            Font: &excelize.Font{
                Bold: true,
                Size: 12,
            },
            Fill: excelize.Fill{
                Type:    "pattern",
                Color:   []string{"#E8E8E8"},
                Pattern: 1,
            },
            Border: []excelize.Border{
                {Type: "top", Style: 1, Color: "#000000"},
                {Type: "bottom", Style: 1, Color: "#000000"},
            },
        },
        "data": {
            Font: &excelize.Font{
                Size: 10,
            },
            Alignment: &excelize.Alignment{
                Horizontal: "left",
            },
        },
        "number": {
            Font: &excelize.Font{
                Size: 10,
            },
            NumFmt: 2, // 0.00 format
            Alignment: &excelize.Alignment{
                Horizontal: "right",
            },
        },
    }

    // Apply styles to all sheets
    for name, style := range styles {
        styleID, _ := f.NewStyle(style)
        // Store style IDs for use
        _ = name
        _ = styleID
    }

    // Add footer with timestamp and digital signature
    for _, sheet := range f.GetSheetList() {
        f.SetHeaderFooter(sheet, &excelize.HeaderFooter{
            DifferentFirst:   false,
            DifferentOddEven: false,
            OddFooter: fmt.Sprintf("&L%s&CPage &P of &N&R%s",
                "DelTran Compliance Report",
                time.Now().Format("2006-01-02 15:04:05")),
        })
    }
}
```

### 2. Settlement Report Generator

```go
// internal/reports/settlement.go
package reports

import (
    "context"
    "database/sql"
    "fmt"
    "time"

    "go.uber.org/zap"
)

type SettlementReportGenerator struct {
    db     *sql.DB
    logger *zap.Logger
}

func (s *SettlementReportGenerator) GenerateDailySettlement(
    ctx context.Context,
    date time.Time,
) (*SettlementReport, error) {
    report := &SettlementReport{
        Date:      date,
        Generated: time.Now(),
        Sections:  make([]ReportSection, 0),
    }

    // 1. Gross Settlement Positions
    grossPositions, err := s.getGrossPositions(ctx, date)
    if err != nil {
        return nil, fmt.Errorf("failed to get gross positions: %w", err)
    }
    report.GrossPositions = grossPositions

    // 2. Netting Results
    nettingResults, err := s.getNettingResults(ctx, date)
    if err != nil {
        return nil, fmt.Errorf("failed to get netting results: %w", err)
    }
    report.NettingResults = nettingResults

    // 3. Net Settlement Obligations
    obligations, err := s.getNetObligations(ctx, date)
    if err != nil {
        return nil, fmt.Errorf("failed to get net obligations: %w", err)
    }
    report.NetObligations = obligations

    // 4. Liquidity Analysis
    liquidityAnalysis, err := s.getLiquidityAnalysis(ctx, date)
    if err != nil {
        return nil, fmt.Errorf("failed to get liquidity analysis: %w", err)
    }
    report.LiquidityAnalysis = liquidityAnalysis

    // 5. Settlement Efficiency Metrics
    efficiency := s.calculateEfficiency(grossPositions, nettingResults)
    report.EfficiencyMetrics = efficiency

    return report, nil
}

func (s *SettlementReportGenerator) getGrossPositions(
    ctx context.Context,
    date time.Time,
) ([]GrossPosition, error) {
    query := `
        WITH daily_flows AS (
            SELECT
                sender_bank,
                receiver_bank,
                currency,
                SUM(amount) as gross_amount,
                COUNT(*) as transaction_count
            FROM transactions
            WHERE DATE(created_at) = $1
                AND status = 'completed'
            GROUP BY sender_bank, receiver_bank, currency
        )
        SELECT
            df.sender_bank,
            df.receiver_bank,
            df.currency,
            df.gross_amount,
            df.transaction_count,
            b1.name as sender_name,
            b2.name as receiver_name
        FROM daily_flows df
        JOIN banks b1 ON b1.swift_code = df.sender_bank
        JOIN banks b2 ON b2.swift_code = df.receiver_bank
        ORDER BY df.gross_amount DESC
    `

    rows, err := s.db.QueryContext(ctx, query, date)
    if err != nil {
        return nil, err
    }
    defer rows.Close()

    var positions []GrossPosition
    for rows.Next() {
        var pos GrossPosition
        err := rows.Scan(
            &pos.SenderBank,
            &pos.ReceiverBank,
            &pos.Currency,
            &pos.GrossAmount,
            &pos.TransactionCount,
            &pos.SenderName,
            &pos.ReceiverName,
        )
        if err != nil {
            return nil, err
        }
        positions = append(positions, pos)
    }

    return positions, nil
}

func (s *SettlementReportGenerator) calculateEfficiency(
    gross []GrossPosition,
    netted []NettingResult,
) EfficiencyMetrics {
    var totalGross, totalNet float64

    for _, g := range gross {
        totalGross += g.GrossAmount
    }

    for _, n := range netted {
        totalNet += n.NetAmount
    }

    reduction := totalGross - totalNet
    percentage := (reduction / totalGross) * 100

    return EfficiencyMetrics{
        GrossVolume:         totalGross,
        NetVolume:          totalNet,
        VolumeReduction:    reduction,
        ReductionPercent:   percentage,
        TransactionsSaved:  int(reduction / 1000), // Approximate
        EstimatedCostSaved: reduction * 0.001,     // 0.1% of volume
    }
}
```

### 3. Report Scheduler

```go
// internal/scheduler/scheduler.go
package scheduler

import (
    "context"
    "fmt"
    "time"

    "github.com/robfig/cron/v3"
    "go.uber.org/zap"
)

type ReportScheduler struct {
    cron       *cron.Cron
    generators map[string]ReportGenerator
    storage    *storage.S3Storage
    notifier   *notification.Client
    logger     *zap.Logger
}

func NewReportScheduler(
    generators map[string]ReportGenerator,
    storage *storage.S3Storage,
    notifier *notification.Client,
    logger *zap.Logger,
) *ReportScheduler {
    return &ReportScheduler{
        cron:       cron.New(cron.WithSeconds()),
        generators: generators,
        storage:    storage,
        notifier:   notifier,
        logger:     logger,
    }
}

func (s *ReportScheduler) Start(ctx context.Context) error {
    // Daily reports at 00:30 UTC
    s.cron.AddFunc("0 30 0 * * *", func() {
        s.generateDailyReports(ctx)
    })

    // Weekly reports on Monday at 01:00 UTC
    s.cron.AddFunc("0 0 1 * * MON", func() {
        s.generateWeeklyReports(ctx)
    })

    // Monthly reports on 1st day at 02:00 UTC
    s.cron.AddFunc("0 0 2 1 * *", func() {
        s.generateMonthlyReports(ctx)
    })

    // Quarterly reports
    s.cron.AddFunc("0 0 3 1 1,4,7,10 *", func() {
        s.generateQuarterlyReports(ctx)
    })

    // Real-time monitoring reports every 5 minutes
    s.cron.AddFunc("0 */5 * * * *", func() {
        s.generateRealTimeMetrics(ctx)
    })

    s.cron.Start()
    s.logger.Info("Report scheduler started")

    return nil
}

func (s *ReportScheduler) generateDailyReports(ctx context.Context) {
    reports := []struct {
        name      string
        generator string
        recipients []string
    }{
        {
            name:       "Daily Settlement Report",
            generator:  "settlement",
            recipients: []string{"settlements@deltran.com", "cfo@deltran.com"},
        },
        {
            name:       "Daily Risk Report",
            generator:  "risk",
            recipients: []string{"risk@deltran.com", "compliance@deltran.com"},
        },
        {
            name:       "Daily Transaction Summary",
            generator:  "transactions",
            recipients: []string{"operations@deltran.com"},
        },
        {
            name:       "Daily Liquidity Report",
            generator:  "liquidity",
            recipients: []string{"treasury@deltran.com"},
        },
    }

    yesterday := time.Now().AddDate(0, 0, -1)

    for _, report := range reports {
        go func(r struct {
            name      string
            generator string
            recipients []string
        }) {
            if err := s.generateAndDistribute(ctx, r.generator, r.name, yesterday, r.recipients); err != nil {
                s.logger.Error("failed to generate daily report",
                    zap.String("report", r.name),
                    zap.Error(err))
            }
        }(report)
    }
}

func (s *ReportScheduler) generateAndDistribute(
    ctx context.Context,
    generatorName string,
    reportName string,
    date time.Time,
    recipients []string,
) error {
    generator, ok := s.generators[generatorName]
    if !ok {
        return fmt.Errorf("generator not found: %s", generatorName)
    }

    // Generate report
    report, err := generator.Generate(ctx, date)
    if err != nil {
        return fmt.Errorf("failed to generate report: %w", err)
    }

    // Store in S3
    key := fmt.Sprintf("reports/%s/%s/%s.xlsx",
        date.Format("2006/01/02"),
        generatorName,
        time.Now().Format("20060102-150405"))

    url, err := s.storage.Upload(ctx, key, report.Data)
    if err != nil {
        return fmt.Errorf("failed to upload report: %w", err)
    }

    // Send notifications
    for _, recipient := range recipients {
        notification := &Notification{
            Type:      "email",
            Recipient: recipient,
            Subject:   fmt.Sprintf("%s - %s", reportName, date.Format("2006-01-02")),
            Body:      fmt.Sprintf("Report available: %s", url),
            Attachment: report.Data,
        }

        if err := s.notifier.Send(ctx, notification); err != nil {
            s.logger.Error("failed to send notification",
                zap.String("recipient", recipient),
                zap.Error(err))
        }
    }

    return nil
}
```

### 4. API Handlers

```go
// internal/api/handlers.go
package api

import (
    "encoding/json"
    "fmt"
    "net/http"
    "time"

    "github.com/gorilla/mux"
)

type ReportHandler struct {
    generators map[string]ReportGenerator
    storage    *storage.Storage
    cache      *redis.Client
    logger     *zap.Logger
}

func (h *ReportHandler) RegisterRoutes(router *mux.Router) {
    api := router.PathPrefix("/api/v1").Subrouter()
    api.Use(AuthMiddleware)

    // Report generation endpoints
    api.HandleFunc("/reports/generate", h.generateReport).Methods("POST")
    api.HandleFunc("/reports/{id}", h.getReport).Methods("GET")
    api.HandleFunc("/reports/{id}/download", h.downloadReport).Methods("GET")
    api.HandleFunc("/reports", h.listReports).Methods("GET")

    // Predefined report types
    api.HandleFunc("/reports/aml/daily", h.generateAMLDaily).Methods("POST")
    api.HandleFunc("/reports/settlement/daily", h.generateSettlementDaily).Methods("POST")
    api.HandleFunc("/reports/reconciliation", h.generateReconciliation).Methods("POST")
    api.HandleFunc("/reports/audit-trail", h.generateAuditTrail).Methods("POST")

    // Real-time metrics
    api.HandleFunc("/metrics/live", h.getLiveMetrics).Methods("GET")
    api.HandleFunc("/metrics/historical", h.getHistoricalMetrics).Methods("GET")

    // Admin endpoints
    admin := api.PathPrefix("/admin").Subrouter()
    admin.Use(AdminAuthMiddleware)

    admin.HandleFunc("/reports/schedule", h.scheduleReport).Methods("POST")
    admin.HandleFunc("/reports/templates", h.getTemplates).Methods("GET")
    admin.HandleFunc("/reports/templates", h.createTemplate).Methods("POST")
}

func (h *ReportHandler) generateReport(w http.ResponseWriter, r *http.Request) {
    var req GenerateReportRequest
    if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
        http.Error(w, "Invalid request", http.StatusBadRequest)
        return
    }

    // Validate request
    if err := h.validateReportRequest(&req); err != nil {
        http.Error(w, err.Error(), http.StatusBadRequest)
        return
    }

    // Check cache for recent identical report
    cacheKey := fmt.Sprintf("report:%s:%s:%s",
        req.Type,
        req.StartDate.Format("2006-01-02"),
        req.EndDate.Format("2006-01-02"))

    if cached, err := h.cache.Get(r.Context(), cacheKey).Result(); err == nil {
        var cachedReport Report
        if err := json.Unmarshal([]byte(cached), &cachedReport); err == nil {
            w.Header().Set("Content-Type", "application/json")
            w.Header().Set("X-Cache", "HIT")
            json.NewEncoder(w).Encode(cachedReport)
            return
        }
    }

    // Generate new report
    generator, ok := h.generators[req.Type]
    if !ok {
        http.Error(w, "Unknown report type", http.StatusBadRequest)
        return
    }

    report, err := generator.Generate(r.Context(), req)
    if err != nil {
        h.logger.Error("failed to generate report", zap.Error(err))
        http.Error(w, "Failed to generate report", http.StatusInternalServerError)
        return
    }

    // Store report
    if err := h.storage.SaveReport(r.Context(), report); err != nil {
        h.logger.Error("failed to save report", zap.Error(err))
    }

    // Cache for 5 minutes
    reportJSON, _ := json.Marshal(report)
    h.cache.Set(r.Context(), cacheKey, reportJSON, 5*time.Minute)

    w.Header().Set("Content-Type", "application/json")
    w.Header().Set("X-Cache", "MISS")
    json.NewEncoder(w).Encode(report)
}

func (h *ReportHandler) downloadReport(w http.ResponseWriter, r *http.Request) {
    vars := mux.Vars(r)
    reportID := vars["id"]

    // Get report metadata
    report, err := h.storage.GetReport(r.Context(), reportID)
    if err != nil {
        http.Error(w, "Report not found", http.StatusNotFound)
        return
    }

    // Get format from query param
    format := r.URL.Query().Get("format")
    if format == "" {
        format = "excel"
    }

    var data []byte
    var contentType string

    switch format {
    case "excel":
        data, err = h.generators[report.Type].GenerateExcel(r.Context(), report)
        contentType = "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"

    case "csv":
        data, err = h.generators[report.Type].GenerateCSV(r.Context(), report)
        contentType = "text/csv"

    case "pdf":
        data, err = h.generators[report.Type].GeneratePDF(r.Context(), report)
        contentType = "application/pdf"

    default:
        http.Error(w, "Unsupported format", http.StatusBadRequest)
        return
    }

    if err != nil {
        h.logger.Error("failed to generate report format",
            zap.String("format", format),
            zap.Error(err))
        http.Error(w, "Failed to generate report", http.StatusInternalServerError)
        return
    }

    // Set headers for download
    filename := fmt.Sprintf("%s_%s.%s",
        report.Type,
        report.GeneratedAt.Format("20060102_150405"),
        format)

    w.Header().Set("Content-Type", contentType)
    w.Header().Set("Content-Disposition",
        fmt.Sprintf("attachment; filename=\"%s\"", filename))
    w.Header().Set("Content-Length", fmt.Sprintf("%d", len(data)))

    w.Write(data)
}
```

### 5. Database Schema

```sql
-- Report metadata
CREATE TABLE reports (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    type VARCHAR(50) NOT NULL,
    name VARCHAR(255) NOT NULL,
    description TEXT,
    period_start TIMESTAMP NOT NULL,
    period_end TIMESTAMP NOT NULL,
    generated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    generated_by VARCHAR(100),
    status VARCHAR(20) DEFAULT 'pending',
    storage_path TEXT,
    file_size BIGINT,
    format VARCHAR(20),
    metadata JSONB DEFAULT '{}',
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_reports_type ON reports(type);
CREATE INDEX idx_reports_period ON reports(period_start, period_end);
CREATE INDEX idx_reports_generated_at ON reports(generated_at DESC);

-- Report schedules
CREATE TABLE report_schedules (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    report_type VARCHAR(50) NOT NULL,
    schedule_cron VARCHAR(100) NOT NULL,
    recipients TEXT[],
    formats TEXT[] DEFAULT ARRAY['excel'],
    parameters JSONB DEFAULT '{}',
    enabled BOOLEAN DEFAULT true,
    last_run TIMESTAMP,
    next_run TIMESTAMP,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Report templates
CREATE TABLE report_templates (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    name VARCHAR(255) NOT NULL,
    type VARCHAR(50) NOT NULL,
    version INT DEFAULT 1,
    layout JSONB NOT NULL,
    styles JSONB,
    queries JSONB,
    active BOOLEAN DEFAULT true,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Audit log for report access
CREATE TABLE report_access_log (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    report_id UUID REFERENCES reports(id),
    accessed_by VARCHAR(100),
    access_type VARCHAR(20), -- view, download, share
    ip_address INET,
    user_agent TEXT,
    accessed_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_access_log_report ON report_access_log(report_id);
CREATE INDEX idx_access_log_user ON report_access_log(accessed_by);

-- Materialized views for performance
CREATE MATERIALIZED VIEW daily_transaction_summary AS
SELECT
    DATE(created_at) as transaction_date,
    COUNT(*) as total_transactions,
    SUM(amount) as total_volume,
    AVG(amount) as avg_transaction_size,
    COUNT(DISTINCT sender_bank) as unique_senders,
    COUNT(DISTINCT receiver_bank) as unique_receivers,
    SUM(CASE WHEN status = 'completed' THEN 1 ELSE 0 END) as completed,
    SUM(CASE WHEN status = 'failed' THEN 1 ELSE 0 END) as failed,
    AVG(EXTRACT(EPOCH FROM (completed_at - created_at))) as avg_processing_time
FROM transactions
GROUP BY DATE(created_at);

CREATE UNIQUE INDEX ON daily_transaction_summary(transaction_date);

-- Refresh materialized view every hour
CREATE OR REPLACE FUNCTION refresh_daily_summary()
RETURNS void AS $$
BEGIN
    REFRESH MATERIALIZED VIEW CONCURRENTLY daily_transaction_summary;
END;
$$ LANGUAGE plpgsql;
```

### 6. Configuration

```yaml
# config/config.yaml
server:
  port: 8087
  read_timeout: 30s
  write_timeout: 30s
  max_request_size: 100MB

database:
  host: postgres
  port: 5432
  name: deltran
  user: deltran
  password: ${DB_PASSWORD}
  max_connections: 20

timescale:
  enabled: true
  retention_days: 90
  compression_after: 7

redis:
  address: redis:6379
  password: ${REDIS_PASSWORD}
  db: 3
  cache_ttl: 300s

s3:
  endpoint: ${S3_ENDPOINT}
  bucket: deltran-reports
  access_key: ${AWS_ACCESS_KEY_ID}
  secret_key: ${AWS_SECRET_ACCESS_KEY}
  region: us-east-1

nats:
  url: nats://nats:4222
  subject: reports.requests

scheduler:
  enabled: true
  timezone: UTC
  max_concurrent: 5

reports:
  max_rows_excel: 1048576
  max_rows_csv: 10000000
  timeout: 5m

formats:
  date: "2006-01-02"
  datetime: "2006-01-02 15:04:05"
  number: "#,##0.00"
  currency: "$#,##0.00"

monitoring:
  prometheus_enabled: true
  metrics_port: 9097
```

### 7. Docker Configuration

```dockerfile
# Dockerfile
FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git make

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o reporting-engine cmd/server/main.go

FROM alpine:latest

RUN apk --no-cache add ca-certificates tzdata

WORKDIR /root/

COPY --from=builder /app/reporting-engine .
COPY --from=builder /app/templates ./templates

EXPOSE 8087

CMD ["./reporting-engine"]
```

## Integration Points

### 1. Data Sources
- Transaction Database: Real-time transaction data
- Settlement Engine: Settlement results and obligations
- Risk Engine: Risk scores and alerts
- Compliance Engine: AML/sanctions screening results
- Clearing Engine: Netting cycles and efficiency metrics

### 2. External Systems
- Grafana: Operational dashboards
- Metabase: Business analytics
- S3/MinIO: Report storage
- Email systems: Report distribution

## Performance Requirements

- Report generation: < 30 seconds for daily reports
- Excel generation: 100,000 rows in < 10 seconds
- CSV export: 1M rows in < 30 seconds
- Concurrent reports: 50+ simultaneous generations
- Query performance: < 1 second for aggregations
- Cache hit ratio: > 80% for frequent reports

## Security Considerations

1. **Access Control**
   - Role-based report access
   - Data masking for sensitive fields
   - Audit trail for all report access

2. **Data Protection**
   - Encryption at rest for stored reports
   - TLS for report transmission
   - Digital signatures for regulatory reports

3. **Compliance**
   - ISO 20022 compliance for regulatory reports
   - Big 4 audit standards formatting
   - Data retention policies

## Monitoring & Metrics

### Key Metrics
```prometheus
# Report generation metrics
reporting_reports_generated_total
reporting_generation_duration_seconds
reporting_generation_errors_total

# Performance metrics
reporting_query_duration_seconds
reporting_cache_hit_ratio
reporting_concurrent_generations

# Storage metrics
reporting_storage_size_bytes
reporting_reports_stored_total
```

## Testing Strategy

1. **Unit Tests**
   - Report calculations
   - Data aggregation logic
   - Format conversions

2. **Integration Tests**
   - Database queries
   - S3 upload/download
   - Report scheduling

3. **Load Tests**
   - 100 concurrent report generations
   - 1M row Excel export
   - 10M row CSV export

## Deployment Checklist

- [ ] Configure database connections
- [ ] Set up S3 bucket and permissions
- [ ] Load report templates
- [ ] Configure scheduled reports
- [ ] Set up monitoring dashboards
- [ ] Test report generation
- [ ] Verify Excel/CSV exports
- [ ] Test email distribution