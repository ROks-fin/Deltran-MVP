import logging
from datetime import datetime
from typing import Optional, Dict, Any, List
from enum import Enum

from fastapi import APIRouter, HTTPException
from pydantic import BaseModel

from shared.utils.errors import RiskError, ErrorCode
from shared.utils.tracing import trace_operation
from shared.clients.nats_client import nats_client
from shared.clients.postgres_client import postgres_client
from shared.clients.redis_client import redis_client
from gateway.middleware.metrics import record_risk_metric

logger = logging.getLogger(__name__)
router = APIRouter()


class RiskMode(str, Enum):
    LOW = "Low"
    MEDIUM = "Medium"
    HIGH = "High"


class RiskThresholds(BaseModel):
    spread_threshold: float
    depth_threshold: float
    deviation_threshold: float
    latency_threshold_ms: int
    volume_threshold_usd: float


class RiskModeResponse(BaseModel):
    current_mode: RiskMode
    thresholds: RiskThresholds
    last_changed: Optional[datetime] = None
    changed_by: Optional[str] = None
    auto_escalation: bool = True


class RiskModeUpdateRequest(BaseModel):
    mode: RiskMode
    reason: Optional[str] = None
    auto_escalation: bool = True


class RiskMetrics(BaseModel):
    spread: float
    depth: float
    deviation: float
    latency_ms: int
    volume_24h_usd: float
    risk_score: float


class RiskAssessmentResponse(BaseModel):
    transaction_id: str
    risk_score: float
    risk_factors: List[str]
    recommended_action: str
    assessment_time: datetime


# Risk mode thresholds configuration
RISK_THRESHOLDS = {
    RiskMode.LOW: RiskThresholds(
        spread_threshold=0.001,
        depth_threshold=1000000,
        deviation_threshold=0.05,
        latency_threshold_ms=100,
        volume_threshold_usd=10000000
    ),
    RiskMode.MEDIUM: RiskThresholds(
        spread_threshold=0.005,
        depth_threshold=500000,
        deviation_threshold=0.10,
        latency_threshold_ms=200,
        volume_threshold_usd=5000000
    ),
    RiskMode.HIGH: RiskThresholds(
        spread_threshold=0.01,
        depth_threshold=100000,
        deviation_threshold=0.20,
        latency_threshold_ms=500,
        volume_threshold_usd=1000000
    )
}


@router.get("/mode", response_model=RiskModeResponse)
@trace_operation("risk_get_mode")
async def get_risk_mode():
    """Get current risk management mode"""

    try:
        # Get current mode from Redis cache
        cached_mode = await redis_client.get("risk:current_mode")

        if cached_mode:
            return RiskModeResponse(**cached_mode)

        # Fallback to database
        risk_config = await postgres_client.fetchrow(
            "SELECT * FROM risk_config WHERE is_active = true ORDER BY updated_at DESC LIMIT 1"
        )

        if not risk_config:
            # Default to Medium mode
            current_mode = RiskMode.MEDIUM
            thresholds = RISK_THRESHOLDS[current_mode]

            response = RiskModeResponse(
                current_mode=current_mode,
                thresholds=thresholds,
                auto_escalation=True
            )

            # Cache the default
            await redis_client.set("risk:current_mode", response.dict(), expire=300)
            return response

        current_mode = RiskMode(risk_config["mode"])
        response = RiskModeResponse(
            current_mode=current_mode,
            thresholds=RISK_THRESHOLDS[current_mode],
            last_changed=risk_config["updated_at"],
            changed_by=risk_config["changed_by"],
            auto_escalation=risk_config["auto_escalation"]
        )

        # Cache the response
        await redis_client.set("risk:current_mode", response.dict(), expire=300)
        return response

    except Exception as e:
        logger.error(f"Failed to get risk mode: {e}")
        raise RiskError("Failed to get risk mode", ErrorCode.RISK_ASSESSMENT_FAILED)


@router.post("/mode", response_model=RiskModeResponse)
@trace_operation("risk_update_mode")
async def update_risk_mode(request: RiskModeUpdateRequest):
    """Update risk management mode"""

    try:
        logger.info(f"Updating risk mode to: {request.mode}")

        # Store in database
        config_data = {
            "mode": request.mode.value,
            "reason": request.reason or "Manual update",
            "changed_by": "system",  # In production, get from auth context
            "auto_escalation": request.auto_escalation,
            "is_active": True,
            "updated_at": datetime.utcnow()
        }

        # Deactivate previous configs
        await postgres_client.execute(
            "UPDATE risk_config SET is_active = false WHERE is_active = true"
        )

        # Insert new config
        await postgres_client.insert_returning("risk_config", config_data)

        response = RiskModeResponse(
            current_mode=request.mode,
            thresholds=RISK_THRESHOLDS[request.mode],
            last_changed=datetime.utcnow(),
            changed_by="system",
            auto_escalation=request.auto_escalation
        )

        # Update cache
        await redis_client.set("risk:current_mode", response.dict(), expire=300)

        # Publish mode change event
        await nats_client.publish(
            "risk.mode_changed",
            {
                "new_mode": request.mode.value,
                "reason": request.reason,
                "timestamp": datetime.utcnow().isoformat(),
                "thresholds": RISK_THRESHOLDS[request.mode].dict()
            }
        )

        logger.info(f"Risk mode updated to: {request.mode}")
        return response

    except Exception as e:
        logger.error(f"Failed to update risk mode: {e}")
        raise RiskError("Failed to update risk mode", ErrorCode.RISK_ASSESSMENT_FAILED)


@router.get("/metrics", response_model=RiskMetrics)
@trace_operation("risk_get_metrics")
async def get_risk_metrics():
    """Get current risk metrics"""

    try:
        # Get cached metrics first
        cached_metrics = await redis_client.cache_get("risk:metrics")
        if cached_metrics:
            return RiskMetrics(**cached_metrics)

        # Calculate metrics from recent data
        metrics = await _calculate_risk_metrics()

        # Cache for 1 minute
        await redis_client.cache_set("risk:metrics", metrics.dict(), expire=60)

        # Record metric for monitoring
        record_risk_metric(metrics.risk_score)

        return metrics

    except Exception as e:
        logger.error(f"Failed to get risk metrics: {e}")
        raise RiskError("Failed to get risk metrics", ErrorCode.RISK_ASSESSMENT_FAILED)


@router.post("/assess/{transaction_id}", response_model=RiskAssessmentResponse)
@trace_operation("risk_assess_transaction")
async def assess_transaction_risk(transaction_id: str):
    """Assess risk for a specific transaction"""

    try:
        logger.info(f"Assessing risk for transaction: {transaction_id}")

        # Get transaction details
        transaction = await postgres_client.fetchrow(
            "SELECT * FROM payments WHERE transaction_id = $1",
            transaction_id
        )

        if not transaction:
            raise HTTPException(status_code=404, detail="Transaction not found")

        # Calculate risk score
        risk_assessment = await _assess_transaction_risk(transaction)

        # Store assessment
        assessment_data = {
            "transaction_id": transaction_id,
            "risk_score": risk_assessment["risk_score"],
            "risk_factors": risk_assessment["risk_factors"],
            "recommended_action": risk_assessment["recommended_action"],
            "assessed_at": datetime.utcnow()
        }

        await postgres_client.insert_returning("risk_assessments", assessment_data)

        # Publish assessment event
        await nats_client.publish(
            "risk.assessment_completed",
            {
                "transaction_id": transaction_id,
                "risk_score": risk_assessment["risk_score"],
                "risk_factors": risk_assessment["risk_factors"],
                "recommended_action": risk_assessment["recommended_action"]
            }
        )

        return RiskAssessmentResponse(
            transaction_id=transaction_id,
            risk_score=risk_assessment["risk_score"],
            risk_factors=risk_assessment["risk_factors"],
            recommended_action=risk_assessment["recommended_action"],
            assessment_time=datetime.utcnow()
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to assess transaction risk: {e}")
        raise RiskError("Failed to assess transaction risk", ErrorCode.RISK_ASSESSMENT_FAILED)


@router.get("/thresholds")
@trace_operation("risk_get_thresholds")
async def get_risk_thresholds():
    """Get risk thresholds for all modes"""
    return {mode.value: thresholds.dict() for mode, thresholds in RISK_THRESHOLDS.items()}


async def _calculate_risk_metrics() -> RiskMetrics:
    """Calculate current system risk metrics"""

    # Get recent market data (last 1 hour)
    recent_quotes = await postgres_client.fetch(
        """
        SELECT spread, amount, latency_ms, created_at
        FROM liquidity_quotes
        WHERE created_at >= NOW() - INTERVAL '1 hour'
        """
    )

    if not recent_quotes:
        # Return default metrics if no recent data
        return RiskMetrics(
            spread=0.002,
            depth=1000000,
            deviation=0.05,
            latency_ms=80,
            volume_24h_usd=5000000,
            risk_score=25.0
        )

    # Calculate metrics
    spreads = [float(q["spread"]) for q in recent_quotes]
    latencies = [q["latency_ms"] for q in recent_quotes]
    amounts = [float(q["amount"]) for q in recent_quotes]

    avg_spread = sum(spreads) / len(spreads) if spreads else 0.002
    avg_latency = sum(latencies) / len(latencies) if latencies else 80
    total_volume = sum(amounts)

    # Calculate deviation (simplified)
    if len(spreads) > 1:
        mean_spread = avg_spread
        variance = sum((s - mean_spread) ** 2 for s in spreads) / len(spreads)
        deviation = (variance ** 0.5) / mean_spread if mean_spread > 0 else 0
    else:
        deviation = 0.05

    # Calculate risk score (0-100)
    current_mode_data = await redis_client.get("risk:current_mode")
    if current_mode_data:
        current_mode = RiskMode(current_mode_data.get("current_mode", "Medium"))
    else:
        current_mode = RiskMode.MEDIUM

    thresholds = RISK_THRESHOLDS[current_mode]

    # Risk score based on threshold breaches
    risk_score = 0
    if avg_spread > thresholds.spread_threshold:
        risk_score += 25
    if deviation > thresholds.deviation_threshold:
        risk_score += 25
    if avg_latency > thresholds.latency_threshold_ms:
        risk_score += 25
    if total_volume > thresholds.volume_threshold_usd:
        risk_score += 25

    return RiskMetrics(
        spread=avg_spread,
        depth=total_volume,
        deviation=deviation,
        latency_ms=int(avg_latency),
        volume_24h_usd=total_volume,
        risk_score=float(risk_score)
    )


async def _assess_transaction_risk(transaction) -> Dict[str, Any]:
    """Assess risk for a specific transaction"""

    risk_factors = []
    risk_score = 0

    amount = float(transaction["amount"])
    currency = transaction["currency"]

    # High value transactions
    if amount > 100000:
        risk_factors.append("HIGH_VALUE")
        risk_score += 20

    # High-risk currencies
    high_risk_currencies = ["AED", "INR", "CNY"]
    if currency in high_risk_currencies:
        risk_factors.append("HIGH_RISK_CURRENCY")
        risk_score += 15

    # Check for unusual patterns (simplified)
    recent_transactions = await postgres_client.fetch(
        """
        SELECT COUNT(*) as count, SUM(CAST(amount AS DECIMAL)) as total
        FROM payments
        WHERE debtor_account = $1
        AND created_at >= NOW() - INTERVAL '24 hours'
        """,
        transaction["debtor_account"]
    )

    if recent_transactions and recent_transactions[0]["count"] > 10:
        risk_factors.append("HIGH_FREQUENCY")
        risk_score += 10

    # Weekend/holiday transactions
    if datetime.utcnow().weekday() >= 5:  # Saturday or Sunday
        risk_factors.append("WEEKEND_TRANSACTION")
        risk_score += 5

    # Determine recommended action
    if risk_score >= 40:
        recommended_action = "MANUAL_REVIEW"
    elif risk_score >= 20:
        recommended_action = "ENHANCED_MONITORING"
    else:
        recommended_action = "APPROVE"

    return {
        "risk_score": float(risk_score),
        "risk_factors": risk_factors,
        "recommended_action": recommended_action
    }