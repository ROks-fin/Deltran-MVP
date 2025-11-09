package storage

import (
	"context"
	"time"

	"github.com/deltran/reporting-engine/internal/config"
	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/go-redis/redis/v8"
	"go.uber.org/zap"
)

// Storage provides unified access to all storage backends
type Storage struct {
	postgres *PostgresStorage
	s3       *S3Storage
	redis    *redis.Client
	logger   *zap.Logger
}

func NewStorage(cfg *config.Config, logger *zap.Logger) (*Storage, error) {
	// Initialize PostgreSQL
	postgres, err := NewPostgresStorage(cfg.DatabaseDSN(), logger)
	if err != nil {
		return nil, err
	}

	// Initialize S3
	s3Storage, err := NewS3Storage(&cfg.S3, logger)
	if err != nil {
		return nil, err
	}

	// Initialize Redis
	redisClient := redis.NewClient(&redis.Options{
		Addr:     cfg.Redis.Address,
		Password: cfg.Redis.Password,
		DB:       cfg.Redis.DB,
	})

	// Test Redis connection
	if err := redisClient.Ping(context.Background()).Err(); err != nil {
		logger.Warn("Redis connection failed, continuing without cache", zap.Error(err))
	}

	return &Storage{
		postgres: postgres,
		s3:       s3Storage,
		redis:    redisClient,
		logger:   logger,
	}, nil
}

// UploadReport uploads report to S3
func (s *Storage) UploadReport(ctx context.Context, result *types.GenerationResult) (string, error) {
	return s.s3.UploadReport(ctx, result)
}

// DownloadReport downloads report from S3
func (s *Storage) DownloadReport(ctx context.Context, storagePath string) ([]byte, error) {
	return s.s3.DownloadReport(ctx, storagePath)
}

// GetPresignedURL generates presigned URL for download
func (s *Storage) GetPresignedURL(ctx context.Context, storagePath string, expiry time.Duration) (string, error) {
	return s.s3.GetPresignedURL(ctx, storagePath, expiry)
}

// SaveReportMetadata saves report metadata to PostgreSQL
func (s *Storage) SaveReportMetadata(ctx context.Context, report *types.Report) error {
	return s.postgres.SaveReportMetadata(ctx, report)
}

// GetReport retrieves report metadata
func (s *Storage) GetReport(ctx context.Context, reportID string) (*types.Report, error) {
	return s.postgres.GetReport(ctx, reportID)
}

// ListReports lists reports with filters
func (s *Storage) ListReports(ctx context.Context, reportType string, limit, offset int) ([]types.Report, error) {
	return s.postgres.ListReports(ctx, reportType, limit, offset)
}

// DeleteReport deletes report from both S3 and database
func (s *Storage) DeleteReport(ctx context.Context, reportID string) error {
	// Get report metadata to find storage path
	report, err := s.postgres.GetReport(ctx, reportID)
	if err != nil {
		return err
	}

	// Delete from S3
	if report.StoragePath != "" {
		if err := s.s3.DeleteReport(ctx, report.StoragePath); err != nil {
			s.logger.Warn("Failed to delete from S3", zap.Error(err))
		}
	}

	// Mark as deleted in database
	return s.postgres.DeleteReport(ctx, reportID)
}

// LogReportAccess logs access for audit trail
func (s *Storage) LogReportAccess(ctx context.Context, reportID, accessedBy, accessType, ipAddress, userAgent string) error {
	return s.postgres.LogReportAccess(ctx, reportID, accessedBy, accessType, ipAddress, userAgent)
}

// RefreshMaterializedViews refreshes all materialized views
func (s *Storage) RefreshMaterializedViews(ctx context.Context) error {
	return s.postgres.RefreshMaterializedViews(ctx)
}

// RefreshCachedMetrics refreshes cached metrics in Redis
func (s *Storage) RefreshCachedMetrics(ctx context.Context) error {
	// Get latest metrics from materialized views
	metrics, err := s.postgres.GetDailyTransactionSummary(ctx,
		time.Now().AddDate(0, 0, -7).Format("2006-01-02"),
		time.Now().Format("2006-01-02"))
	if err != nil {
		return err
	}

	// Cache in Redis
	for _, metric := range metrics {
		key := "metrics:daily:" + metric["date"].(string)
		if err := s.redis.Set(ctx, key, metric, 1*time.Hour).Err(); err != nil {
			s.logger.Warn("Failed to cache metric", zap.Error(err))
		}
	}

	return nil
}

// GetCachedMetrics retrieves cached metrics from Redis
func (s *Storage) GetCachedMetrics(ctx context.Context, date string) (map[string]interface{}, error) {
	key := "metrics:daily:" + date

	var metrics map[string]interface{}
	err := s.redis.Get(ctx, key).Scan(&metrics)
	if err == redis.Nil {
		// Not in cache, fetch from database
		results, err := s.postgres.GetDailyTransactionSummary(ctx, date, date)
		if err != nil || len(results) == 0 {
			return nil, err
		}
		return results[0], nil
	}
	if err != nil {
		return nil, err
	}

	return metrics, nil
}

// Close closes all storage connections
func (s *Storage) Close() error {
	if err := s.postgres.Close(); err != nil {
		return err
	}
	if err := s.redis.Close(); err != nil {
		return err
	}
	return nil
}
