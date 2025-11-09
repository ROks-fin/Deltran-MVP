use crate::aml::AmlScorer;
use crate::errors::ComplianceError;
use crate::models::*;
use crate::pep::PepChecker;
use crate::sanctions::SanctionsMatcher;
use actix_web::{web, HttpResponse};
use chrono::Utc;
use sqlx::PgPool;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

// ===== Health Check =====
pub async fn health_check(pool: web::Data<PgPool>) -> HttpResponse {
    let _db_status = match sqlx::query("SELECT 1").execute(pool.get_ref()).await {
        Ok(_) => "connected",
        Err(_) => "disconnected",
    };

    let uptime = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs();

    let health = HealthResponse {
        status: "healthy".to_string(),
        version: env!("CARGO_PKG_VERSION").to_string(),
        uptime_seconds: uptime,
    };

    HttpResponse::Ok().json(health)
}

// ===== Compliance Check =====
pub async fn check_compliance(
    req: web::Json<ComplianceCheckRequest>,
    sanctions_matcher: web::Data<Arc<SanctionsMatcher>>,
    aml_scorer: web::Data<Arc<AmlScorer>>,
    pep_checker: web::Data<Arc<PepChecker>>,
    pool: web::Data<PgPool>,
) -> Result<HttpResponse, ComplianceError> {
    // 1. Sanctions Check
    let sender_sanctions = sanctions_matcher
        .check_sanctions(&req.sender_name, &req.sender_country)?;

    let receiver_sanctions = sanctions_matcher
        .check_sanctions(&req.receiver_name, &req.receiver_country)?;

    let is_sanctioned = sender_sanctions.is_sanctioned || receiver_sanctions.is_sanctioned;

    let mut all_matches = sender_sanctions.match_details.clone();
    all_matches.extend(receiver_sanctions.match_details);

    let mut all_lists = sender_sanctions.lists_matched.clone();
    all_lists.extend(receiver_sanctions.lists_matched);
    all_lists.dedup();

    let sanctions_check = SanctionsResult {
        is_sanctioned,
        lists_matched: all_lists,
        match_details: all_matches,
        confidence: sender_sanctions.confidence.min(receiver_sanctions.confidence),
    };

    // 2. AML Check
    let aml_check = aml_scorer.calculate_aml_risk(&req, &pool).await?;

    // 3. PEP Check
    let sender_pep = pep_checker.check_pep(&req.sender_name, &req.sender_country)?;
    let receiver_pep = pep_checker.check_pep(&req.receiver_name, &req.receiver_country)?;

    let is_pep = sender_pep.is_pep || receiver_pep.is_pep;

    let pep_check = PepResult {
        is_pep,
        pep_type: sender_pep.pep_type.or(receiver_pep.pep_type),
        position: sender_pep.position.or(receiver_pep.position),
        country: sender_pep.country.or(receiver_pep.country),
        risk_level: sender_pep.risk_level.or(receiver_pep.risk_level),
    };

    // 4. Pattern Analysis (simplified for MVP)
    let pattern_analysis = PatternResult {
        normal_behavior: aml_check.suspicious_patterns.is_empty(),
        anomaly_score: aml_check.risk_score,
        detected_patterns: aml_check.suspicious_patterns.iter()
            .map(|p| p.pattern_name.clone())
            .collect(),
        ml_confidence: 0.0, // No ML in MVP
    };

    // Determine overall status and risk rating
    let (overall_status, risk_rating) = determine_compliance_status(
        &sanctions_check,
        &aml_check,
        &pep_check,
    );

    // Required actions
    let required_actions = determine_required_actions(
        &sanctions_check,
        &aml_check,
        &pep_check,
        &overall_status,
    );

    let result = ComplianceCheckResult {
        transaction_id: req.transaction_id,
        overall_status,
        sanctions_check,
        aml_check,
        pep_check,
        pattern_analysis,
        risk_rating,
        required_actions,
        checked_at: Utc::now(),
    };

    // Save to database (simplified for MVP)
    save_compliance_check(&result, &pool).await?;

    let response = ComplianceResponse::from(result);
    Ok(HttpResponse::Ok().json(response))
}

fn determine_compliance_status(
    sanctions: &SanctionsResult,
    aml: &AmlResult,
    pep: &PepResult,
) -> (ComplianceStatus, RiskRating) {
    // If sanctioned, reject immediately
    if sanctions.is_sanctioned {
        return (ComplianceStatus::Rejected, RiskRating::Prohibited);
    }

    // Determine risk rating based on AML score
    let risk_rating = match aml.risk_score {
        s if s < 30.0 => RiskRating::Low,
        s if s < 60.0 => RiskRating::Medium,
        s if s < 80.0 => RiskRating::High,
        _ => RiskRating::VeryHigh,
    };

    // Determine status
    let status = if aml.risk_score > 75.0 || (pep.is_pep && aml.risk_score > 50.0) {
        ComplianceStatus::ReviewRequired
    } else if aml.risk_score > 85.0 || aml.requires_sar {
        ComplianceStatus::Hold
    } else {
        ComplianceStatus::Approved
    };

    (status, risk_rating)
}

fn determine_required_actions(
    sanctions: &SanctionsResult,
    aml: &AmlResult,
    pep: &PepResult,
    status: &ComplianceStatus,
) -> Vec<RequiredAction> {
    let mut actions = Vec::new();

    if sanctions.is_sanctioned {
        actions.push(RequiredAction::BlockTransaction);
        actions.push(RequiredAction::NotifyAuthorities);
    }

    if aml.requires_sar {
        actions.push(RequiredAction::FileSAR);
    }

    if aml.requires_ctr {
        actions.push(RequiredAction::FileCTR);
    }

    if pep.is_pep {
        actions.push(RequiredAction::EnhancedDueDiligence);
    }

    if matches!(status, ComplianceStatus::ReviewRequired | ComplianceStatus::Hold) {
        actions.push(RequiredAction::ManualReview);
    }

    if aml.risk_score > 85.0 {
        actions.push(RequiredAction::SeniorApproval);
    }

    actions
}

async fn save_compliance_check(
    result: &ComplianceCheckResult,
    pool: &PgPool,
) -> Result<(), ComplianceError> {
    // Save to compliance_checks table
    sqlx::query(
        "INSERT INTO compliance_checks
         (transaction_id, overall_status, risk_rating, sanctions_result, aml_result, pep_result, pattern_result, required_actions, checked_at)
         VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)"
    )
    .bind(result.transaction_id)
    .bind(format!("{:?}", result.overall_status))
    .bind(format!("{:?}", result.risk_rating))
    .bind(serde_json::to_value(&result.sanctions_check).unwrap())
    .bind(serde_json::to_value(&result.aml_check).unwrap())
    .bind(serde_json::to_value(&result.pep_check).unwrap())
    .bind(serde_json::to_value(&result.pattern_analysis).unwrap())
    .bind(result.required_actions.iter().map(|a| format!("{:?}", a)).collect::<Vec<_>>())
    .bind(result.checked_at)
    .execute(pool)
    .await?;

    Ok(())
}
