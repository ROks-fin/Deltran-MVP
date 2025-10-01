//! Report Generation Service
//!
//! Generates regulatory reports in various formats (XLSX, CSV, PDF, JSON)

use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::PgPool;
use std::io::Write;
use thiserror::Error;
use tracing::{error, info};
use uuid::Uuid;
use xlsxwriter::{Format, FormatAlignment, FormatBorder, Workbook, Worksheet};

#[derive(Debug, Error)]
pub enum ReportGeneratorError {
    #[error("Database error: {0}")]
    DatabaseError(#[from] sqlx::Error),

    #[error("Report generation error: {0}")]
    GenerationError(String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("XLSX error: {0}")]
    XlsxError(String),
}

/// Report format
#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Xlsx,
    Csv,
    Pdf,
    Json,
    Xml,
}

/// Report type
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportType {
    PruMonthly,
    PruQuarterly,
    IfrsAnnual,
    SafeguardingMonthly,
    PaymentStatsQuarterly,
    AmlAnnual,
    StrSubmission,
    TechRiskQuarterly,
    ModelValidationAnnual,
}

/// Generated report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GeneratedReport {
    pub report_id: Uuid,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub file_path: String,
    pub file_size_bytes: i64,
    pub file_hash: String,
    pub generated_at: DateTime<Utc>,
    pub generated_by: String,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
}

/// Report data row (generic)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportDataRow {
    pub fields: Vec<String>,
}

/// Report Generator Service
pub struct ReportGenerator {
    pool: PgPool,
    output_dir: String,
}

impl ReportGenerator {
    /// Create new report generator
    pub fn new(pool: PgPool, output_dir: String) -> Self {
        Self { pool, output_dir }
    }

    /// Generate PRU monthly report
    pub async fn generate_pru_monthly(
        &self,
        year: i32,
        month: u32,
        format: ReportFormat,
        generated_by: &str,
    ) -> Result<GeneratedReport, ReportGeneratorError> {
        info!(year = year, month = month, "Generating PRU monthly report");

        let period_start = chrono::NaiveDate::from_ymd_opt(year, month, 1)
            .unwrap()
            .and_hms_opt(0, 0, 0)
            .unwrap()
            .and_utc();

        let period_end = if month == 12 {
            chrono::NaiveDate::from_ymd_opt(year + 1, 1, 1)
        } else {
            chrono::NaiveDate::from_ymd_opt(year, month + 1, 1)
        }
        .unwrap()
        .and_hms_opt(0, 0, 0)
        .unwrap()
        .and_utc();

        // Fetch data from gold.v_pru_monthly
        let rows = sqlx::query(
            r#"
            SELECT
                reporting_month,
                currency,
                high_risk_exposure,
                total_exposure,
                cross_border_exposure,
                transaction_count,
                avg_transaction_value,
                value_volatility,
                avg_risk_score,
                high_risk_count,
                avg_settlement_hours,
                compliance_issues
            FROM gold.v_pru_monthly
            WHERE reporting_month >= $1
              AND reporting_month < $2
            ORDER BY currency, reporting_month
            "#,
        )
        .bind(period_start)
        .bind(period_end)
        .fetch_all(&self.pool)
        .await?;

        // Generate report file
        let report_id = Uuid::new_v4();
        let filename = format!("PRU_Monthly_{}_{:02}.xlsx", year, month);
        let file_path = format!("{}/{}", self.output_dir, filename);

        match format {
            ReportFormat::Xlsx => {
                self.generate_pru_xlsx(&file_path, &rows)?;
            }
            ReportFormat::Csv => {
                self.generate_pru_csv(&file_path, &rows)?;
            }
            _ => {
                return Err(ReportGeneratorError::GenerationError(format!(
                    "Unsupported format: {:?}",
                    format
                )));
            }
        }

        // Calculate file size and hash
        let file_size_bytes = std::fs::metadata(&file_path)?.len() as i64;
        let file_hash = self.calculate_file_hash(&file_path)?;

        let report = GeneratedReport {
            report_id,
            report_type: ReportType::PruMonthly,
            format,
            file_path: file_path.clone(),
            file_size_bytes,
            file_hash,
            generated_at: Utc::now(),
            generated_by: generated_by.to_string(),
            period_start,
            period_end,
        };

        // Store in database
        self.store_report(&report).await?;

        info!(
            report_id = %report_id,
            file_path = file_path,
            "PRU monthly report generated successfully"
        );

        Ok(report)
    }

    /// Generate PRU report as XLSX
    fn generate_pru_xlsx(
        &self,
        file_path: &str,
        rows: &[sqlx::postgres::PgRow],
    ) -> Result<(), ReportGeneratorError> {
        let workbook = Workbook::new(file_path)
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;

        let mut sheet = workbook
            .add_worksheet(Some("PRU Monthly"))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;

        // Header format
        let header_format = workbook
            .add_format()
            .set_bold()
            .set_align(FormatAlignment::Center)
            .set_border(FormatBorder::Thin);

        // Write headers
        sheet
            .write_string(0, 0, "Reporting Month", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 1, "Currency", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 2, "High Risk Exposure", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 3, "Total Exposure", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 4, "Cross-Border Exposure", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 5, "Transaction Count", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 6, "Avg Transaction Value", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 7, "Avg Risk Score", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 8, "High Risk Count", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 9, "Avg Settlement Hours", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
        sheet
            .write_string(0, 10, "Compliance Issues", Some(&header_format))
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;

        // Write data
        for (idx, row) in rows.iter().enumerate() {
            let row_idx = (idx + 1) as u32;

            let reporting_month: DateTime<Utc> = row.get("reporting_month");
            sheet
                .write_string(row_idx, 0, &reporting_month.format("%Y-%m").to_string(), None)
                .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;

            let currency: String = row.get("currency");
            sheet
                .write_string(row_idx, 1, &currency, None)
                .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;

            let high_risk_exposure: Option<Decimal> = row.get("high_risk_exposure");
            if let Some(val) = high_risk_exposure {
                sheet
                    .write_number(row_idx, 2, val.to_string().parse().unwrap_or(0.0), None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let total_exposure: Option<Decimal> = row.get("total_exposure");
            if let Some(val) = total_exposure {
                sheet
                    .write_number(row_idx, 3, val.to_string().parse().unwrap_or(0.0), None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let cross_border_exposure: Option<Decimal> = row.get("cross_border_exposure");
            if let Some(val) = cross_border_exposure {
                sheet
                    .write_number(row_idx, 4, val.to_string().parse().unwrap_or(0.0), None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let transaction_count: Option<i64> = row.get("transaction_count");
            if let Some(val) = transaction_count {
                sheet
                    .write_number(row_idx, 5, val as f64, None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let avg_transaction_value: Option<Decimal> = row.get("avg_transaction_value");
            if let Some(val) = avg_transaction_value {
                sheet
                    .write_number(row_idx, 6, val.to_string().parse().unwrap_or(0.0), None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let avg_risk_score: Option<Decimal> = row.get("avg_risk_score");
            if let Some(val) = avg_risk_score {
                sheet
                    .write_number(row_idx, 7, val.to_string().parse().unwrap_or(0.0), None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let high_risk_count: Option<i64> = row.get("high_risk_count");
            if let Some(val) = high_risk_count {
                sheet
                    .write_number(row_idx, 8, val as f64, None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let avg_settlement_hours: Option<Decimal> = row.get("avg_settlement_hours");
            if let Some(val) = avg_settlement_hours {
                sheet
                    .write_number(row_idx, 9, val.to_string().parse().unwrap_or(0.0), None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }

            let compliance_issues: Option<i64> = row.get("compliance_issues");
            if let Some(val) = compliance_issues {
                sheet
                    .write_number(row_idx, 10, val as f64, None)
                    .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;
            }
        }

        workbook
            .close()
            .map_err(|e| ReportGeneratorError::XlsxError(e.to_string()))?;

        Ok(())
    }

    /// Generate PRU report as CSV
    fn generate_pru_csv(
        &self,
        file_path: &str,
        rows: &[sqlx::postgres::PgRow],
    ) -> Result<(), ReportGeneratorError> {
        let mut file = std::fs::File::create(file_path)?;

        // Write header
        writeln!(
            file,
            "Reporting Month,Currency,High Risk Exposure,Total Exposure,Cross-Border Exposure,Transaction Count,Avg Transaction Value,Avg Risk Score,High Risk Count,Avg Settlement Hours,Compliance Issues"
        )?;

        // Write data
        for row in rows {
            let reporting_month: DateTime<Utc> = row.get("reporting_month");
            let currency: String = row.get("currency");
            let high_risk_exposure: Option<Decimal> = row.get("high_risk_exposure");
            let total_exposure: Option<Decimal> = row.get("total_exposure");
            let cross_border_exposure: Option<Decimal> = row.get("cross_border_exposure");
            let transaction_count: Option<i64> = row.get("transaction_count");
            let avg_transaction_value: Option<Decimal> = row.get("avg_transaction_value");
            let avg_risk_score: Option<Decimal> = row.get("avg_risk_score");
            let high_risk_count: Option<i64> = row.get("high_risk_count");
            let avg_settlement_hours: Option<Decimal> = row.get("avg_settlement_hours");
            let compliance_issues: Option<i64> = row.get("compliance_issues");

            writeln!(
                file,
                "{},{},{},{},{},{},{},{},{},{},{}",
                reporting_month.format("%Y-%m"),
                currency,
                high_risk_exposure.map(|d| d.to_string()).unwrap_or_default(),
                total_exposure.map(|d| d.to_string()).unwrap_or_default(),
                cross_border_exposure.map(|d| d.to_string()).unwrap_or_default(),
                transaction_count.unwrap_or(0),
                avg_transaction_value.map(|d| d.to_string()).unwrap_or_default(),
                avg_risk_score.map(|d| d.to_string()).unwrap_or_default(),
                high_risk_count.unwrap_or(0),
                avg_settlement_hours.map(|d| d.to_string()).unwrap_or_default(),
                compliance_issues.unwrap_or(0),
            )?;
        }

        Ok(())
    }

    /// Calculate file hash (SHA-256)
    fn calculate_file_hash(&self, file_path: &str) -> Result<String, ReportGeneratorError> {
        use sha2::{Digest, Sha256};

        let contents = std::fs::read(file_path)?;
        let hash = Sha256::digest(&contents);
        Ok(hex::encode(hash))
    }

    /// Store report metadata in database
    async fn store_report(&self, report: &GeneratedReport) -> Result<(), ReportGeneratorError> {
        sqlx::query(
            r#"
            INSERT INTO compliance.regulatory_reports
            (report_id, report_type, report_format, generation_timestamp,
             status, data_version_hash, rules_version, file_path,
             file_size_bytes, file_hash, generated_by)
            VALUES ($1, $2, $3, $4, 'ready', $5, '1.0', $6, $7, $8, $9)
            "#,
        )
        .bind(report.report_id)
        .bind(serde_json::to_string(&report.report_type).unwrap())
        .bind(match report.format {
            ReportFormat::Xlsx => "xlsx",
            ReportFormat::Csv => "csv",
            ReportFormat::Pdf => "pdf",
            ReportFormat::Json => "json",
            ReportFormat::Xml => "xml",
        })
        .bind(report.generated_at)
        .bind(&report.file_hash)
        .bind(&report.file_path)
        .bind(report.file_size_bytes)
        .bind(&report.file_hash)
        .bind(&report.generated_by)
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}
