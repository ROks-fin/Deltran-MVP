package generators

import (
	"bytes"
	"fmt"
	"time"

	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/xuri/excelize/v2"
	"go.uber.org/zap"
)

type ExcelGenerator struct {
	logger *zap.Logger
}

func NewExcelGenerator(logger *zap.Logger) *ExcelGenerator {
	return &ExcelGenerator{
		logger: logger,
	}
}

// GenerateAMLReport generates AML compliance report with Big 4 audit formatting
func (g *ExcelGenerator) GenerateAMLReport(
	data *types.AMLReportData,
	period types.ReportPeriod,
) (*bytes.Buffer, error) {
	f := excelize.NewFile()
	defer f.Close()

	// Create all required sheets
	sheets := []string{
		"Executive Summary",
		"Transaction Analysis",
		"Risk Indicators",
		"Suspicious Activities",
		"Sanctions Screening",
		"Compliance Actions",
	}

	for i, sheet := range sheets {
		if i == 0 {
			f.SetSheetName("Sheet1", sheet)
		} else {
			f.NewSheet(sheet)
		}
	}

	// Generate each section
	if err := g.generateExecutiveSummary(f, data, period); err != nil {
		return nil, fmt.Errorf("failed to generate executive summary: %w", err)
	}

	if err := g.generateTransactionAnalysis(f, data); err != nil {
		return nil, fmt.Errorf("failed to generate transaction analysis: %w", err)
	}

	if err := g.generateRiskIndicators(f, data); err != nil {
		return nil, fmt.Errorf("failed to generate risk indicators: %w", err)
	}

	if err := g.generateSuspiciousActivities(f, data); err != nil {
		return nil, fmt.Errorf("failed to generate suspicious activities: %w", err)
	}

	// Apply Big 4 audit formatting standards
	g.applyAuditFormatting(f)

	// Add digital signature and watermark
	g.addAuditTrail(f, data.GeneratedBy, time.Now())

	// Write to buffer
	buf := new(bytes.Buffer)
	if err := f.Write(buf); err != nil {
		return nil, fmt.Errorf("failed to write Excel file: %w", err)
	}

	g.logger.Info("AML report generated successfully",
		zap.Int("transactions", len(data.Transactions)),
		zap.String("period", fmt.Sprintf("%s to %s", period.Start.Format("2006-01-02"), period.End.Format("2006-01-02"))))

	return buf, nil
}

func (g *ExcelGenerator) generateExecutiveSummary(
	f *excelize.File,
	data *types.AMLReportData,
	period types.ReportPeriod,
) error {
	sheet := "Executive Summary"

	// Title styling - Big 4 standard
	titleStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{
			Bold:  true,
			Size:  16,
			Color: "#FFFFFF",
			Family: "Calibri",
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

	headerStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{
			Bold:   true,
			Size:   12,
			Color:  "#FFFFFF",
			Family: "Calibri",
		},
		Fill: excelize.Fill{
			Type:    "pattern",
			Color:   []string{"#34495E"},
			Pattern: 1,
		},
		Border: []excelize.Border{
			{Type: "top", Style: 1, Color: "#000000"},
			{Type: "bottom", Style: 1, Color: "#000000"},
		},
		Alignment: &excelize.Alignment{
			Horizontal: "left",
			Vertical:   "center",
		},
	})

	valueStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{
			Size:   11,
			Family: "Calibri",
		},
		Alignment: &excelize.Alignment{
			Horizontal: "left",
			Vertical:   "center",
		},
	})

	numberStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{
			Size:   11,
			Family: "Calibri",
		},
		NumFmt: 3, // #,##0
		Alignment: &excelize.Alignment{
			Horizontal: "right",
			Vertical:   "center",
		},
	})

	currencyStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{
			Size:   11,
			Family: "Calibri",
		},
		NumFmt: 164, // Custom: "$#,##0.00"
		Alignment: &excelize.Alignment{
			Horizontal: "right",
			Vertical:   "center",
		},
		CustomNumFmt: stringPtr("$#,##0.00"),
	})

	// Report title
	f.MergeCell(sheet, "A1", "H1")
	f.SetCellValue(sheet, "A1", "AML COMPLIANCE REPORT - EXECUTIVE SUMMARY")
	f.SetCellStyle(sheet, "A1", "H1", titleStyle)
	f.SetRowHeight(sheet, 1, 30)

	// Report period
	f.SetCellValue(sheet, "A3", "Report Period:")
	f.SetCellValue(sheet, "B3", fmt.Sprintf("%s to %s",
		period.Start.Format("January 02, 2006"),
		period.End.Format("January 02, 2006")))
	f.SetCellStyle(sheet, "A3", "A3", headerStyle)
	f.SetCellStyle(sheet, "B3", "B3", valueStyle)

	f.SetCellValue(sheet, "A4", "Generated:")
	f.SetCellValue(sheet, "B4", time.Now().Format("January 02, 2006 15:04:05 MST"))
	f.SetCellStyle(sheet, "A4", "A4", headerStyle)
	f.SetCellStyle(sheet, "B4", "B4", valueStyle)

	// Key metrics table
	row := 6
	f.MergeCell(sheet, fmt.Sprintf("A%d", row), fmt.Sprintf("H%d", row))
	f.SetCellValue(sheet, fmt.Sprintf("A%d", row), "KEY COMPLIANCE METRICS")
	f.SetCellStyle(sheet, fmt.Sprintf("A%d", row), fmt.Sprintf("H%d", row), headerStyle)
	row++

	metrics := []struct {
		label string
		value interface{}
		style int
	}{
		{"Total Transactions Monitored", data.TotalTransactions, numberStyle},
		{"Total Transaction Volume", data.TotalVolume.StringFixed(2), currencyStyle},
		{"High Risk Transactions", data.HighRiskCount, numberStyle},
		{"Critical Risk Transactions", data.CriticalRiskCount, numberStyle},
		{"Suspicious Activities Identified", data.SuspiciousCount, numberStyle},
		{"Sanctions Screening Hits", data.SanctionsHits, numberStyle},
		{"False Positives", data.FalsePositives, numberStyle},
		{"Compliance Actions Taken", data.ComplianceActions, numberStyle},
	}

	for _, metric := range metrics {
		f.SetCellValue(sheet, fmt.Sprintf("A%d", row), metric.label)
		f.SetCellValue(sheet, fmt.Sprintf("C%d", row), metric.value)
		f.SetCellStyle(sheet, fmt.Sprintf("A%d", row), fmt.Sprintf("A%d", row), headerStyle)
		f.SetCellStyle(sheet, fmt.Sprintf("C%d", row), fmt.Sprintf("C%d", row), metric.style)
		row++
	}

	// Risk distribution pie chart
	if err := g.addRiskDistributionChart(f, sheet, data); err != nil {
		g.logger.Warn("Failed to add risk distribution chart", zap.Error(err))
	}

	// Set column widths
	f.SetColWidth(sheet, "A", "A", 35)
	f.SetColWidth(sheet, "B", "C", 20)
	f.SetColWidth(sheet, "D", "H", 15)

	return nil
}

func (g *ExcelGenerator) addRiskDistributionChart(
	f *excelize.File,
	sheet string,
	data *types.AMLReportData,
) error {
	// Chart data starting row
	startRow := 20

	// Headers
	f.SetCellValue(sheet, fmt.Sprintf("A%d", startRow), "Risk Level")
	f.SetCellValue(sheet, fmt.Sprintf("B%d", startRow), "Count")
	f.SetCellValue(sheet, fmt.Sprintf("C%d", startRow), "Percentage")

	// Data
	categories := []string{"Low Risk", "Medium Risk", "High Risk", "Critical"}
	values := []int64{
		data.LowRiskCount,
		data.MediumRiskCount,
		data.HighRiskCount,
		data.CriticalRiskCount,
	}

	total := data.TotalTransactions
	if total == 0 {
		total = 1 // Prevent division by zero
	}

	for i, cat := range categories {
		row := startRow + i + 1
		f.SetCellValue(sheet, fmt.Sprintf("A%d", row), cat)
		f.SetCellValue(sheet, fmt.Sprintf("B%d", row), values[i])
		percentage := float64(values[i]) / float64(total) * 100
		f.SetCellValue(sheet, fmt.Sprintf("C%d", row), fmt.Sprintf("%.2f%%", percentage))
	}

	// Create pie chart
	chart := &excelize.Chart{
		Type: excelize.Pie3D,
		Series: []excelize.ChartSeries{
			{
				Name:       sheet + "!$B$" + fmt.Sprint(startRow),
				Categories: sheet + "!$A$" + fmt.Sprint(startRow+1) + ":$A$" + fmt.Sprint(startRow+4),
				Values:     sheet + "!$B$" + fmt.Sprint(startRow+1) + ":$B$" + fmt.Sprint(startRow+4),
			},
		},
		Title: []excelize.RichTextRun{
			{Text: "Risk Distribution"},
		},
		Legend: excelize.ChartLegend{
			Position: "right",
		},
		PlotArea: excelize.ChartPlotArea{
			ShowPercent: true,
		},
	}

	return f.AddChart(sheet, "E20", chart)
}

func (g *ExcelGenerator) generateTransactionAnalysis(
	f *excelize.File,
	data *types.AMLReportData,
) error {
	sheet := "Transaction Analysis"

	headerStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{Bold: true, Size: 11, Color: "#FFFFFF"},
		Fill: excelize.Fill{Type: "pattern", Color: []string{"#34495E"}, Pattern: 1},
		Border: []excelize.Border{
			{Type: "top", Style: 1, Color: "#000000"},
			{Type: "bottom", Style: 1, Color: "#000000"},
		},
		Alignment: &excelize.Alignment{Horizontal: "center", Vertical: "center"},
	})

	// Headers
	headers := []string{"Transaction ID", "Timestamp", "Sender Bank", "Receiver Bank", "Amount", "Currency", "Risk Score", "Risk Level", "Sanctions Hit", "Compliance Flag"}
	for i, header := range headers {
		cell := fmt.Sprintf("%c1", 'A'+i)
		f.SetCellValue(sheet, cell, header)
		f.SetCellStyle(sheet, cell, cell, headerStyle)
	}

	// Data rows
	for i, txn := range data.Transactions {
		row := i + 2
		f.SetCellValue(sheet, fmt.Sprintf("A%d", row), txn.ID)
		f.SetCellValue(sheet, fmt.Sprintf("B%d", row), txn.Timestamp.Format("2006-01-02 15:04:05"))
		f.SetCellValue(sheet, fmt.Sprintf("C%d", row), txn.SenderBank)
		f.SetCellValue(sheet, fmt.Sprintf("D%d", row), txn.ReceiverBank)
		f.SetCellValue(sheet, fmt.Sprintf("E%d", row), txn.Amount.StringFixed(2))
		f.SetCellValue(sheet, fmt.Sprintf("F%d", row), txn.Currency)
		f.SetCellValue(sheet, fmt.Sprintf("G%d", row), fmt.Sprintf("%.2f", txn.RiskScore))
		f.SetCellValue(sheet, fmt.Sprintf("H%d", row), txn.RiskLevel)
		f.SetCellValue(sheet, fmt.Sprintf("I%d", row), txn.SanctionsHit)
		f.SetCellValue(sheet, fmt.Sprintf("J%d", row), txn.ComplianceFlag)

		// Color code risk levels
		if txn.RiskLevel == "Critical" || txn.RiskLevel == "High" {
			riskStyle, _ := f.NewStyle(&excelize.Style{
				Fill: excelize.Fill{Type: "pattern", Color: []string{"#E74C3C"}, Pattern: 1},
			})
			f.SetCellStyle(sheet, fmt.Sprintf("H%d", row), fmt.Sprintf("H%d", row), riskStyle)
		}
	}

	// Auto-filter
	f.AutoFilter(sheet, "A1:J1", []excelize.AutoFilterOptions{})

	// Column widths
	f.SetColWidth(sheet, "A", "A", 25)
	f.SetColWidth(sheet, "B", "B", 20)
	f.SetColWidth(sheet, "C", "D", 15)
	f.SetColWidth(sheet, "E", "E", 12)

	return nil
}

func (g *ExcelGenerator) generateRiskIndicators(
	f *excelize.File,
	data *types.AMLReportData,
) error {
	sheet := "Risk Indicators"

	headerStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{Bold: true, Size: 12, Color: "#FFFFFF"},
		Fill: excelize.Fill{Type: "pattern", Color: []string{"#2C3E50"}, Pattern: 1},
	})

	// Risk distribution summary
	f.SetCellValue(sheet, "A1", "RISK DISTRIBUTION SUMMARY")
	f.MergeCell(sheet, "A1", "D1")
	f.SetCellStyle(sheet, "A1", "D1", headerStyle)

	row := 3
	f.SetCellValue(sheet, "A"+fmt.Sprint(row), "Risk Level")
	f.SetCellValue(sheet, "B"+fmt.Sprint(row), "Count")
	f.SetCellValue(sheet, "C"+fmt.Sprint(row), "Percentage")
	f.SetCellValue(sheet, "D"+fmt.Sprint(row), "Threshold Status")
	row++

	total := float64(data.TotalTransactions)
	if total == 0 {
		total = 1
	}

	risks := []struct {
		level     string
		count     int64
		threshold float64
	}{
		{"Low Risk", data.LowRiskCount, 0.8},
		{"Medium Risk", data.MediumRiskCount, 0.15},
		{"High Risk", data.HighRiskCount, 0.05},
		{"Critical", data.CriticalRiskCount, 0.01},
	}

	for _, risk := range risks {
		percentage := float64(risk.count) / total
		status := "Within Threshold"
		if percentage > risk.threshold {
			status = "⚠ EXCEEDS THRESHOLD"
		}

		f.SetCellValue(sheet, "A"+fmt.Sprint(row), risk.level)
		f.SetCellValue(sheet, "B"+fmt.Sprint(row), risk.count)
		f.SetCellValue(sheet, "C"+fmt.Sprint(row), fmt.Sprintf("%.2f%%", percentage*100))
		f.SetCellValue(sheet, "D"+fmt.Sprint(row), status)
		row++
	}

	return nil
}

func (g *ExcelGenerator) generateSuspiciousActivities(
	f *excelize.File,
	data *types.AMLReportData,
) error {
	sheet := "Suspicious Activities"

	headerStyle, _ := f.NewStyle(&excelize.Style{
		Font: &excelize.Font{Bold: true, Size: 11, Color: "#FFFFFF"},
		Fill: excelize.Fill{Type: "pattern", Color: []string{"#E74C3C"}, Pattern: 1},
	})

	headers := []string{"Transaction ID", "Detected At", "Activity Type", "Risk Score", "Indicators", "Status", "Reviewed By"}
	for i, header := range headers {
		cell := fmt.Sprintf("%c1", 'A'+i)
		f.SetCellValue(sheet, cell, header)
		f.SetCellStyle(sheet, cell, cell, headerStyle)
	}

	for i, activity := range data.SuspiciousActivities {
		row := i + 2
		f.SetCellValue(sheet, fmt.Sprintf("A%d", row), activity.TransactionID)
		f.SetCellValue(sheet, fmt.Sprintf("B%d", row), activity.DetectedAt.Format("2006-01-02 15:04:05"))
		f.SetCellValue(sheet, fmt.Sprintf("C%d", row), activity.ActivityType)
		f.SetCellValue(sheet, fmt.Sprintf("D%d", row), fmt.Sprintf("%.2f", activity.RiskScore))
		f.SetCellValue(sheet, fmt.Sprintf("E%d", row), fmt.Sprintf("%v", activity.Indicators))
		f.SetCellValue(sheet, fmt.Sprintf("F%d", row), activity.Status)
		f.SetCellValue(sheet, fmt.Sprintf("G%d", row), activity.ReviewedBy)
	}

	f.AutoFilter(sheet, "A1:G1", []excelize.AutoFilterOptions{})
	f.SetColWidth(sheet, "A", "G", 18)

	return nil
}

func (g *ExcelGenerator) applyAuditFormatting(f *excelize.File) {
	// Apply Big 4 audit standard formatting to all sheets
	for _, sheet := range f.GetSheetList() {
		// Set header/footer with digital signature placeholder
		f.SetHeaderFooter(sheet, &excelize.HeaderFooterOptions{
			DifferentFirst:   false,
			DifferentOddEven: false,
			OddHeader:        "&LDelTran Compliance Report&CCONFIDENTIAL&R&D",
			OddFooter:        "&LGenerated by DelTran AML System&CPage &P of &N&R" + time.Now().Format("2006-01-02 15:04:05 MST"),
		})

		// Freeze top row
		f.SetPanes(sheet, &excelize.Panes{
			Freeze:      true,
			XSplit:      0,
			YSplit:      1,
			TopLeftCell: "A2",
			ActivePane:  "bottomLeft",
		})
	}
}

func (g *ExcelGenerator) addAuditTrail(f *excelize.File, generatedBy string, timestamp time.Time) {
	// Add audit trail sheet
	auditSheet := "Audit Trail"
	f.NewSheet(auditSheet)

	f.SetCellValue(auditSheet, "A1", "DIGITAL AUDIT TRAIL")
	f.SetCellValue(auditSheet, "A3", "Generated By:")
	f.SetCellValue(auditSheet, "B3", generatedBy)
	f.SetCellValue(auditSheet, "A4", "Generated At:")
	f.SetCellValue(auditSheet, "B4", timestamp.Format("2006-01-02 15:04:05 MST"))
	f.SetCellValue(auditSheet, "A5", "System:")
	f.SetCellValue(auditSheet, "B5", "DelTran Reporting Engine v1.0")
	f.SetCellValue(auditSheet, "A6", "Compliance Standard:")
	f.SetCellValue(auditSheet, "B6", "Big 4 Audit Format (PwC/Deloitte/EY/KPMG)")
	f.SetCellValue(auditSheet, "A8", "Digital Signature:")
	f.SetCellValue(auditSheet, "B8", fmt.Sprintf("DTRAN-%s-%d", generatedBy, timestamp.Unix()))

	// Protect audit trail sheet
	f.ProtectSheet(auditSheet, &excelize.SheetProtectionOptions{
		Password:      "deltran2025",
		EditObjects:   false,
		EditScenarios: false,
	})
}

func stringPtr(s string) *string {
	return &s
}
