package audit

import (
	"context"
	"database/sql"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"os"
	"time"

	"github.com/xuri/excelize/v2"
)

// ExportFormat represents the format for audit exports
type ExportFormat string

const (
	FormatCSV   ExportFormat = "csv"
	FormatExcel ExportFormat = "xlsx"
	FormatJSON  ExportFormat = "json"
)

// AuditExporter handles export of audit data for Big Four compliance
type AuditExporter struct {
	db *sql.DB
}

// NewAuditExporter creates a new audit exporter
func NewAuditExporter(db *sql.DB) *AuditExporter {
	return &AuditExporter{db: db}
}

// ExportRequest contains parameters for audit export
type ExportRequest struct {
	ReportType      string       `json:"report_type"` // "audit_trail", "transaction_ledger", "reconciliation"
	StartDate       time.Time    `json:"start_date"`
	EndDate         time.Time    `json:"end_date"`
	EntityType      string       `json:"entity_type,omitempty"`
	ComplianceType  string       `json:"compliance_type,omitempty"` // "SOX", "IFRS-9", "Basel-III"
	Format          ExportFormat `json:"format"`
	IncludeMetadata bool         `json:"include_metadata"`
}

// ExportResponse contains the result of export operation
type ExportResponse struct {
	FilePath      string    `json:"file_path"`
	RecordCount   int       `json:"record_count"`
	GeneratedAt   time.Time `json:"generated_at"`
	ExportedBy    string    `json:"exported_by"`
	ReportType    string    `json:"report_type"`
	ComplianceRef string    `json:"compliance_ref"`
}

// ExportAuditTrail exports complete audit trail for Big Four compliance
func (e *AuditExporter) ExportAuditTrail(ctx context.Context, req ExportRequest) (*ExportResponse, error) {
	query := `
		SELECT
			audit_record_id,
			audit_timestamp,
			audit_type,
			entity_type,
			entity_id,
			actor_email,
			actor_name,
			actor_role,
			actor_ip_address,
			action_description,
			old_values,
			new_values,
			compliance_category,
			regulatory_impact,
			mfa_verified,
			signed_off_by,
			signed_off_at,
			metadata
		FROM deltran.v_big_four_audit_export
		WHERE audit_timestamp BETWEEN $1 AND $2
	`

	args := []interface{}{req.StartDate, req.EndDate}

	if req.EntityType != "" {
		query += " AND entity_type = $3"
		args = append(args, req.EntityType)
	}

	if req.ComplianceType != "" {
		if req.EntityType != "" {
			query += " AND compliance_category = $4"
		} else {
			query += " AND compliance_category = $3"
		}
		args = append(args, req.ComplianceType)
	}

	query += " ORDER BY audit_timestamp DESC"

	rows, err := e.db.QueryContext(ctx, query, args...)
	if err != nil {
		return nil, fmt.Errorf("failed to query audit trail: %w", err)
	}
	defer rows.Close()

	var records []map[string]interface{}
	for rows.Next() {
		var (
			auditRecordID     int64
			auditTimestamp    time.Time
			auditType         string
			entityType        string
			entityID          string
			actorEmail        sql.NullString
			actorName         sql.NullString
			actorRole         sql.NullString
			actorIP           sql.NullString
			actionDescription string
			oldValues         sql.NullString
			newValues         sql.NullString
			complianceCategory sql.NullString
			regulatoryImpact  sql.NullString
			mfaVerified       sql.NullBool
			signedOffBy       sql.NullString
			signedOffAt       sql.NullTime
			metadata          sql.NullString
		)

		err := rows.Scan(
			&auditRecordID, &auditTimestamp, &auditType, &entityType, &entityID,
			&actorEmail, &actorName, &actorRole, &actorIP, &actionDescription,
			&oldValues, &newValues, &complianceCategory, &regulatoryImpact,
			&mfaVerified, &signedOffBy, &signedOffAt, &metadata,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan row: %w", err)
		}

		record := map[string]interface{}{
			"audit_record_id":     auditRecordID,
			"audit_timestamp":     auditTimestamp.Format(time.RFC3339),
			"audit_type":          auditType,
			"entity_type":         entityType,
			"entity_id":           entityID,
			"actor_email":         actorEmail.String,
			"actor_name":          actorName.String,
			"actor_role":          actorRole.String,
			"actor_ip_address":    actorIP.String,
			"action_description":  actionDescription,
			"compliance_category": complianceCategory.String,
			"regulatory_impact":   regulatoryImpact.String,
			"mfa_verified":        mfaVerified.Bool,
		}

		if req.IncludeMetadata {
			record["old_values"] = oldValues.String
			record["new_values"] = newValues.String
			record["metadata"] = metadata.String
		}

		if signedOffAt.Valid {
			record["signed_off_by"] = signedOffBy.String
			record["signed_off_at"] = signedOffAt.Time.Format(time.RFC3339)
		}

		records = append(records, record)
	}

	// Generate file based on format
	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("audit_trail_%s_%s", req.ComplianceType, timestamp)
	var filePath string

	switch req.Format {
	case FormatCSV:
		filePath = filename + ".csv"
		err = e.writeCSV(filePath, records)
	case FormatExcel:
		filePath = filename + ".xlsx"
		err = e.writeExcel(filePath, records, "Audit Trail")
	case FormatJSON:
		filePath = filename + ".json"
		err = e.writeJSON(filePath, records)
	default:
		return nil, fmt.Errorf("unsupported format: %s", req.Format)
	}

	if err != nil {
		return nil, fmt.Errorf("failed to write file: %w", err)
	}

	return &ExportResponse{
		FilePath:      filePath,
		RecordCount:   len(records),
		GeneratedAt:   time.Now(),
		ReportType:    "audit_trail",
		ComplianceRef: fmt.Sprintf("BIG4-AUDIT-%s", timestamp),
	}, nil
}

// ExportTransactionLedger exports immutable transaction ledger
func (e *AuditExporter) ExportTransactionLedger(ctx context.Context, req ExportRequest) (*ExportResponse, error) {
	query := `
		SELECT
			ledger_entry_id,
			transaction_reference,
			payment_reference,
			debit_account,
			credit_account,
			debit_bank,
			credit_bank,
			transaction_amount,
			transaction_currency,
			fx_rate,
			settlement_amount,
			settlement_currency,
			value_date,
			booking_date,
			settlement_date,
			transaction_hash,
			digital_signature,
			compliance_status,
			risk_score,
			is_reversal,
			reversal_reason,
			posted_by,
			posted_at,
			metadata
		FROM deltran.v_transaction_ledger_export
		WHERE booking_date BETWEEN $1 AND $2
		ORDER BY booking_date DESC
	`

	rows, err := e.db.QueryContext(ctx, query, req.StartDate, req.EndDate)
	if err != nil {
		return nil, fmt.Errorf("failed to query transaction ledger: %w", err)
	}
	defer rows.Close()

	var records []map[string]interface{}
	for rows.Next() {
		var (
			ledgerEntryID      string
			txnRef             string
			paymentRef         sql.NullString
			debitAccount       sql.NullString
			creditAccount      sql.NullString
			debitBank          sql.NullString
			creditBank         sql.NullString
			amount             float64
			currency           string
			fxRate             sql.NullFloat64
			settlementAmount   sql.NullFloat64
			settlementCurrency sql.NullString
			valueDate          time.Time
			bookingDate        time.Time
			settlementDate     sql.NullTime
			txnHash            string
			digitalSignature   sql.NullString
			complianceStatus   string
			riskScore          sql.NullFloat64
			isReversal         bool
			reversalReason     sql.NullString
			postedBy           sql.NullString
			postedAt           sql.NullTime
			metadata           sql.NullString
		)

		err := rows.Scan(
			&ledgerEntryID, &txnRef, &paymentRef, &debitAccount, &creditAccount,
			&debitBank, &creditBank, &amount, &currency, &fxRate,
			&settlementAmount, &settlementCurrency, &valueDate, &bookingDate,
			&settlementDate, &txnHash, &digitalSignature, &complianceStatus,
			&riskScore, &isReversal, &reversalReason, &postedBy, &postedAt, &metadata,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan row: %w", err)
		}

		record := map[string]interface{}{
			"ledger_entry_id":      ledgerEntryID,
			"transaction_reference": txnRef,
			"payment_reference":    paymentRef.String,
			"debit_account":        debitAccount.String,
			"credit_account":       creditAccount.String,
			"debit_bank":           debitBank.String,
			"credit_bank":          creditBank.String,
			"transaction_amount":   amount,
			"transaction_currency": currency,
			"value_date":           valueDate.Format("2006-01-02"),
			"booking_date":         bookingDate.Format(time.RFC3339),
			"transaction_hash":     txnHash,
			"compliance_status":    complianceStatus,
			"is_reversal":          isReversal,
		}

		if fxRate.Valid {
			record["fx_rate"] = fxRate.Float64
			record["settlement_amount"] = settlementAmount.Float64
			record["settlement_currency"] = settlementCurrency.String
		}

		if settlementDate.Valid {
			record["settlement_date"] = settlementDate.Time.Format("2006-01-02")
		}

		if riskScore.Valid {
			record["risk_score"] = riskScore.Float64
		}

		if digitalSignature.Valid {
			record["digital_signature"] = digitalSignature.String
		}

		if postedAt.Valid {
			record["posted_by"] = postedBy.String
			record["posted_at"] = postedAt.Time.Format(time.RFC3339)
		}

		if req.IncludeMetadata && metadata.Valid {
			record["metadata"] = metadata.String
		}

		records = append(records, record)
	}

	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("transaction_ledger_%s", timestamp)
	var filePath string

	switch req.Format {
	case FormatCSV:
		filePath = filename + ".csv"
		err = e.writeCSV(filePath, records)
	case FormatExcel:
		filePath = filename + ".xlsx"
		err = e.writeExcel(filePath, records, "Transaction Ledger")
	case FormatJSON:
		filePath = filename + ".json"
		err = e.writeJSON(filePath, records)
	}

	if err != nil {
		return nil, fmt.Errorf("failed to write file: %w", err)
	}

	return &ExportResponse{
		FilePath:      filePath,
		RecordCount:   len(records),
		GeneratedAt:   time.Now(),
		ReportType:    "transaction_ledger",
		ComplianceRef: fmt.Sprintf("TXN-LEDGER-%s", timestamp),
	}, nil
}

// ExportReconciliation exports reconciliation reports
func (e *AuditExporter) ExportReconciliation(ctx context.Context, req ExportRequest) (*ExportResponse, error) {
	query := `
		SELECT
			reconciliation_reference,
			reconciliation_type,
			reconciliation_date,
			bank_name,
			bic_code,
			currency,
			opening_balance,
			total_debits,
			total_credits,
			closing_balance,
			expected_balance,
			variance,
			status,
			unmatched_items_count,
			reconciled_by,
			reconciled_at,
			approved_by,
			approved_at,
			external_audit_reference
		FROM deltran.v_reconciliation_export
		WHERE reconciliation_date BETWEEN $1 AND $2
		ORDER BY reconciliation_date DESC, bank_name
	`

	rows, err := e.db.QueryContext(ctx, query, req.StartDate, req.EndDate)
	if err != nil {
		return nil, fmt.Errorf("failed to query reconciliation: %w", err)
	}
	defer rows.Close()

	var records []map[string]interface{}
	for rows.Next() {
		var (
			reconRef           string
			reconType          string
			reconDate          time.Time
			bankName           string
			bicCode            string
			currency           string
			openingBalance     float64
			totalDebits        float64
			totalCredits       float64
			closingBalance     float64
			expectedBalance    float64
			variance           float64
			status             string
			unmatchedCount     int
			reconciledBy       sql.NullString
			reconciledAt       sql.NullTime
			approvedBy         sql.NullString
			approvedAt         sql.NullTime
			externalAuditRef   sql.NullString
		)

		err := rows.Scan(
			&reconRef, &reconType, &reconDate, &bankName, &bicCode, &currency,
			&openingBalance, &totalDebits, &totalCredits, &closingBalance,
			&expectedBalance, &variance, &status, &unmatchedCount,
			&reconciledBy, &reconciledAt, &approvedBy, &approvedAt, &externalAuditRef,
		)
		if err != nil {
			return nil, fmt.Errorf("failed to scan row: %w", err)
		}

		record := map[string]interface{}{
			"reconciliation_reference": reconRef,
			"reconciliation_type":      reconType,
			"reconciliation_date":      reconDate.Format("2006-01-02"),
			"bank_name":                bankName,
			"bic_code":                 bicCode,
			"currency":                 currency,
			"opening_balance":          openingBalance,
			"total_debits":             totalDebits,
			"total_credits":            totalCredits,
			"closing_balance":          closingBalance,
			"expected_balance":         expectedBalance,
			"variance":                 variance,
			"status":                   status,
			"unmatched_items_count":    unmatchedCount,
		}

		if reconciledAt.Valid {
			record["reconciled_by"] = reconciledBy.String
			record["reconciled_at"] = reconciledAt.Time.Format(time.RFC3339)
		}

		if approvedAt.Valid {
			record["approved_by"] = approvedBy.String
			record["approved_at"] = approvedAt.Time.Format(time.RFC3339)
		}

		if externalAuditRef.Valid {
			record["external_audit_reference"] = externalAuditRef.String
		}

		records = append(records, record)
	}

	timestamp := time.Now().Format("20060102_150405")
	filename := fmt.Sprintf("reconciliation_report_%s", timestamp)
	var filePath string

	switch req.Format {
	case FormatCSV:
		filePath = filename + ".csv"
		err = e.writeCSV(filePath, records)
	case FormatExcel:
		filePath = filename + ".xlsx"
		err = e.writeExcel(filePath, records, "Reconciliation")
	case FormatJSON:
		filePath = filename + ".json"
		err = e.writeJSON(filePath, records)
	}

	if err != nil {
		return nil, fmt.Errorf("failed to write file: %w", err)
	}

	return &ExportResponse{
		FilePath:      filePath,
		RecordCount:   len(records),
		GeneratedAt:   time.Now(),
		ReportType:    "reconciliation",
		ComplianceRef: fmt.Sprintf("RECON-%s", timestamp),
	}, nil
}

// Helper functions for file writing

func (e *AuditExporter) writeCSV(filePath string, records []map[string]interface{}) error {
	if len(records) == 0 {
		return fmt.Errorf("no records to export")
	}

	file, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer file.Close()

	writer := csv.NewWriter(file)
	defer writer.Flush()

	// Write header
	var headers []string
	for key := range records[0] {
		headers = append(headers, key)
	}
	if err := writer.Write(headers); err != nil {
		return err
	}

	// Write records
	for _, record := range records {
		var row []string
		for _, header := range headers {
			row = append(row, fmt.Sprintf("%v", record[header]))
		}
		if err := writer.Write(row); err != nil {
			return err
		}
	}

	return nil
}

func (e *AuditExporter) writeExcel(filePath string, records []map[string]interface{}, sheetName string) error {
	if len(records) == 0 {
		return fmt.Errorf("no records to export")
	}

	f := excelize.NewFile()
	defer f.Close()

	// Create sheet
	index, err := f.NewSheet(sheetName)
	if err != nil {
		return err
	}

	// Write headers
	var headers []string
	for key := range records[0] {
		headers = append(headers, key)
	}

	for i, header := range headers {
		cell, _ := excelize.CoordinatesToCellName(i+1, 1)
		f.SetCellValue(sheetName, cell, header)
	}

	// Write records
	for rowIdx, record := range records {
		for colIdx, header := range headers {
			cell, _ := excelize.CoordinatesToCellName(colIdx+1, rowIdx+2)
			f.SetCellValue(sheetName, cell, record[header])
		}
	}

	f.SetActiveSheet(index)
	return f.SaveAs(filePath)
}

func (e *AuditExporter) writeJSON(filePath string, records []map[string]interface{}) error {
	file, err := os.Create(filePath)
	if err != nil {
		return err
	}
	defer file.Close()

	encoder := json.NewEncoder(file)
	encoder.SetIndent("", "  ")
	return encoder.Encode(records)
}
