package reports

import (
	"context"
	"database/sql"
	"fmt"
	"time"

	"github.com/deltran/reporting-engine/internal/generators"
	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/google/uuid"
	"github.com/shopspring/decimal"
	"go.uber.org/zap"
)

type AMLReportGenerator struct {
	db            *sql.DB
	excelGen      *generators.ExcelGenerator
	csvGen        *generators.CSVGenerator
	logger        *zap.Logger
}

func NewAMLReportGenerator(
	db *sql.DB,
	excelGen *generators.ExcelGenerator,
	csvGen *generators.CSVGenerator,
	logger *zap.Logger,
) *AMLReportGenerator {
	return &AMLReportGenerator{
		db:       db,
		excelGen: excelGen,
		csvGen:   csvGen,
		logger:   logger,
	}
}

func (g *AMLReportGenerator) Generate(
	ctx context.Context,
	req types.ReportRequest,
) (*types.GenerationResult, error) {
	g.logger.Info("Generating AML report",
		zap.String("format", req.Format),
		zap.Time("period_start", req.PeriodStart),
		zap.Time("period_end", req.PeriodEnd))

	// Gather AML data
	data, err := g.gatherAMLData(ctx, req.PeriodStart, req.PeriodEnd)
	if err != nil {
		return nil, fmt.Errorf("failed to gather AML data: %w", err)
	}

	data.GeneratedBy = req.RequestedBy

	reportID := uuid.New().String()
	var reportData []byte
	var format string

	// Generate report based on format
	switch req.Format {
	case types.FormatExcel, "":
		buf, err := g.excelGen.GenerateAMLReport(data, types.ReportPeriod{
			Start: req.PeriodStart,
			End:   req.PeriodEnd,
		})
		if err != nil {
			return nil, fmt.Errorf("failed to generate Excel: %w", err)
		}
		reportData = buf.Bytes()
		format = types.FormatExcel

	case types.FormatCSV:
		buf, err := g.csvGen.GenerateCSVBuffer(ctx, types.ReportTypeAML, req.PeriodStart, req.PeriodEnd)
		if err != nil {
			return nil, fmt.Errorf("failed to generate CSV: %w", err)
		}
		reportData = buf
		format = types.FormatCSV

	default:
		return nil, fmt.Errorf("unsupported format: %s", req.Format)
	}

	return &types.GenerationResult{
		ReportID:    reportID,
		Data:        reportData,
		Format:      format,
		FileSize:    int64(len(reportData)),
		StoragePath: "",
		GeneratedAt: time.Now(),
	}, nil
}

func (g *AMLReportGenerator) gatherAMLData(
	ctx context.Context,
	periodStart, periodEnd time.Time,
) (*types.AMLReportData, error) {
	data := &types.AMLReportData{
		PeriodStart: periodStart,
		PeriodEnd:   periodEnd,
	}

	// Get overall metrics
	metricsQuery := `
		SELECT
			COUNT(*) as total_transactions,
			SUM(t.amount) as total_volume,
			SUM(CASE WHEN cc.risk_level = 'High' THEN 1 ELSE 0 END) as high_risk_count,
			SUM(CASE WHEN cc.risk_level = 'Medium' THEN 1 ELSE 0 END) as medium_risk_count,
			SUM(CASE WHEN cc.risk_level = 'Low' THEN 1 ELSE 0 END) as low_risk_count,
			SUM(CASE WHEN cc.risk_level = 'Critical' THEN 1 ELSE 0 END) as critical_risk_count,
			SUM(CASE WHEN cc.suspicious_activity = true THEN 1 ELSE 0 END) as suspicious_count,
			SUM(CASE WHEN cc.sanctions_hit = true THEN 1 ELSE 0 END) as sanctions_hits,
			SUM(CASE WHEN cc.false_positive = true THEN 1 ELSE 0 END) as false_positives,
			SUM(CASE WHEN cc.compliance_action IS NOT NULL THEN 1 ELSE 0 END) as compliance_actions
		FROM transactions t
		LEFT JOIN compliance_checks cc ON t.id = cc.transaction_id
		WHERE t.created_at >= $1 AND t.created_at < $2
	`

	var totalVolume sql.NullFloat64
	err := g.db.QueryRowContext(ctx, metricsQuery, periodStart, periodEnd).Scan(
		&data.TotalTransactions,
		&totalVolume,
		&data.HighRiskCount,
		&data.MediumRiskCount,
		&data.LowRiskCount,
		&data.CriticalRiskCount,
		&data.SuspiciousCount,
		&data.SanctionsHits,
		&data.FalsePositives,
		&data.ComplianceActions,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to get metrics: %w", err)
	}

	if totalVolume.Valid {
		data.TotalVolume = decimal.NewFromFloat(totalVolume.Float64)
	}

	// Get high-risk transactions
	txnQuery := `
		SELECT
			t.id,
			t.created_at,
			t.sender_bank,
			t.receiver_bank,
			t.amount,
			t.currency,
			COALESCE(cc.risk_score, 0) as risk_score,
			COALESCE(cc.risk_level, 'Unknown') as risk_level,
			COALESCE(cc.sanctions_hit, false) as sanctions_hit,
			COALESCE(cc.compliance_flag, '') as compliance_flag
		FROM transactions t
		LEFT JOIN compliance_checks cc ON t.id = cc.transaction_id
		WHERE t.created_at >= $1 AND t.created_at < $2
			AND (cc.risk_score > 70 OR cc.sanctions_hit = true)
		ORDER BY cc.risk_score DESC
		LIMIT 1000
	`

	rows, err := g.db.QueryContext(ctx, txnQuery, periodStart, periodEnd)
	if err != nil {
		return nil, fmt.Errorf("failed to get transactions: %w", err)
	}
	defer rows.Close()

	for rows.Next() {
		var txn types.AMLTransaction
		var amount float64

		err := rows.Scan(
			&txn.ID,
			&txn.Timestamp,
			&txn.SenderBank,
			&txn.ReceiverBank,
			&amount,
			&txn.Currency,
			&txn.RiskScore,
			&txn.RiskLevel,
			&txn.SanctionsHit,
			&txn.ComplianceFlag,
		)
		if err != nil {
			g.logger.Error("Failed to scan transaction", zap.Error(err))
			continue
		}

		txn.Amount = decimal.NewFromFloat(amount)
		data.Transactions = append(data.Transactions, txn)
	}

	// Get suspicious activities
	suspiciousQuery := `
		SELECT
			transaction_id,
			detected_at,
			activity_type,
			risk_score,
			indicators,
			status,
			reviewed_by
		FROM suspicious_activities
		WHERE detected_at >= $1 AND detected_at < $2
		ORDER BY risk_score DESC
		LIMIT 500
	`

	rows, err = g.db.QueryContext(ctx, suspiciousQuery, periodStart, periodEnd)
	if err != nil {
		g.logger.Warn("Failed to get suspicious activities", zap.Error(err))
	} else {
		defer rows.Close()
		for rows.Next() {
			var activity types.SuspiciousActivity
			var indicators string
			var reviewedBy sql.NullString

			err := rows.Scan(
				&activity.TransactionID,
				&activity.DetectedAt,
				&activity.ActivityType,
				&activity.RiskScore,
				&indicators,
				&activity.Status,
				&reviewedBy,
			)
			if err != nil {
				g.logger.Error("Failed to scan suspicious activity", zap.Error(err))
				continue
			}

			if reviewedBy.Valid {
				activity.ReviewedBy = reviewedBy.String
			}

			// Parse indicators (assume comma-separated)
			activity.Indicators = []string{indicators}

			data.SuspiciousActivities = append(data.SuspiciousActivities, activity)
		}
	}

	return data, nil
}
