// compliance/screening_service.rs
// Asynchronous sanctions screening service (fail-closed)

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use sqlx::{PgPool, Row};
use std::sync::Arc;
use tokio::sync::RwLock;
use tokio::time::{timeout, Duration};
use tracing::{error, info, warn};
use uuid::Uuid;

const DEFAULT_TIMEOUT_MS: u64 = 500;
const MAX_TIMEOUT_MS: u64 = 5000;
const FUZZY_MATCH_THRESHOLD: f32 = 0.85;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningRequest {
    pub payment_id: Uuid,
    pub amount: Decimal,
    pub currency: String,

    // Debtor
    pub debtor_name: String,
    pub debtor_account: String,
    pub debtor_country: String,
    pub debtor_address: Option<Address>,

    // Creditor
    pub creditor_name: String,
    pub creditor_account: String,
    pub creditor_country: String,
    pub creditor_address: Option<Address>,

    // Purpose
    pub purpose: Option<String>,

    // Options
    pub async_mode: bool,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Address {
    pub address_line_1: Option<String>,
    pub address_line_2: Option<String>,
    pub city: Option<String>,
    pub postal_code: Option<String>,
    pub country_code: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningResponse {
    pub screening_id: Uuid,
    pub payment_id: Uuid,
    pub result: ScreeningResult,
    pub hits: Vec<ScreeningHit>,
    pub screened_at: chrono::NaiveDateTime,
    pub status: ScreeningStatus,
    pub processing_duration_ms: i32,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScreeningResult {
    Clear,           // No hits, proceed
    Hit,             // Match found, investigate
    FalsePositive,   // Hit but cleared
    Blocked,         // Blocked by sanctions
    Pending,         // Still processing
}

impl ToString for ScreeningResult {
    fn to_string(&self) -> String {
        match self {
            ScreeningResult::Clear => "CLEAR".to_string(),
            ScreeningResult::Hit => "HIT".to_string(),
            ScreeningResult::FalsePositive => "FALSE_POSITIVE".to_string(),
            ScreeningResult::Blocked => "BLOCKED".to_string(),
            ScreeningResult::Pending => "PENDING".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum ScreeningStatus {
    Pending,
    Completed,
    Failed,
    Timeout,
}

impl ToString for ScreeningStatus {
    fn to_string(&self) -> String {
        match self {
            ScreeningStatus::Pending => "PENDING".to_string(),
            ScreeningStatus::Completed => "COMPLETED".to_string(),
            ScreeningStatus::Failed => "FAILED".to_string(),
            ScreeningStatus::Timeout => "TIMEOUT".to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScreeningHit {
    pub hit_id: Uuid,
    pub list_name: String,
    pub list_source: String,
    pub entity_name: String,
    pub match_score: f32,
    pub match_reason: String,
    pub hit_type: HitType,
    pub country: Option<String>,
    pub aka_names: Vec<String>,
    pub list_date: Option<String>,
    pub entity_type: String,
    pub program: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HitType {
    Sanctions,
    Pep,
    AdverseMedia,
    HighRisk,
}

impl ToString for HitType {
    fn to_string(&self) -> String {
        match self {
            HitType::Sanctions => "SANCTIONS".to_string(),
            HitType::Pep => "PEP".to_string(),
            HitType::AdverseMedia => "ADVERSE_MEDIA".to_string(),
            HitType::HighRisk => "HIGH_RISK".to_string(),
        }
    }
}

// Screening service with fail-closed semantics
pub struct ScreeningService {
    pool: PgPool,
    // In-memory cache of active screenings for async mode
    active_screenings: Arc<RwLock<std::collections::HashMap<Uuid, ScreeningStatus>>>,
}

impl ScreeningService {
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            active_screenings: Arc::new(RwLock::new(std::collections::HashMap::new())),
        }
    }

    /// Screen payment - main entry point
    pub async fn screen_payment(
        &self,
        request: ScreeningRequest,
    ) -> Result<ScreeningResponse, ScreeningError> {
        let start_time = std::time::Instant::now();
        let screening_id = Uuid::new_v4();

        // Create initial screening record
        self.create_screening_record(screening_id, &request).await?;

        if request.async_mode {
            // Async mode: return immediately with PENDING status
            let response = ScreeningResponse {
                screening_id,
                payment_id: request.payment_id,
                result: ScreeningResult::Pending,
                hits: vec![],
                screened_at: chrono::Utc::now().naive_utc(),
                status: ScreeningStatus::Pending,
                processing_duration_ms: start_time.elapsed().as_millis() as i32,
            };

            // Mark as pending
            self.active_screenings
                .write()
                .await
                .insert(screening_id, ScreeningStatus::Pending);

            // Spawn async screening task
            let service_clone = self.clone();
            let request_clone = request.clone();
            tokio::spawn(async move {
                let _ = service_clone
                    .perform_screening_async(screening_id, request_clone)
                    .await;
            });

            Ok(response)
        } else {
            // Sync mode: wait for result with timeout
            let timeout_ms = request
                .timeout_ms
                .unwrap_or(DEFAULT_TIMEOUT_MS)
                .min(MAX_TIMEOUT_MS);

            match timeout(
                Duration::from_millis(timeout_ms),
                self.perform_screening_sync(screening_id, request),
            )
            .await
            {
                Ok(Ok(response)) => Ok(response),
                Ok(Err(e)) => {
                    // Screening failed
                    self.update_screening_status(screening_id, ScreeningStatus::Failed)
                        .await?;
                    Err(e)
                }
                Err(_) => {
                    // Timeout - FAIL CLOSED
                    warn!(
                        "Screening timeout for payment {} - BLOCKING",
                        request.payment_id
                    );
                    self.update_screening_status(screening_id, ScreeningStatus::Timeout)
                        .await?;

                    // Return BLOCKED result (fail-closed)
                    Ok(ScreeningResponse {
                        screening_id,
                        payment_id: request.payment_id,
                        result: ScreeningResult::Blocked,
                        hits: vec![],
                        screened_at: chrono::Utc::now().naive_utc(),
                        status: ScreeningStatus::Timeout,
                        processing_duration_ms: timeout_ms as i32,
                    })
                }
            }
        }
    }

    /// Get screening result (for async mode)
    pub async fn get_screening_result(
        &self,
        screening_id: Uuid,
    ) -> Result<ScreeningResponse, ScreeningError> {
        let row = sqlx::query(
            r#"
            SELECT
                screening_id,
                payment_id,
                screening_result,
                screening_status,
                screened_at,
                processing_duration_ms
            FROM compliance_screenings
            WHERE screening_id = $1
            "#,
        )
        .bind(screening_id)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        let result_str: String = row.get("screening_result");
        let status_str: String = row.get("screening_status");

        let result = match result_str.as_str() {
            "CLEAR" => ScreeningResult::Clear,
            "HIT" => ScreeningResult::Hit,
            "FALSE_POSITIVE" => ScreeningResult::FalsePositive,
            "BLOCKED" => ScreeningResult::Blocked,
            "PENDING" => ScreeningResult::Pending,
            _ => ScreeningResult::Pending,
        };

        let status = match status_str.as_str() {
            "PENDING" => ScreeningStatus::Pending,
            "COMPLETED" => ScreeningStatus::Completed,
            "FAILED" => ScreeningStatus::Failed,
            "TIMEOUT" => ScreeningStatus::Timeout,
            _ => ScreeningStatus::Pending,
        };

        // Get hits
        let hits = self.get_screening_hits(screening_id).await?;

        Ok(ScreeningResponse {
            screening_id: row.get("screening_id"),
            payment_id: row.get("payment_id"),
            result,
            hits,
            screened_at: row.get("screened_at"),
            status,
            processing_duration_ms: row.get("processing_duration_ms"),
        })
    }

    /// Perform screening synchronously
    async fn perform_screening_sync(
        &self,
        screening_id: Uuid,
        request: ScreeningRequest,
    ) -> Result<ScreeningResponse, ScreeningError> {
        let start_time = std::time::Instant::now();

        // Screen debtor
        let debtor_hits = self
            .screen_entity(&request.debtor_name, &request.debtor_country)
            .await?;

        // Screen creditor
        let creditor_hits = self
            .screen_entity(&request.creditor_name, &request.creditor_country)
            .await?;

        // Check high-risk countries
        let country_risk = self
            .check_country_risk(&request.debtor_country, &request.creditor_country)
            .await?;

        // Combine hits
        let mut all_hits = Vec::new();
        all_hits.extend(debtor_hits);
        all_hits.extend(creditor_hits);

        // Determine result (fail-closed)
        let result = if all_hits.iter().any(|h| h.match_score > 0.95) {
            // High confidence match - BLOCK
            ScreeningResult::Blocked
        } else if !all_hits.is_empty() {
            // Some hits - require review
            ScreeningResult::Hit
        } else if country_risk {
            // High-risk country - flag for review
            ScreeningResult::Hit
        } else {
            // Clear
            ScreeningResult::Clear
        };

        let duration_ms = start_time.elapsed().as_millis() as i32;

        // Save hits to database
        for hit in &all_hits {
            self.save_screening_hit(screening_id, hit).await?;
        }

        // Update screening record
        self.update_screening_result(screening_id, &result, duration_ms)
            .await?;

        info!(
            "Screening {} completed: {:?} ({} hits, {}ms)",
            screening_id,
            result,
            all_hits.len(),
            duration_ms
        );

        Ok(ScreeningResponse {
            screening_id,
            payment_id: request.payment_id,
            result,
            hits: all_hits,
            screened_at: chrono::Utc::now().naive_utc(),
            status: ScreeningStatus::Completed,
            processing_duration_ms: duration_ms,
        })
    }

    /// Perform screening asynchronously (for async mode)
    async fn perform_screening_async(
        &self,
        screening_id: Uuid,
        request: ScreeningRequest,
    ) -> Result<(), ScreeningError> {
        match self.perform_screening_sync(screening_id, request).await {
            Ok(_) => {
                self.active_screenings.write().await.remove(&screening_id);
                Ok(())
            }
            Err(e) => {
                error!("Async screening {} failed: {:?}", screening_id, e);
                self.update_screening_status(screening_id, ScreeningStatus::Failed)
                    .await?;
                self.active_screenings.write().await.remove(&screening_id);
                Err(e)
            }
        }
    }

    /// Screen entity against sanctions lists
    async fn screen_entity(
        &self,
        entity_name: &str,
        country: &str,
    ) -> Result<Vec<ScreeningHit>, ScreeningError> {
        // Use fuzzy matching with trigram similarity
        let rows = sqlx::query(
            r#"
            SELECT
                se.entry_id,
                se.entity_name,
                se.entity_type,
                se.program,
                sl.list_source,
                sl.list_name,
                se.list_date,
                SIMILARITY(se.entity_name, $1) as score
            FROM sanctions_entries se
            JOIN sanctions_lists sl ON se.list_id = sl.list_id
            WHERE
                se.is_active = true
                AND (
                    SIMILARITY(se.entity_name, $1) > $2
                    OR se.entry_id IN (
                        SELECT entry_id FROM sanctions_aka_names
                        WHERE SIMILARITY(aka_name, $1) > $2
                    )
                )
            ORDER BY score DESC
            LIMIT 10
            "#,
        )
        .bind(entity_name)
        .bind(FUZZY_MATCH_THRESHOLD)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        let mut hits = Vec::new();
        for row in rows {
            let entry_id: Uuid = row.get("entry_id");

            // Get AKA names
            let aka_names: Vec<String> = sqlx::query_scalar(
                "SELECT aka_name FROM sanctions_aka_names WHERE entry_id = $1",
            )
            .bind(entry_id)
            .fetch_all(&self.pool)
            .await
            .unwrap_or_default();

            hits.push(ScreeningHit {
                hit_id: Uuid::new_v4(),
                list_name: row.get("list_name"),
                list_source: row.get("list_source"),
                entity_name: row.get("entity_name"),
                match_score: row.get::<f32, _>("score"),
                match_reason: format!("Fuzzy match on name: {}", entity_name),
                hit_type: HitType::Sanctions,
                country: Some(country.to_string()),
                aka_names,
                list_date: row.get("list_date"),
                entity_type: row.get("entity_type"),
                program: row.get("program"),
            });
        }

        Ok(hits)
    }

    /// Check if country is high-risk
    async fn check_country_risk(
        &self,
        debtor_country: &str,
        creditor_country: &str,
    ) -> Result<bool, ScreeningError> {
        let count: i64 = sqlx::query_scalar(
            r#"
            SELECT COUNT(*)
            FROM compliance_high_risk_countries
            WHERE country_code IN ($1, $2)
            AND risk_level IN ('HIGH')
            "#,
        )
        .bind(debtor_country)
        .bind(creditor_country)
        .fetch_one(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        Ok(count > 0)
    }

    /// Create initial screening record
    async fn create_screening_record(
        &self,
        screening_id: Uuid,
        request: &ScreeningRequest,
    ) -> Result<(), ScreeningError> {
        sqlx::query(
            r#"
            INSERT INTO compliance_screenings
            (screening_id, payment_id, screening_result, screening_status, entity_name, entity_country)
            VALUES ($1, $2, $3, $4, $5, $6)
            "#,
        )
        .bind(screening_id)
        .bind(request.payment_id)
        .bind(ScreeningResult::Pending.to_string())
        .bind(ScreeningStatus::Pending.to_string())
        .bind(&request.debtor_name)
        .bind(&request.debtor_country)
        .execute(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update screening result
    async fn update_screening_result(
        &self,
        screening_id: Uuid,
        result: &ScreeningResult,
        duration_ms: i32,
    ) -> Result<(), ScreeningError> {
        sqlx::query(
            r#"
            UPDATE compliance_screenings
            SET screening_result = $1, screening_status = $2, processing_duration_ms = $3, updated_at = NOW()
            WHERE screening_id = $4
            "#,
        )
        .bind(result.to_string())
        .bind(ScreeningStatus::Completed.to_string())
        .bind(duration_ms)
        .bind(screening_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Update screening status
    async fn update_screening_status(
        &self,
        screening_id: Uuid,
        status: ScreeningStatus,
    ) -> Result<(), ScreeningError> {
        sqlx::query(
            r#"
            UPDATE compliance_screenings
            SET screening_status = $1, updated_at = NOW()
            WHERE screening_id = $2
            "#,
        )
        .bind(status.to_string())
        .bind(screening_id)
        .execute(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Save screening hit
    async fn save_screening_hit(
        &self,
        screening_id: Uuid,
        hit: &ScreeningHit,
    ) -> Result<(), ScreeningError> {
        sqlx::query(
            r#"
            INSERT INTO compliance_screening_hits
            (hit_id, screening_id, entry_id, match_score, match_reason, hit_type, list_source, list_name, entity_name, program)
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            "#,
        )
        .bind(hit.hit_id)
        .bind(screening_id)
        .bind(Uuid::new_v4()) // TODO: Use actual entry_id
        .bind(Decimal::from_f32_retain(hit.match_score).unwrap_or_default())
        .bind(&hit.match_reason)
        .bind(hit.hit_type.to_string())
        .bind(&hit.list_source)
        .bind(&hit.list_name)
        .bind(&hit.entity_name)
        .bind(&hit.program)
        .execute(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        Ok(())
    }

    /// Get screening hits
    async fn get_screening_hits(
        &self,
        screening_id: Uuid,
    ) -> Result<Vec<ScreeningHit>, ScreeningError> {
        let rows = sqlx::query(
            r#"
            SELECT
                hit_id, list_name, list_source, entity_name,
                match_score, match_reason, hit_type, program
            FROM compliance_screening_hits
            WHERE screening_id = $1
            "#,
        )
        .bind(screening_id)
        .fetch_all(&self.pool)
        .await
        .map_err(|e| ScreeningError::DatabaseError(e.to_string()))?;

        let hits = rows
            .into_iter()
            .map(|row| {
                let hit_type_str: String = row.get("hit_type");
                let hit_type = match hit_type_str.as_str() {
                    "SANCTIONS" => HitType::Sanctions,
                    "PEP" => HitType::Pep,
                    "ADVERSE_MEDIA" => HitType::AdverseMedia,
                    "HIGH_RISK" => HitType::HighRisk,
                    _ => HitType::Sanctions,
                };

                ScreeningHit {
                    hit_id: row.get("hit_id"),
                    list_name: row.get("list_name"),
                    list_source: row.get("list_source"),
                    entity_name: row.get("entity_name"),
                    match_score: row.get::<Decimal, _>("match_score").to_string().parse().unwrap_or(0.0),
                    match_reason: row.get("match_reason"),
                    hit_type,
                    country: None,
                    aka_names: vec![],
                    list_date: None,
                    entity_type: "unknown".to_string(),
                    program: row.get("program"),
                }
            })
            .collect();

        Ok(hits)
    }
}

impl Clone for ScreeningService {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
            active_screenings: self.active_screenings.clone(),
        }
    }
}

#[derive(Debug)]
pub enum ScreeningError {
    DatabaseError(String),
    ValidationError(String),
    TimeoutError,
    InternalError(String),
}

impl std::fmt::Display for ScreeningError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ScreeningError::DatabaseError(e) => write!(f, "Database error: {}", e),
            ScreeningError::ValidationError(e) => write!(f, "Validation error: {}", e),
            ScreeningError::TimeoutError => write!(f, "Screening timeout"),
            ScreeningError::InternalError(e) => write!(f, "Internal error: {}", e),
        }
    }
}

impl std::error::Error for ScreeningError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_screening_clear() {
        // Test clear screening
    }

    #[tokio::test]
    async fn test_screening_hit() {
        // Test screening with hit
    }

    #[tokio::test]
    async fn test_screening_timeout_fail_closed() {
        // Test timeout results in BLOCKED (fail-closed)
    }

    #[tokio::test]
    async fn test_async_screening() {
        // Test async mode
    }
}
