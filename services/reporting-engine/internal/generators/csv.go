package generators

import (
	"context"
	"database/sql"
	"encoding/csv"
	"fmt"
	"io"
	"time"

	"github.com/deltran/reporting-engine/pkg/types"
	"go.uber.org/zap"
)

type CSVGenerator struct {
	db     *sql.DB
	logger *zap.Logger
}

func NewCSVGenerator(db *sql.DB, logger *zap.Logger) *CSVGenerator {
	return &CSVGenerator{
		db:     db,
		logger: logger,
	}
}

// StreamCSV generates CSV with streaming for 1M+ rows without loading all data into memory
func (g *CSVGenerator) StreamCSV(
	ctx context.Context,
	writer io.Writer,
	reportType string,
	periodStart, periodEnd time.Time,
) error {
	csvWriter := csv.NewWriter(writer)
	defer csvWriter.Flush()

	startTime := time.Now()
	rowCount := 0

	switch reportType {
	case types.ReportTypeAML:
		rowCount = g.streamAMLTransactions(ctx, csvWriter, periodStart, periodEnd)
	case types.ReportTypeSettlement:
		rowCount = g.streamSettlementData(ctx, csvWriter, periodStart, periodEnd)
	case types.ReportTypeReconciliation:
		rowCount = g.streamReconciliationData(ctx, csvWriter, periodStart, periodEnd)
	default:
		return fmt.Errorf("unsupported report type: %s", reportType)
	}

	duration := time.Since(startTime)
	g.logger.Info("CSV generation completed",
		zap.String("type", reportType),
		zap.Int("rows", rowCount),
		zap.Duration("duration", duration),
		zap.Float64("rows_per_second", float64(rowCount)/duration.Seconds()))

	return nil
}

func (g *CSVGenerator) streamAMLTransactions(
	ctx context.Context,
	csvWriter *csv.Writer,
	periodStart, periodEnd time.Time,
) int {
	// Write headers
	headers := []string{
		"Transaction ID",
		"Timestamp",
		"Sender Bank",
		"Receiver Bank",
		"Amount",
		"Currency",
		"Risk Score",
		"Risk Level",
		"Sanctions Hit",
		"Compliance Flag",
		"Notes",
	}
	csvWriter.Write(headers)

	// Query with streaming (using cursor)
	query := `
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
		ORDER BY t.created_at DESC
	`

	rows, err := g.db.QueryContext(ctx, query, periodStart, periodEnd)
	if err != nil {
		g.logger.Error("Failed to query transactions", zap.Error(err))
		return 0
	}
	defer rows.Close()

	rowCount := 0
	batchSize := 1000
	batch := make([][]string, 0, batchSize)

	for rows.Next() {
		var (
			id             string
			timestamp      time.Time
			senderBank     string
			receiverBank   string
			amount         float64
			currency       string
			riskScore      float64
			riskLevel      string
			sanctionsHit   bool
			complianceFlag string
		)

		if err := rows.Scan(
			&id, &timestamp, &senderBank, &receiverBank, &amount, &currency,
			&riskScore, &riskLevel, &sanctionsHit, &complianceFlag,
		); err != nil {
			g.logger.Error("Failed to scan row", zap.Error(err))
			continue
		}

		record := []string{
			id,
			timestamp.Format("2006-01-02 15:04:05"),
			senderBank,
			receiverBank,
			fmt.Sprintf("%.2f", amount),
			currency,
			fmt.Sprintf("%.2f", riskScore),
			riskLevel,
			fmt.Sprintf("%t", sanctionsHit),
			complianceFlag,
			"",
		}

		batch = append(batch, record)
		rowCount++

		// Write in batches to improve performance
		if len(batch) >= batchSize {
			for _, rec := range batch {
				csvWriter.Write(rec)
			}
			csvWriter.Flush()
			batch = batch[:0]

			// Log progress every 100k rows
			if rowCount%100000 == 0 {
				g.logger.Info("CSV streaming progress", zap.Int("rows", rowCount))
			}
		}
	}

	// Write remaining batch
	for _, rec := range batch {
		csvWriter.Write(rec)
	}

	return rowCount
}

func (g *CSVGenerator) streamSettlementData(
	ctx context.Context,
	csvWriter *csv.Writer,
	periodStart, periodEnd time.Time,
) int {
	headers := []string{
		"Window ID",
		"Window Start",
		"Window End",
		"Sender Bank",
		"Receiver Bank",
		"Currency",
		"Gross Amount",
		"Net Amount",
		"Settlement Status",
		"Settlement Time",
	}
	csvWriter.Write(headers)

	query := `
		SELECT
			si.window_id,
			si.window_start,
			si.window_end,
			si.sender_bank,
			si.receiver_bank,
			si.currency,
			si.gross_amount,
			si.net_amount,
			si.status,
			si.settled_at
		FROM settlement_instructions si
		WHERE si.window_start >= $1 AND si.window_start < $2
		ORDER BY si.window_start DESC, si.sender_bank
	`

	rows, err := g.db.QueryContext(ctx, query, periodStart, periodEnd)
	if err != nil {
		g.logger.Error("Failed to query settlement data", zap.Error(err))
		return 0
	}
	defer rows.Close()

	rowCount := 0
	for rows.Next() {
		var (
			windowID     int64
			windowStart  time.Time
			windowEnd    time.Time
			senderBank   string
			receiverBank string
			currency     string
			grossAmount  float64
			netAmount    float64
			status       string
			settledAt    sql.NullTime
		)

		if err := rows.Scan(
			&windowID, &windowStart, &windowEnd, &senderBank, &receiverBank,
			&currency, &grossAmount, &netAmount, &status, &settledAt,
		); err != nil {
			g.logger.Error("Failed to scan settlement row", zap.Error(err))
			continue
		}

		settledTime := ""
		if settledAt.Valid {
			settledTime = settledAt.Time.Format("2006-01-02 15:04:05")
		}

		record := []string{
			fmt.Sprintf("%d", windowID),
			windowStart.Format("2006-01-02 15:04:05"),
			windowEnd.Format("2006-01-02 15:04:05"),
			senderBank,
			receiverBank,
			currency,
			fmt.Sprintf("%.2f", grossAmount),
			fmt.Sprintf("%.2f", netAmount),
			status,
			settledTime,
		}

		csvWriter.Write(record)
		rowCount++

		if rowCount%10000 == 0 {
			csvWriter.Flush()
			g.logger.Info("Settlement CSV streaming progress", zap.Int("rows", rowCount))
		}
	}

	return rowCount
}

func (g *CSVGenerator) streamReconciliationData(
	ctx context.Context,
	csvWriter *csv.Writer,
	periodStart, periodEnd time.Time,
) int {
	headers := []string{
		"Transaction ID",
		"Bank Reference",
		"Amount",
		"Currency",
		"Status",
		"Match Type",
		"Discrepancy",
		"Reconciled At",
		"Notes",
	}
	csvWriter.Write(headers)

	query := `
		SELECT
			t.id,
			t.bank_reference,
			t.amount,
			t.currency,
			t.status,
			CASE
				WHEN r.matched THEN 'Matched'
				WHEN r.discrepancy_amount != 0 THEN 'Discrepancy'
				ELSE 'Unmatched'
			END as match_type,
			COALESCE(r.discrepancy_amount, 0) as discrepancy,
			r.reconciled_at,
			COALESCE(r.notes, '') as notes
		FROM transactions t
		LEFT JOIN reconciliation_records r ON t.id = r.transaction_id
		WHERE t.created_at >= $1 AND t.created_at < $2
		ORDER BY t.created_at DESC
	`

	rows, err := g.db.QueryContext(ctx, query, periodStart, periodEnd)
	if err != nil {
		g.logger.Error("Failed to query reconciliation data", zap.Error(err))
		return 0
	}
	defer rows.Close()

	rowCount := 0
	for rows.Next() {
		var (
			id            string
			bankRef       sql.NullString
			amount        float64
			currency      string
			status        string
			matchType     string
			discrepancy   float64
			reconciledAt  sql.NullTime
			notes         string
		)

		if err := rows.Scan(
			&id, &bankRef, &amount, &currency, &status,
			&matchType, &discrepancy, &reconciledAt, &notes,
		); err != nil {
			g.logger.Error("Failed to scan reconciliation row", zap.Error(err))
			continue
		}

		bankReference := ""
		if bankRef.Valid {
			bankReference = bankRef.String
		}

		reconciledTime := ""
		if reconciledAt.Valid {
			reconciledTime = reconciledAt.Time.Format("2006-01-02 15:04:05")
		}

		record := []string{
			id,
			bankReference,
			fmt.Sprintf("%.2f", amount),
			currency,
			status,
			matchType,
			fmt.Sprintf("%.2f", discrepancy),
			reconciledTime,
			notes,
		}

		csvWriter.Write(record)
		rowCount++

		if rowCount%50000 == 0 {
			csvWriter.Flush()
			g.logger.Info("Reconciliation CSV streaming progress", zap.Int("rows", rowCount))
		}
	}

	return rowCount
}

// GenerateCSVBuffer generates CSV into a buffer (for smaller datasets)
func (g *CSVGenerator) GenerateCSVBuffer(
	ctx context.Context,
	reportType string,
	periodStart, periodEnd time.Time,
) ([]byte, error) {
	var buffer []byte
	writer := &byteWriter{data: &buffer}

	err := g.StreamCSV(ctx, writer, reportType, periodStart, periodEnd)
	if err != nil {
		return nil, err
	}

	return buffer, nil
}

// byteWriter implements io.Writer for []byte
type byteWriter struct {
	data *[]byte
}

func (bw *byteWriter) Write(p []byte) (n int, err error) {
	*bw.data = append(*bw.data, p...)
	return len(p), nil
}
