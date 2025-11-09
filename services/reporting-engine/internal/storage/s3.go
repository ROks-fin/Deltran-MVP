package storage

import (
	"bytes"
	"context"
	"fmt"
	"time"

	"github.com/aws/aws-sdk-go/aws"
	"github.com/aws/aws-sdk-go/aws/credentials"
	"github.com/aws/aws-sdk-go/aws/session"
	"github.com/aws/aws-sdk-go/service/s3"
	"github.com/aws/aws-sdk-go/service/s3/s3manager"
	"github.com/deltran/reporting-engine/internal/config"
	"github.com/deltran/reporting-engine/pkg/types"
	"go.uber.org/zap"
)

type S3Storage struct {
	client   *s3.S3
	uploader *s3manager.Uploader
	bucket   string
	logger   *zap.Logger
}

func NewS3Storage(cfg *config.S3Config, logger *zap.Logger) (*S3Storage, error) {
	sess, err := session.NewSession(&aws.Config{
		Endpoint:         aws.String(cfg.Endpoint),
		Region:           aws.String(cfg.Region),
		Credentials:      credentials.NewStaticCredentials(cfg.AccessKey, cfg.SecretKey, ""),
		S3ForcePathStyle: aws.Bool(true),
		DisableSSL:       aws.Bool(!cfg.UseSSL),
	})
	if err != nil {
		return nil, fmt.Errorf("failed to create AWS session: %w", err)
	}

	client := s3.New(sess)
	uploader := s3manager.NewUploader(sess)

	// Ensure bucket exists
	_, err = client.HeadBucket(&s3.HeadBucketInput{
		Bucket: aws.String(cfg.Bucket),
	})
	if err != nil {
		// Try to create bucket
		_, err = client.CreateBucket(&s3.CreateBucketInput{
			Bucket: aws.String(cfg.Bucket),
		})
		if err != nil {
			logger.Warn("Failed to create bucket, it may already exist", zap.Error(err))
		}
	}

	return &S3Storage{
		client:   client,
		uploader: uploader,
		bucket:   cfg.Bucket,
		logger:   logger,
	}, nil
}

// UploadReport uploads a generated report to S3
func (s *S3Storage) UploadReport(
	ctx context.Context,
	result *types.GenerationResult,
) (string, error) {
	// Generate S3 key with organized structure
	// reports/2025/01/07/aml/report_uuid.xlsx
	key := fmt.Sprintf("reports/%s/%s/%s.%s",
		result.GeneratedAt.Format("2006/01/02"),
		result.ReportID[:8],
		result.ReportID,
		result.Format)

	// Upload file
	uploadResult, err := s.uploader.UploadWithContext(ctx, &s3manager.UploadInput{
		Bucket:      aws.String(s.bucket),
		Key:         aws.String(key),
		Body:        bytes.NewReader(result.Data),
		ContentType: aws.String(s.getContentType(result.Format)),
		Metadata: map[string]*string{
			"report-id":   aws.String(result.ReportID),
			"generated-at": aws.String(result.GeneratedAt.Format(time.RFC3339)),
			"format":      aws.String(result.Format),
		},
	})
	if err != nil {
		return "", fmt.Errorf("failed to upload to S3: %w", err)
	}

	s.logger.Info("Report uploaded to S3",
		zap.String("report_id", result.ReportID),
		zap.String("location", uploadResult.Location),
		zap.String("key", key))

	return key, nil
}

// GetPresignedURL generates a pre-signed URL for report download
func (s *S3Storage) GetPresignedURL(
	ctx context.Context,
	storagePath string,
	expiryDuration time.Duration,
) (string, error) {
	req, _ := s.client.GetObjectRequest(&s3.GetObjectInput{
		Bucket: aws.String(s.bucket),
		Key:    aws.String(storagePath),
	})

	url, err := req.Presign(expiryDuration)
	if err != nil {
		return "", fmt.Errorf("failed to generate presigned URL: %w", err)
	}

	return url, nil
}

// DownloadReport downloads a report from S3
func (s *S3Storage) DownloadReport(
	ctx context.Context,
	storagePath string,
) ([]byte, error) {
	buffer := aws.NewWriteAtBuffer([]byte{})

	downloader := s3manager.NewDownloaderWithClient(s.client)
	_, err := downloader.DownloadWithContext(ctx, buffer, &s3.GetObjectInput{
		Bucket: aws.String(s.bucket),
		Key:    aws.String(storagePath),
	})
	if err != nil {
		return nil, fmt.Errorf("failed to download from S3: %w", err)
	}

	return buffer.Bytes(), nil
}

// DeleteReport deletes a report from S3
func (s *S3Storage) DeleteReport(
	ctx context.Context,
	storagePath string,
) error {
	_, err := s.client.DeleteObjectWithContext(ctx, &s3.DeleteObjectInput{
		Bucket: aws.String(s.bucket),
		Key:    aws.String(storagePath),
	})
	if err != nil {
		return fmt.Errorf("failed to delete from S3: %w", err)
	}

	s.logger.Info("Report deleted from S3", zap.String("key", storagePath))
	return nil
}

// ListReports lists all reports in a given date range
func (s *S3Storage) ListReports(
	ctx context.Context,
	startDate, endDate time.Time,
) ([]string, error) {
	prefix := fmt.Sprintf("reports/%s/", startDate.Format("2006/01"))

	input := &s3.ListObjectsV2Input{
		Bucket: aws.String(s.bucket),
		Prefix: aws.String(prefix),
	}

	var keys []string
	err := s.client.ListObjectsV2PagesWithContext(ctx, input,
		func(page *s3.ListObjectsV2Output, lastPage bool) bool {
			for _, obj := range page.Contents {
				keys = append(keys, *obj.Key)
			}
			return true
		})
	if err != nil {
		return nil, fmt.Errorf("failed to list objects: %w", err)
	}

	return keys, nil
}

// CleanupOldReports deletes reports older than retention period
func (s *S3Storage) CleanupOldReports(
	ctx context.Context,
	retentionDays int,
) (int, error) {
	cutoffDate := time.Now().AddDate(0, 0, -retentionDays)

	input := &s3.ListObjectsV2Input{
		Bucket: aws.String(s.bucket),
		Prefix: aws.String("reports/"),
	}

	deletedCount := 0
	err := s.client.ListObjectsV2PagesWithContext(ctx, input,
		func(page *s3.ListObjectsV2Output, lastPage bool) bool {
			for _, obj := range page.Contents {
				if obj.LastModified.Before(cutoffDate) {
					if err := s.DeleteReport(ctx, *obj.Key); err != nil {
						s.logger.Error("Failed to delete old report",
							zap.String("key", *obj.Key),
							zap.Error(err))
						continue
					}
					deletedCount++
				}
			}
			return true
		})
	if err != nil {
		return deletedCount, fmt.Errorf("failed to cleanup old reports: %w", err)
	}

	s.logger.Info("Old reports cleaned up",
		zap.Int("count", deletedCount),
		zap.Int("retention_days", retentionDays))

	return deletedCount, nil
}

func (s *S3Storage) getContentType(format string) string {
	switch format {
	case types.FormatExcel:
		return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
	case types.FormatCSV:
		return "text/csv"
	case types.FormatPDF:
		return "application/pdf"
	default:
		return "application/octet-stream"
	}
}
