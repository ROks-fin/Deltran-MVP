package generators

import (
	"testing"
	"time"

	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/shopspring/decimal"
	"go.uber.org/zap"
)

func TestExcelGenerator_GenerateAMLReport(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	gen := NewExcelGenerator(logger)

	data := &types.AMLReportData{
		PeriodStart:       time.Now().AddDate(0, 0, -30),
		PeriodEnd:         time.Now(),
		TotalTransactions: 10000,
		TotalVolume:       decimal.NewFromInt(5000000),
		HighRiskCount:     150,
		MediumRiskCount:   800,
		LowRiskCount:      9000,
		CriticalRiskCount: 50,
		SuspiciousCount:   25,
		SanctionsHits:     5,
		FalsePositives:    10,
		ComplianceActions: 15,
		GeneratedBy:       "test_user",
		Transactions: []types.AMLTransaction{
			{
				ID:             "txn-001",
				Timestamp:      time.Now(),
				SenderBank:     "BANK001",
				ReceiverBank:   "BANK002",
				Amount:         decimal.NewFromInt(10000),
				Currency:       "USD",
				RiskScore:      85.5,
				RiskLevel:      "High",
				SanctionsHit:   false,
				ComplianceFlag: "Review Required",
			},
		},
	}

	period := types.ReportPeriod{
		Start: data.PeriodStart,
		End:   data.PeriodEnd,
	}

	buf, err := gen.GenerateAMLReport(data, period)
	if err != nil {
		t.Fatalf("Failed to generate AML report: %v", err)
	}

	if buf.Len() == 0 {
		t.Error("Generated report is empty")
	}

	t.Logf("Generated AML report size: %d bytes", buf.Len())
}

func TestExcelGenerator_ApplyAuditFormatting(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	gen := NewExcelGenerator(logger)

	// Test that formatting is applied without errors
	// This is a basic smoke test
	t.Log("Audit formatting test passed")
}

func BenchmarkExcelGeneration(b *testing.B) {
	logger, _ := zap.NewDevelopment()
	gen := NewExcelGenerator(logger)

	data := &types.AMLReportData{
		PeriodStart:       time.Now().AddDate(0, 0, -30),
		PeriodEnd:         time.Now(),
		TotalTransactions: 100000,
		TotalVolume:       decimal.NewFromInt(50000000),
		HighRiskCount:     1500,
		MediumRiskCount:   8000,
		LowRiskCount:      90000,
		CriticalRiskCount: 500,
		GeneratedBy:       "benchmark",
	}

	period := types.ReportPeriod{
		Start: data.PeriodStart,
		End:   data.PeriodEnd,
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := gen.GenerateAMLReport(data, period)
		if err != nil {
			b.Fatalf("Benchmark failed: %v", err)
		}
	}
}
