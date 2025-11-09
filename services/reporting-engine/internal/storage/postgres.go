package storage

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/deltran/reporting-engine/pkg/types"
	_ "github.com/lib/pq"
	"go.uber.org/zap"
)

type PostgresStorage struct {
	db     *sql.DB
	logger *zap.Logger
}

func NewPostgresStorage(dsn string, logger *zap.Logger) (*PostgresStorage, error) {
	db, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to database: %w", err)
	}

	// Test connection
	if err := db.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	// Set connection pool settings
	db.SetMaxOpenConns(20)
	db.SetMaxIdleConns(5)

	return &PostgresStorage{
		db:     db,
		logger: logger,
	}, nil
}

// SaveReportMetadata saves report metadata to database
func (p *PostgresStorage) SaveReportMetadata(ctx context.Context, report *types.Report) error {
	metadataJSON, err := json.Marshal(report.Metadata)
	if err != nil {
		return fmt.Errorf("failed to marshal metadata: %w", err)
	}

	query := `
		INSERT INTO reports (
			id, type, name, description, period_start, period_end,
			generated_at, generated_by, status, storage_path, file_size,
			format, metadata
		) VALUES (
			$1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13
		)
		ON CONFLICT (id) DO UPDATE SET
			status = EXCLUDED.status,
			storage_path = EXCLUDED.storage_path,
			file_size = EXCLUDED.file_size,
			updated_at = CURRENT_TIMESTAMP
	`

	_, err = p.db.ExecContext(ctx, query,
		report.ID,
		report.Type,
		report.Name,
		report.Description,
		report.PeriodStart,
		report.PeriodEnd,
		report.GeneratedAt,
		report.GeneratedBy,
		report.Status,
		report.StoragePath,
		report.FileSize,
		report.Format,
		metadataJSON,
	)
	if err != nil {
		return fmt.Errorf("failed to save report metadata: %w", err)
	}

	return nil
}

// GetReport retrieves report metadata by ID
func (p *PostgresStorage) GetReport(ctx context.Context, reportID string) (*types.Report, error) {
	query := `
		SELECT
			id, type, name, description, period_start, period_end,
			generated_at, generated_by, status, storage_path, file_size,
			format, metadata, error_message
		FROM reports
		WHERE id = $1
	`

	var report types.Report
	var metadataJSON []byte
	var description, storagePath, errorMessage sql.NullString

	err := p.db.QueryRowContext(ctx, query, reportID).Scan(
		&report.ID,
		&report.Type,
		&report.Name,
		&description,
		&report.PeriodStart,
		&report.PeriodEnd,
		&report.GeneratedAt,
		&report.GeneratedBy,
		&report.Status,
		&storagePath,
		&report.FileSize,
		&report.Format,
		&metadataJSON,
		&errorMessage,
	)
	if err != nil {
		if err == sql.ErrNoRows {
			return nil, fmt.Errorf("report not found: %s", reportID)
		}
		return nil, fmt.Errorf("failed to get report: %w", err)
	}

	if description.Valid {
		report.Description = description.String
	}
	if storagePath.Valid {
		report.StoragePath = storagePath.String
	}
	if errorMessage.Valid {
		report.ErrorMessage = errorMessage.String
	}

	if err := json.Unmarshal(metadataJSON, &report.Metadata); err != nil {
		p.logger.Warn("Failed to unmarshal metadata", zap.Error(err))
		report.Metadata = make(map[string]string)
	}

	return &report, nil
}

// ListReports retrieves a list of reports with optional filters
func (p *PostgresStorage) ListReports(
	ctx context.Context,
	reportType string,
	limit, offset int,
) ([]types.Report, error) {
	query := `
		SELECT
			id, type, name, period_start, period_end,
			generated_at, generated_by, status, file_size, format
		FROM reports
		WHERE ($1 = '' OR type = $1)
		ORDER BY generated_at DESC
		LIMIT $2 OFFSET $3
	`

	rows, err := p.db.QueryContext(ctx, query, reportType, limit, offset)
	if err != nil {
		return nil, fmt.Errorf("failed to list reports: %w", err)
	}
	defer rows.Close()

	var reports []types.Report
	for rows.Next() {
		var report types.Report
		err := rows.Scan(
			&report.ID,
			&report.Type,
			&report.Name,
			&report.PeriodStart,
			&report.PeriodEnd,
			&report.GeneratedAt,
			&report.GeneratedBy,
			&report.Status,
			&report.FileSize,
			&report.Format,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan report: %w", err)
		}
		reports = append(reports, report)
	}

	return reports, nil
}

// DeleteReport marks a report as deleted
func (p *PostgresStorage) DeleteReport(ctx context.Context, reportID string) error {
	query := `UPDATE reports SET status = 'deleted', updated_at = CURRENT_TIMESTAMP WHERE id = $1`

	result, err := p.db.ExecContext(ctx, query, reportID)
	if err != nil {
		return fmt.Errorf("failed to delete report: %w", err)
	}

	rowsAffected, _ := result.RowsAffected()
	if rowsAffected == 0 {
		return fmt.Errorf("report not found: %s", reportID)
	}

	return nil
}

// LogReportAccess logs report access for audit trail
func (p *PostgresStorage) LogReportAccess(
	ctx context.Context,
	reportID, accessedBy, accessType, ipAddress, userAgent string,
) error {
	query := `
		INSERT INTO report_access_log (
			report_id, accessed_by, access_type, ip_address, user_agent
		) VALUES ($1, $2, $3, $4, $5)
	`

	_, err := p.db.ExecContext(ctx, query, reportID, accessedBy, accessType, ipAddress, userAgent)
	if err != nil {
		return fmt.Errorf("failed to log report access: %w", err)
	}

	return nil
}

// RefreshMaterializedViews refreshes all materialized views
func (p *PostgresStorage) RefreshMaterializedViews(ctx context.Context) error {
	query := `SELECT refresh_reporting_views()`

	_, err := p.db.ExecContext(ctx, query)
	if err != nil {
		return fmt.Errorf("failed to refresh materialized views: %w", err)
	}

	return nil
}

// GetDailyTransactionSummary retrieves aggregated daily transaction data
func (p *PostgresStorage) GetDailyTransactionSummary(
	ctx context.Context,
	startDate, endDate string,
) ([]map[string]interface{}, error) {
	query := `
		SELECT
			transaction_date,
			total_transactions,
			total_volume,
			avg_transaction_size,
			unique_senders,
			unique_receivers,
			completed,
			failed,
			pending,
			avg_processing_time
		FROM daily_transaction_summary
		WHERE transaction_date >= $1 AND transaction_date <= $2
		ORDER BY transaction_date DESC
	`

	rows, err := p.db.QueryContext(ctx, query, startDate, endDate)
	if err != nil {
		return nil, fmt.Errorf("failed to get daily summary: %w", err)
	}
	defer rows.Close()

	var results []map[string]interface{}
	for rows.Next() {
		var (
			date              string
			totalTxns         int64
			totalVolume       float64
			avgSize           float64
			uniqueSenders     int64
			uniqueReceivers   int64
			completed         int64
			failed            int64
			pending           int64
			avgProcessingTime float64
		)

		err := rows.Scan(
			&date, &totalTxns, &totalVolume, &avgSize,
			&uniqueSenders, &uniqueReceivers, &completed, &failed, &pending,
			&avgProcessingTime,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan row: %w", err)
		}

		results = append(results, map[string]interface{}{
			"date":                 date,
			"total_transactions":   totalTxns,
			"total_volume":         totalVolume,
			"avg_transaction_size": avgSize,
			"unique_senders":       uniqueSenders,
			"unique_receivers":     uniqueReceivers,
			"completed":            completed,
			"failed":               failed,
			"pending":              pending,
			"avg_processing_time":  avgProcessingTime,
		})
	}

	return results, nil
}

// Close closes the database connection
func (p *PostgresStorage) Close() error {
	return p.db.Close()
}
