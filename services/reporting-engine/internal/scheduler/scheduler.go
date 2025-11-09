package scheduler

import (
	"context"
	"fmt"
	"time"

	"github.com/deltran/reporting-engine/internal/storage"
	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/robfig/cron/v3"
	"go.uber.org/zap"
)

type ReportGenerator interface {
	Generate(ctx context.Context, req types.ReportRequest) (*types.GenerationResult, error)
}

type ReportScheduler struct {
	cron       *cron.Cron
	generators map[string]ReportGenerator
	storage    *storage.Storage
	logger     *zap.Logger
	semaphore  chan struct{} // Limit concurrent report generation
}

func NewReportScheduler(
	generators map[string]ReportGenerator,
	storage *storage.Storage,
	maxConcurrent int,
	logger *zap.Logger,
) *ReportScheduler {
	location, _ := time.LoadLocation("UTC")

	return &ReportScheduler{
		cron:       cron.New(cron.WithLocation(location), cron.WithSeconds()),
		generators: generators,
		storage:    storage,
		logger:     logger,
		semaphore:  make(chan struct{}, maxConcurrent),
	}
}

func (s *ReportScheduler) Start(ctx context.Context) error {
	// Daily reports at 00:30 UTC
	_, err := s.cron.AddFunc("0 30 0 * * *", func() {
		s.generateDailyReports(ctx)
	})
	if err != nil {
		return fmt.Errorf("failed to schedule daily reports: %w", err)
	}

	// Weekly reports on Monday at 01:00 UTC
	_, err = s.cron.AddFunc("0 0 1 * * MON", func() {
		s.generateWeeklyReports(ctx)
	})
	if err != nil {
		return fmt.Errorf("failed to schedule weekly reports: %w", err)
	}

	// Monthly reports on 1st day at 02:00 UTC
	_, err = s.cron.AddFunc("0 0 2 1 * *", func() {
		s.generateMonthlyReports(ctx)
	})
	if err != nil {
		return fmt.Errorf("failed to schedule monthly reports: %w", err)
	}

	// Quarterly reports on 1st day of quarter at 03:00 UTC
	_, err = s.cron.AddFunc("0 0 3 1 1,4,7,10 *", func() {
		s.generateQuarterlyReports(ctx)
	})
	if err != nil {
		return fmt.Errorf("failed to schedule quarterly reports: %w", err)
	}

	// Real-time metrics refresh every 5 minutes
	_, err = s.cron.AddFunc("0 */5 * * * *", func() {
		s.refreshMetrics(ctx)
	})
	if err != nil {
		return fmt.Errorf("failed to schedule metrics refresh: %w", err)
	}

	// Materialized view refresh every hour
	_, err = s.cron.AddFunc("0 0 * * * *", func() {
		s.refreshMaterializedViews(ctx)
	})
	if err != nil {
		return fmt.Errorf("failed to schedule view refresh: %w", err)
	}

	s.cron.Start()
	s.logger.Info("Report scheduler started", zap.Int("jobs", len(s.cron.Entries())))

	return nil
}

func (s *ReportScheduler) Stop() {
	s.cron.Stop()
	s.logger.Info("Report scheduler stopped")
}

func (s *ReportScheduler) generateDailyReports(ctx context.Context) {
	s.logger.Info("Starting daily report generation")

	yesterday := time.Now().AddDate(0, 0, -1)
	startOfDay := time.Date(yesterday.Year(), yesterday.Month(), yesterday.Day(), 0, 0, 0, 0, time.UTC)
	endOfDay := startOfDay.Add(24 * time.Hour)

	reports := []struct {
		name       string
		reportType string
		format     string
		recipients []string
	}{
		{
			name:       "Daily AML Compliance Report",
			reportType: types.ReportTypeAML,
			format:     types.FormatExcel,
			recipients: []string{"compliance@deltran.com", "risk@deltran.com"},
		},
		{
			name:       "Daily Settlement Report",
			reportType: types.ReportTypeSettlement,
			format:     types.FormatExcel,
			recipients: []string{"settlements@deltran.com", "cfo@deltran.com"},
		},
		{
			name:       "Daily Transaction Summary (CSV)",
			reportType: types.ReportTypeOperational,
			format:     types.FormatCSV,
			recipients: []string{"operations@deltran.com"},
		},
	}

	for _, report := range reports {
		go s.generateAndDistribute(ctx, types.ReportRequest{
			Type:        report.reportType,
			Format:      report.format,
			PeriodStart: startOfDay,
			PeriodEnd:   endOfDay,
			Parameters: map[string]string{
				"name":       report.name,
				"recipients": fmt.Sprintf("%v", report.recipients),
			},
			RequestedBy: "scheduler",
		})
	}
}

func (s *ReportScheduler) generateWeeklyReports(ctx context.Context) {
	s.logger.Info("Starting weekly report generation")

	now := time.Now()
	startOfWeek := now.AddDate(0, 0, -7)
	endOfWeek := now

	reports := []struct {
		name       string
		reportType string
		format     string
	}{
		{
			name:       "Weekly Reconciliation Report",
			reportType: types.ReportTypeReconciliation,
			format:     types.FormatExcel,
		},
		{
			name:       "Weekly Risk Summary",
			reportType: types.ReportTypeAML,
			format:     types.FormatExcel,
		},
	}

	for _, report := range reports {
		go s.generateAndDistribute(ctx, types.ReportRequest{
			Type:        report.reportType,
			Format:      report.format,
			PeriodStart: startOfWeek,
			PeriodEnd:   endOfWeek,
			Parameters: map[string]string{
				"name": report.name,
			},
			RequestedBy: "scheduler",
		})
	}
}

func (s *ReportScheduler) generateMonthlyReports(ctx context.Context) {
	s.logger.Info("Starting monthly report generation")

	now := time.Now()
	startOfMonth := time.Date(now.Year(), now.Month()-1, 1, 0, 0, 0, 0, time.UTC)
	endOfMonth := time.Date(now.Year(), now.Month(), 1, 0, 0, 0, 0, time.UTC)

	reports := []struct {
		name       string
		reportType string
		format     string
	}{
		{
			name:       "Monthly Compliance Report",
			reportType: types.ReportTypeAML,
			format:     types.FormatExcel,
		},
		{
			name:       "Monthly Settlement Analysis",
			reportType: types.ReportTypeSettlement,
			format:     types.FormatExcel,
		},
		{
			name:       "Monthly Operational Report",
			reportType: types.ReportTypeOperational,
			format:     types.FormatExcel,
		},
	}

	for _, report := range reports {
		go s.generateAndDistribute(ctx, types.ReportRequest{
			Type:        report.reportType,
			Format:      report.format,
			PeriodStart: startOfMonth,
			PeriodEnd:   endOfMonth,
			Parameters: map[string]string{
				"name": report.name,
			},
			RequestedBy: "scheduler",
		})
	}
}

func (s *ReportScheduler) generateQuarterlyReports(ctx context.Context) {
	s.logger.Info("Starting quarterly report generation")

	now := time.Now()
	quarter := (int(now.Month()) - 1) / 3
	startOfQuarter := time.Date(now.Year(), time.Month(quarter*3+1), 1, 0, 0, 0, 0, time.UTC)
	endOfQuarter := startOfQuarter.AddDate(0, 3, 0)

	// Quarterly reports are critical for Big 4 audits
	go s.generateAndDistribute(ctx, types.ReportRequest{
		Type:        types.ReportTypeAML,
		Format:      types.FormatExcel,
		PeriodStart: startOfQuarter,
		PeriodEnd:   endOfQuarter,
		Parameters: map[string]string{
			"name":       "Quarterly Compliance Report for Auditors",
			"recipients": "compliance@deltran.com,auditors@deltran.com",
			"big4_format": "true",
		},
		RequestedBy: "scheduler",
	})
}

func (s *ReportScheduler) generateAndDistribute(
	ctx context.Context,
	req types.ReportRequest,
) {
	// Acquire semaphore to limit concurrent generation
	s.semaphore <- struct{}{}
	defer func() { <-s.semaphore }()

	startTime := time.Now()

	generator, ok := s.generators[req.Type]
	if !ok {
		s.logger.Error("Generator not found", zap.String("type", req.Type))
		return
	}

	// Generate report
	result, err := generator.Generate(ctx, req)
	if err != nil {
		s.logger.Error("Failed to generate report",
			zap.String("type", req.Type),
			zap.Error(err))
		return
	}

	// Upload to S3
	storagePath, err := s.storage.UploadReport(ctx, result)
	if err != nil {
		s.logger.Error("Failed to upload report",
			zap.String("report_id", result.ReportID),
			zap.Error(err))
		return
	}

	// Save metadata to database
	report := &types.Report{
		ID:          result.ReportID,
		Type:        req.Type,
		Name:        req.Parameters["name"],
		PeriodStart: req.PeriodStart,
		PeriodEnd:   req.PeriodEnd,
		GeneratedAt: result.GeneratedAt,
		GeneratedBy: req.RequestedBy,
		Status:      types.StatusCompleted,
		StoragePath: storagePath,
		FileSize:    result.FileSize,
		Format:      result.Format,
		Metadata:    req.Parameters,
	}

	if err := s.storage.SaveReportMetadata(ctx, report); err != nil {
		s.logger.Error("Failed to save report metadata",
			zap.String("report_id", result.ReportID),
			zap.Error(err))
	}

	duration := time.Since(startTime)
	s.logger.Info("Report generated and distributed successfully",
		zap.String("report_id", result.ReportID),
		zap.String("type", req.Type),
		zap.String("format", result.Format),
		zap.Int64("size_bytes", result.FileSize),
		zap.Duration("duration", duration))

	// TODO: Send email notifications to recipients
	// This would integrate with notification-engine
}

func (s *ReportScheduler) refreshMetrics(ctx context.Context) {
	// Refresh cached metrics for real-time dashboards
	if err := s.storage.RefreshCachedMetrics(ctx); err != nil {
		s.logger.Error("Failed to refresh metrics cache", zap.Error(err))
	}
}

func (s *ReportScheduler) refreshMaterializedViews(ctx context.Context) {
	s.logger.Info("Refreshing materialized views")

	if err := s.storage.RefreshMaterializedViews(ctx); err != nil {
		s.logger.Error("Failed to refresh materialized views", zap.Error(err))
		return
	}

	s.logger.Info("Materialized views refreshed successfully")
}

// AddCustomSchedule allows adding custom report schedules at runtime
func (s *ReportScheduler) AddCustomSchedule(
	cronExpr string,
	reportType string,
	format string,
	periodFunc func() (time.Time, time.Time),
) (cron.EntryID, error) {
	return s.cron.AddFunc(cronExpr, func() {
		start, end := periodFunc()
		s.generateAndDistribute(context.Background(), types.ReportRequest{
			Type:        reportType,
			Format:      format,
			PeriodStart: start,
			PeriodEnd:   end,
			RequestedBy: "custom_schedule",
		})
	})
}

// RemoveSchedule removes a scheduled job
func (s *ReportScheduler) RemoveSchedule(id cron.EntryID) {
	s.cron.Remove(id)
}

// ListSchedules returns all active schedules
func (s *ReportScheduler) ListSchedules() []cron.Entry {
	return s.cron.Entries()
}
