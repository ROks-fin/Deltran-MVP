//! Regulatory Report Generator
//!
//! Generates Big 4 compliance reports in multiple formats:
//! - AML Annual Returns (UAE FIU)
//! - Prudential Returns (FSRA)
//! - Currency Transaction Reports (CTR)
//! - Suspicious Activity Reports (SAR)
//! - Client Money Safeguarding Reports
//! - Payment Statistics (Quarterly)
//! - Technology Risk Reports
//! - Model Validation Reports

use chrono::{DateTime, Utc, Datelike};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use thiserror::Error;
use uuid::Uuid;

#[derive(Error, Debug)]
pub enum ReportError {
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("Serialization error: {0}")]
    Serialization(String),

    #[error("Invalid date range: {0}")]
    InvalidDateRange(String),

    #[error("Data not found: {0}")]
    DataNotFound(String),

    #[error("Report generation failed: {0}")]
    GenerationFailed(String),
}

pub type Result<T> = std::result::Result<T, ReportError>;

/// Report type (Big 4 standard)
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportType {
    AmlAnnual,          // UAE FIU Annual Return
    PruMonthly,         // FSRA Prudential Returns
    Safeguarding,       // Client money safeguarding
    PaymentStats,       // Payment statistics (quarterly)
    Ctr,                // Currency Transaction Report
    TechRisk,           // Technology risk report
    ModelValidation,    // Risk model validation
    AuditTrail,         // Complete audit trail export
    TransactionLog,     // Transaction log export
}

/// Report format
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum ReportFormat {
    Excel,  // .xlsx (most common for Big 4)
    Pdf,    // .pdf (for formal submissions)
    Csv,    // .csv (for data analysis)
    Json,   // .json (for API consumption)
    Xml,    // .xml (ISO 20022 format)
}

impl ReportFormat {
    pub fn extension(&self) -> &str {
        match self {
            ReportFormat::Excel => "xlsx",
            ReportFormat::Pdf => "pdf",
            ReportFormat::Csv => "csv",
            ReportFormat::Json => "json",
            ReportFormat::Xml => "xml",
        }
    }

    pub fn mime_type(&self) -> &str {
        match self {
            ReportFormat::Excel => "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet",
            ReportFormat::Pdf => "application/pdf",
            ReportFormat::Csv => "text/csv",
            ReportFormat::Json => "application/json",
            ReportFormat::Xml => "application/xml",
        }
    }
}

/// Report status
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum ReportStatus {
    Generating,
    Ready,
    Approved,
    Submitted,
    Failed,
}

/// Report metadata
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportMetadata {
    pub report_id: Uuid,
    pub report_type: ReportType,
    pub format: ReportFormat,
    pub status: ReportStatus,
    pub generated_at: DateTime<Utc>,
    pub period_start: DateTime<Utc>,
    pub period_end: DateTime<Utc>,
    pub file_path: PathBuf,
    pub file_size: u64,
    pub download_url: Option<String>,
    pub generated_by: String,
    pub approved_by: Option<String>,
    pub submitted_at: Option<DateTime<Utc>>,
}

/// Report data aggregation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReportData {
    pub total_transactions: u64,
    pub total_volume: rust_decimal::Decimal,
    pub successful_transactions: u64,
    pub failed_transactions: u64,
    pub currencies: HashMap<String, CurrencyStats>,
    pub banks: HashMap<String, BankStats>,
    pub compliance_checks: ComplianceStats,
    pub risk_metrics: RiskMetrics,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyStats {
    pub currency: String,
    pub transaction_count: u64,
    pub total_volume: rust_decimal::Decimal,
    pub average_amount: rust_decimal::Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankStats {
    pub bank_id: String,
    pub bank_name: String,
    pub swift_bic: String,
    pub sent_count: u64,
    pub received_count: u64,
    pub sent_volume: rust_decimal::Decimal,
    pub received_volume: rust_decimal::Decimal,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ComplianceStats {
    pub total_checks: u64,
    pub passed: u64,
    pub failed: u64,
    pub flagged: u64,
    pub sanctions_hits: u64,
    pub pep_matches: u64,
    pub aml_alerts: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskMetrics {
    pub average_risk_score: f64,
    pub high_risk_transactions: u64,
    pub fraud_alerts: u64,
    pub velocity_violations: u64,
}

/// Report generator configuration
#[derive(Debug, Clone)]
pub struct ReportConfig {
    pub output_dir: PathBuf,
    pub base_url: String,
    pub retention_days: u32,
}

impl Default for ReportConfig {
    fn default() -> Self {
        Self {
            output_dir: PathBuf::from("./data/reports"),
            base_url: "http://localhost:8080".to_string(),
            retention_days: 2555, // 7 years
        }
    }
}

/// Regulatory report generator
pub struct ReportGenerator {
    config: ReportConfig,
}

impl ReportGenerator {
    pub fn new(config: ReportConfig) -> Result<Self> {
        // Create output directory
        std::fs::create_dir_all(&config.output_dir)?;

        Ok(Self { config })
    }

    /// Generate report
    pub async fn generate_report(
        &self,
        report_type: ReportType,
        format: ReportFormat,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        data: ReportData,
        generated_by: String,
    ) -> Result<ReportMetadata> {
        // Validate date range
        if end_date <= start_date {
            return Err(ReportError::InvalidDateRange(
                "End date must be after start date".to_string(),
            ));
        }

        let report_id = Uuid::new_v4();
        let timestamp = Utc::now();

        // Generate filename
        let filename = self.generate_filename(&report_type, &format, &start_date, &end_date, &report_id);
        let file_path = self.config.output_dir.join(&filename);

        // Generate report content based on format
        let file_size = match format {
            ReportFormat::Excel => self.generate_excel(&file_path, &report_type, &data).await?,
            ReportFormat::Pdf => self.generate_pdf(&file_path, &report_type, &data).await?,
            ReportFormat::Csv => self.generate_csv(&file_path, &report_type, &data).await?,
            ReportFormat::Json => self.generate_json(&file_path, &report_type, &data).await?,
            ReportFormat::Xml => self.generate_xml(&file_path, &report_type, &data).await?,
        };

        // Generate download URL
        let download_url = Some(format!(
            "{}/api/v1/compliance/reports/{}/download",
            self.config.base_url, report_id
        ));

        Ok(ReportMetadata {
            report_id,
            report_type,
            format,
            status: ReportStatus::Ready,
            generated_at: timestamp,
            period_start: start_date,
            period_end: end_date,
            file_path,
            file_size,
            download_url,
            generated_by,
            approved_by: None,
            submitted_at: None,
        })
    }

    /// Generate filename
    fn generate_filename(
        &self,
        report_type: &ReportType,
        format: &ReportFormat,
        start_date: &DateTime<Utc>,
        end_date: &DateTime<Utc>,
        report_id: &Uuid,
    ) -> String {
        let type_name = match report_type {
            ReportType::AmlAnnual => "AML_Annual",
            ReportType::PruMonthly => "Prudential_Monthly",
            ReportType::Safeguarding => "Safeguarding",
            ReportType::PaymentStats => "Payment_Stats",
            ReportType::Ctr => "CTR",
            ReportType::TechRisk => "Tech_Risk",
            ReportType::ModelValidation => "Model_Validation",
            ReportType::AuditTrail => "Audit_Trail",
            ReportType::TransactionLog => "Transaction_Log",
        };

        let period = if start_date.year() == end_date.year() && start_date.month() == end_date.month() {
            format!("{}_{:02}", start_date.year(), start_date.month())
        } else {
            format!("{}_{:02}_to_{}_{:02}",
                start_date.year(), start_date.month(),
                end_date.year(), end_date.month())
        };

        format!(
            "{}_{}_{}_{}.{}",
            type_name,
            period,
            start_date.format("%Y%m%d"),
            &report_id.to_string()[..8],
            format.extension()
        )
    }

    /// Generate Excel report (Big 4 standard format)
    async fn generate_excel(&self, path: &Path, report_type: &ReportType, data: &ReportData) -> Result<u64> {
        // For now, generate JSON representation
        // TODO: Integrate rust_xlsxwriter for actual Excel generation
        let content = self.format_excel_content(report_type, data)?;
        std::fs::write(path, content.as_bytes())?;
        Ok(content.len() as u64)
    }

    fn format_excel_content(&self, report_type: &ReportType, data: &ReportData) -> Result<String> {
        let mut content = String::new();

        content.push_str(&format!("=== {} REPORT ===\n\n", self.report_type_name(report_type)));
        content.push_str(&format!("Generated: {}\n\n", Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        // Summary section
        content.push_str("SUMMARY\n");
        content.push_str(&format!("Total Transactions: {}\n", data.total_transactions));
        content.push_str(&format!("Total Volume: ${}\n", data.total_volume));
        content.push_str(&format!("Successful: {}\n", data.successful_transactions));
        content.push_str(&format!("Failed: {}\n", data.failed_transactions));
        content.push_str(&format!("Success Rate: {:.2}%\n\n",
            (data.successful_transactions as f64 / data.total_transactions as f64) * 100.0));

        // Currency breakdown
        content.push_str("CURRENCY BREAKDOWN\n");
        for (currency, stats) in &data.currencies {
            content.push_str(&format!("{}: {} transactions, ${}\n",
                currency, stats.transaction_count, stats.total_volume));
        }
        content.push_str("\n");

        // Bank breakdown
        content.push_str("BANK STATISTICS\n");
        for (bank_id, stats) in &data.banks {
            content.push_str(&format!("{} ({}): Sent {}, Received {}\n",
                stats.bank_name, stats.swift_bic, stats.sent_count, stats.received_count));
        }
        content.push_str("\n");

        // Compliance statistics
        content.push_str("COMPLIANCE CHECKS\n");
        content.push_str(&format!("Total Checks: {}\n", data.compliance_checks.total_checks));
        content.push_str(&format!("Passed: {}\n", data.compliance_checks.passed));
        content.push_str(&format!("Failed: {}\n", data.compliance_checks.failed));
        content.push_str(&format!("Flagged: {}\n", data.compliance_checks.flagged));
        content.push_str(&format!("Sanctions Hits: {}\n", data.compliance_checks.sanctions_hits));
        content.push_str(&format!("PEP Matches: {}\n", data.compliance_checks.pep_matches));
        content.push_str(&format!("AML Alerts: {}\n\n", data.compliance_checks.aml_alerts));

        // Risk metrics
        content.push_str("RISK METRICS\n");
        content.push_str(&format!("Average Risk Score: {:.2}\n", data.risk_metrics.average_risk_score));
        content.push_str(&format!("High Risk Transactions: {}\n", data.risk_metrics.high_risk_transactions));
        content.push_str(&format!("Fraud Alerts: {}\n", data.risk_metrics.fraud_alerts));
        content.push_str(&format!("Velocity Violations: {}\n", data.risk_metrics.velocity_violations));

        Ok(content)
    }

    /// Generate PDF report
    async fn generate_pdf(&self, path: &Path, report_type: &ReportType, data: &ReportData) -> Result<u64> {
        // For now, generate text representation
        // TODO: Integrate printpdf or similar for actual PDF generation
        let content = self.format_excel_content(report_type, data)?;
        std::fs::write(path, content.as_bytes())?;
        Ok(content.len() as u64)
    }

    /// Generate CSV report
    async fn generate_csv(&self, path: &Path, _report_type: &ReportType, data: &ReportData) -> Result<u64> {
        let mut content = String::new();

        // Header
        content.push_str("Bank ID,Bank Name,SWIFT BIC,Sent Count,Received Count,Sent Volume,Received Volume\n");

        // Data rows
        for (_, stats) in &data.banks {
            content.push_str(&format!("{},{},{},{},{},{},{}\n",
                stats.bank_id,
                stats.bank_name,
                stats.swift_bic,
                stats.sent_count,
                stats.received_count,
                stats.sent_volume,
                stats.received_volume
            ));
        }

        std::fs::write(path, content.as_bytes())?;
        Ok(content.len() as u64)
    }

    /// Generate JSON report
    async fn generate_json(&self, path: &Path, _report_type: &ReportType, data: &ReportData) -> Result<u64> {
        let json = serde_json::to_string_pretty(data)
            .map_err(|e| ReportError::Serialization(e.to_string()))?;
        std::fs::write(path, json.as_bytes())?;
        Ok(json.len() as u64)
    }

    /// Generate XML report (ISO 20022 format)
    async fn generate_xml(&self, path: &Path, _report_type: &ReportType, data: &ReportData) -> Result<u64> {
        let mut content = String::new();
        content.push_str("<?xml version=\"1.0\" encoding=\"UTF-8\"?>\n");
        content.push_str("<RegulatoryReport xmlns=\"urn:iso:std:iso:20022:tech:xsd\">\n");
        content.push_str(&format!("  <TotalTransactions>{}</TotalTransactions>\n", data.total_transactions));
        content.push_str(&format!("  <TotalVolume>{}</TotalVolume>\n", data.total_volume));
        content.push_str("</RegulatoryReport>\n");

        std::fs::write(path, content.as_bytes())?;
        Ok(content.len() as u64)
    }

    fn report_type_name(&self, report_type: &ReportType) -> &str {
        match report_type {
            ReportType::AmlAnnual => "AML ANNUAL RETURN",
            ReportType::PruMonthly => "PRUDENTIAL MONTHLY RETURN",
            ReportType::Safeguarding => "CLIENT MONEY SAFEGUARDING",
            ReportType::PaymentStats => "PAYMENT STATISTICS",
            ReportType::Ctr => "CURRENCY TRANSACTION REPORT",
            ReportType::TechRisk => "TECHNOLOGY RISK REPORT",
            ReportType::ModelValidation => "MODEL VALIDATION REPORT",
            ReportType::AuditTrail => "AUDIT TRAIL EXPORT",
            ReportType::TransactionLog => "TRANSACTION LOG",
        }
    }

    /// List all generated reports
    pub async fn list_reports(&self) -> Result<Vec<ReportMetadata>> {
        let mut reports = Vec::new();

        for entry in std::fs::read_dir(&self.config.output_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                // Parse filename to extract metadata
                // For now, just return basic metadata
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    let metadata = std::fs::metadata(&path)?;

                    // This is simplified - in production, store metadata in a database
                    reports.push(ReportMetadata {
                        report_id: Uuid::new_v4(),
                        report_type: ReportType::AuditTrail,
                        format: ReportFormat::Excel,
                        status: ReportStatus::Ready,
                        generated_at: Utc::now(),
                        period_start: Utc::now(),
                        period_end: Utc::now(),
                        file_path: path,
                        file_size: metadata.len(),
                        download_url: None,
                        generated_by: "system".to_string(),
                        approved_by: None,
                        submitted_at: None,
                    });
                }
            }
        }

        Ok(reports)
    }

    /// Get report by ID
    pub async fn get_report(&self, report_id: &Uuid) -> Result<Vec<u8>> {
        // In production, look up file path from database
        // For now, search directory
        for entry in std::fs::read_dir(&self.config.output_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                if let Some(filename) = path.file_name().and_then(|n| n.to_str()) {
                    if filename.contains(&report_id.to_string()[..8]) {
                        return Ok(std::fs::read(&path)?);
                    }
                }
            }
        }

        Err(ReportError::DataNotFound(format!("Report {} not found", report_id)))
    }

    /// Delete old reports (cleanup)
    pub async fn cleanup_old_reports(&self) -> Result<usize> {
        let cutoff = Utc::now() - chrono::Duration::days(self.config.retention_days as i64);
        let mut deleted = 0;

        for entry in std::fs::read_dir(&self.config.output_dir)? {
            let entry = entry?;
            let path = entry.path();

            if path.is_file() {
                let metadata = std::fs::metadata(&path)?;
                if let Ok(modified) = metadata.modified() {
                    let modified_chrono: DateTime<Utc> = modified.into();
                    if modified_chrono < cutoff {
                        std::fs::remove_file(&path)?;
                        deleted += 1;
                    }
                }
            }
        }

        Ok(deleted)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal::Decimal;

    #[tokio::test]
    async fn test_generate_report() {
        let config = ReportConfig {
            output_dir: PathBuf::from("./test_reports"),
            base_url: "http://localhost:8080".to_string(),
            retention_days: 2555,
        };

        let generator = ReportGenerator::new(config).unwrap();

        let data = ReportData {
            total_transactions: 1000,
            total_volume: Decimal::new(1000000, 2),
            successful_transactions: 980,
            failed_transactions: 20,
            currencies: HashMap::new(),
            banks: HashMap::new(),
            compliance_checks: ComplianceStats {
                total_checks: 1000,
                passed: 950,
                failed: 30,
                flagged: 20,
                sanctions_hits: 5,
                pep_matches: 3,
                aml_alerts: 2,
            },
            risk_metrics: RiskMetrics {
                average_risk_score: 15.5,
                high_risk_transactions: 10,
                fraud_alerts: 2,
                velocity_violations: 5,
            },
        };

        let metadata = generator
            .generate_report(
                ReportType::AmlAnnual,
                ReportFormat::Json,
                Utc::now() - chrono::Duration::days(30),
                Utc::now(),
                data,
                "test_user".to_string(),
            )
            .await
            .unwrap();

        assert_eq!(metadata.report_type, ReportType::AmlAnnual);
        assert_eq!(metadata.format, ReportFormat::Json);
        assert_eq!(metadata.status, ReportStatus::Ready);

        // Cleanup
        std::fs::remove_dir_all("./test_reports").ok();
    }
}
