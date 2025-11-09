package reports

import (
	"context"
	"database/sql"
	"fmt"
	"time"

	"github.com/deltran/reporting-engine/internal/generators"
	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/google/uuid"
	"go.uber.org/zap"
)

type SettlementReportGenerator struct {
	db       *sql.DB
	excelGen *generators.ExcelGenerator
	csvGen   *generators.CSVGenerator
	logger   *zap.Logger
}

func NewSettlementReportGenerator(
	db *sql.DB,
	excelGen *generators.ExcelGenerator,
	csvGen *generators.CSVGenerator,
	logger *zap.Logger,
) *SettlementReportGenerator {
	return &SettlementReportGenerator{
		db:       db,
		excelGen: excelGen,
		csvGen:   csvGen,
		logger:   logger,
	}
}

func (g *SettlementReportGenerator) Generate(
	ctx context.Context,
	req types.ReportRequest,
) (*types.GenerationResult, error) {
	g.logger.Info("Generating Settlement report")

	reportID := uuid.New().String()
	var reportData []byte
	var format string

	switch req.Format {
	case types.FormatCSV:
		buf, err := g.csvGen.GenerateCSVBuffer(ctx, types.ReportTypeSettlement, req.PeriodStart, req.PeriodEnd)
		if err != nil {
			return nil, err
		}
		reportData = buf
		format = types.FormatCSV

	default:
		return nil, fmt.Errorf("settlement report Excel format not yet implemented")
	}

	return &types.GenerationResult{
		ReportID:    reportID,
		Data:        reportData,
		Format:      format,
		FileSize:    int64(len(reportData)),
		GeneratedAt: time.Now(),
	}, nil
}
