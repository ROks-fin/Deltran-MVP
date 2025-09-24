import logging
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any
from uuid import UUID

from fastapi import APIRouter, Query
from pydantic import BaseModel

from shared.utils.errors import ErrorCode, DeltranError
from shared.utils.tracing import trace_operation
from shared.clients.postgres_client import postgres_client
from shared.clients.nats_client import nats_client

logger = logging.getLogger(__name__)
router = APIRouter()


class ProofOfReservesResponse(BaseModel):
    report_id: str
    generated_at: datetime
    total_reserves_usd: float
    total_liabilities_usd: float
    reserve_ratio: float
    currencies: Dict[str, Dict[str, float]]
    attestation_hash: str
    valid_until: datetime


class ProofOfSettlementResponse(BaseModel):
    report_id: str
    settlement_date: str
    generated_at: datetime
    total_settled_transactions: int
    total_settled_amount_usd: float
    settlement_batches: List[Dict[str, Any]]
    iso20022_manifest: Dict[str, Any]
    merkle_root: str
    block_references: List[str]


class TransactionReport(BaseModel):
    transaction_id: str
    uetr: str
    amount: float
    currency: str
    status: str
    created_at: datetime
    settled_at: Optional[datetime] = None
    risk_score: Optional[float] = None


class ComplianceReport(BaseModel):
    report_id: str
    period_start: datetime
    period_end: datetime
    total_transactions: int
    travel_rule_applicable: int
    sanctions_hits: int
    pep_matches: int
    manual_reviews: int
    compliance_rate: float


@router.get("/proof-of-reserves", response_model=ProofOfReservesResponse)
@trace_operation("reports_proof_of_reserves")
async def generate_proof_of_reserves():
    """Generate proof of reserves report"""

    try:
        logger.info("Generating proof of reserves report")

        from shared.utils.uuidv7 import generate_uuidv7
        report_id = str(generate_uuidv7())

        # Get current balances by currency
        balances_query = """
            SELECT
                currency,
                SUM(CASE WHEN status = 'COMPLETED' THEN CAST(amount AS DECIMAL) ELSE 0 END) as settled_amount,
                COUNT(CASE WHEN status IN ('INITIATED', 'VALIDATED', 'APPROVED') THEN 1 END) as pending_count,
                SUM(CASE WHEN status IN ('INITIATED', 'VALIDATED', 'APPROVED') THEN CAST(amount AS DECIMAL) ELSE 0 END) as pending_amount
            FROM payments
            WHERE created_at >= NOW() - INTERVAL '30 days'
            GROUP BY currency
        """

        currency_balances = await postgres_client.fetch(balances_query)

        # Mock exchange rates for USD conversion
        usd_rates = {
            "USD": 1.0,
            "EUR": 1.18,
            "GBP": 1.33,
            "JPY": 0.009,
            "AED": 0.27,
            "INR": 0.012
        }

        currencies = {}
        total_reserves_usd = 0
        total_liabilities_usd = 0

        for balance in currency_balances:
            currency = balance["currency"]
            settled = float(balance["settled_amount"] or 0)
            pending = float(balance["pending_amount"] or 0)
            rate = usd_rates.get(currency, 1.0)

            # Mock reserve calculations
            reserves = settled * 1.1  # 110% reserve ratio
            liabilities = pending

            currencies[currency] = {
                "settled_amount": settled,
                "pending_amount": pending,
                "reserves": reserves,
                "liabilities": liabilities,
                "reserve_ratio": reserves / liabilities if liabilities > 0 else float('inf'),
                "usd_value_reserves": reserves * rate,
                "usd_value_liabilities": liabilities * rate
            }

            total_reserves_usd += reserves * rate
            total_liabilities_usd += liabilities * rate

        overall_ratio = total_reserves_usd / total_liabilities_usd if total_liabilities_usd > 0 else float('inf')

        # Generate attestation hash (simplified)
        import hashlib
        attestation_data = f"{report_id}{total_reserves_usd}{total_liabilities_usd}{datetime.utcnow().isoformat()}"
        attestation_hash = hashlib.sha256(attestation_data.encode()).hexdigest()

        response = ProofOfReservesResponse(
            report_id=report_id,
            generated_at=datetime.utcnow(),
            total_reserves_usd=total_reserves_usd,
            total_liabilities_usd=total_liabilities_usd,
            reserve_ratio=overall_ratio,
            currencies=currencies,
            attestation_hash=attestation_hash,
            valid_until=datetime.utcnow() + timedelta(hours=24)
        )

        # Store report in database
        report_data = {
            "report_id": report_id,
            "report_type": "PROOF_OF_RESERVES",
            "data": response.dict(),
            "generated_at": datetime.utcnow()
        }
        await postgres_client.insert_returning("reports", report_data)

        # Publish report event
        await nats_client.publish(
            "reports.proof_of_reserves_generated",
            {
                "report_id": report_id,
                "total_reserves_usd": total_reserves_usd,
                "reserve_ratio": overall_ratio
            }
        )

        logger.info(f"Proof of reserves report generated: {report_id}")
        return response

    except Exception as e:
        logger.error(f"Failed to generate proof of reserves: {e}")
        raise DeltranError("Failed to generate proof of reserves", ErrorCode.INTERNAL_ERROR)


@router.get("/proof-of-settlement", response_model=ProofOfSettlementResponse)
@trace_operation("reports_proof_of_settlement")
async def generate_proof_of_settlement(
    settlement_date: Optional[str] = Query(None, description="Settlement date (YYYY-MM-DD), defaults to today")
):
    """Generate proof of settlement report with ISO20022 manifest"""

    try:
        if settlement_date:
            target_date = datetime.strptime(settlement_date, "%Y-%m-%d").date()
        else:
            target_date = datetime.utcnow().date()

        logger.info(f"Generating proof of settlement for: {target_date}")

        from shared.utils.uuidv7 import generate_uuidv7
        report_id = str(generate_uuidv7())

        # Get settled transactions for the date
        settled_transactions_query = """
            SELECT
                p.transaction_id,
                p.uetr,
                p.amount,
                p.currency,
                p.settlement_batch_id,
                sb.closed_at,
                sb.window
            FROM payments p
            JOIN settlement_batches sb ON p.settlement_batch_id = sb.batch_id
            WHERE DATE(sb.closed_at) = $1
            AND p.status = 'SETTLED'
            ORDER BY sb.closed_at
        """

        transactions = await postgres_client.fetch(settled_transactions_query, target_date)

        # Group by settlement batch
        batches = {}
        total_amount_usd = 0
        usd_rates = {"USD": 1.0, "EUR": 1.18, "GBP": 1.33, "AED": 0.27, "INR": 0.012}

        for txn in transactions:
            batch_id = txn["settlement_batch_id"]
            if batch_id not in batches:
                batches[batch_id] = {
                    "batch_id": batch_id,
                    "window": txn["window"],
                    "closed_at": txn["closed_at"].isoformat(),
                    "transactions": [],
                    "total_amount_usd": 0
                }

            amount_usd = float(txn["amount"]) * usd_rates.get(txn["currency"], 1.0)
            batches[batch_id]["transactions"].append({
                "transaction_id": txn["transaction_id"],
                "uetr": txn["uetr"],
                "amount": float(txn["amount"]),
                "currency": txn["currency"],
                "amount_usd": amount_usd
            })
            batches[batch_id]["total_amount_usd"] += amount_usd
            total_amount_usd += amount_usd

        settlement_batches = list(batches.values())

        # Generate ISO20022 manifest
        iso20022_manifest = {
            "message_type": "camt.053.001.08",
            "creation_date_time": datetime.utcnow().isoformat(),
            "number_of_transactions": len(transactions),
            "control_sum": total_amount_usd,
            "settlement_method": "NETTING",
            "currency_breakdown": _calculate_currency_breakdown(transactions),
            "batch_references": [batch["batch_id"] for batch in settlement_batches]
        }

        # Generate Merkle root (simplified)
        merkle_root = _calculate_merkle_root([txn["transaction_id"] for txn in transactions])

        # Mock block references
        block_references = [f"block_{i}_{target_date}" for i in range(len(settlement_batches))]

        response = ProofOfSettlementResponse(
            report_id=report_id,
            settlement_date=target_date.isoformat(),
            generated_at=datetime.utcnow(),
            total_settled_transactions=len(transactions),
            total_settled_amount_usd=total_amount_usd,
            settlement_batches=settlement_batches,
            iso20022_manifest=iso20022_manifest,
            merkle_root=merkle_root,
            block_references=block_references
        )

        # Store report
        report_data = {
            "report_id": report_id,
            "report_type": "PROOF_OF_SETTLEMENT",
            "data": response.dict(),
            "generated_at": datetime.utcnow()
        }
        await postgres_client.insert_returning("reports", report_data)

        logger.info(f"Proof of settlement report generated: {report_id} for {target_date}")
        return response

    except Exception as e:
        logger.error(f"Failed to generate proof of settlement: {e}")
        raise DeltranError("Failed to generate proof of settlement", ErrorCode.INTERNAL_ERROR)


@router.get("/transactions")
@trace_operation("reports_transactions")
async def get_transaction_report(
    start_date: Optional[str] = Query(None, description="Start date (YYYY-MM-DD)"),
    end_date: Optional[str] = Query(None, description="End date (YYYY-MM-DD)"),
    currency: Optional[str] = Query(None, description="Filter by currency"),
    status: Optional[str] = Query(None, description="Filter by status"),
    limit: int = Query(100, ge=1, le=1000, description="Number of transactions to return")
):
    """Get transaction report with filtering"""

    try:
        # Build query with filters
        where_conditions = ["1=1"]
        query_params = []

        if start_date:
            where_conditions.append("created_at >= $" + str(len(query_params) + 1))
            query_params.append(datetime.strptime(start_date, "%Y-%m-%d"))

        if end_date:
            where_conditions.append("created_at <= $" + str(len(query_params) + 1))
            query_params.append(datetime.strptime(end_date, "%Y-%m-%d") + timedelta(days=1))

        if currency:
            where_conditions.append("currency = $" + str(len(query_params) + 1))
            query_params.append(currency)

        if status:
            where_conditions.append("status = $" + str(len(query_params) + 1))
            query_params.append(status)

        query = f"""
            SELECT
                p.transaction_id,
                p.uetr,
                p.amount,
                p.currency,
                p.status,
                p.created_at,
                p.updated_at as settled_at,
                ra.risk_score
            FROM payments p
            LEFT JOIN risk_assessments ra ON p.transaction_id = ra.transaction_id
            WHERE {' AND '.join(where_conditions)}
            ORDER BY p.created_at DESC
            LIMIT ${len(query_params) + 1}
        """

        query_params.append(limit)

        transactions = await postgres_client.fetch(query, *query_params)

        transaction_reports = []
        for txn in transactions:
            transaction_reports.append(TransactionReport(
                transaction_id=txn["transaction_id"],
                uetr=txn["uetr"],
                amount=float(txn["amount"]),
                currency=txn["currency"],
                status=txn["status"],
                created_at=txn["created_at"],
                settled_at=txn["settled_at"],
                risk_score=float(txn["risk_score"]) if txn["risk_score"] else None
            ))

        return {
            "transactions": transaction_reports,
            "total_count": len(transaction_reports),
            "filters": {
                "start_date": start_date,
                "end_date": end_date,
                "currency": currency,
                "status": status
            }
        }

    except Exception as e:
        logger.error(f"Failed to generate transaction report: {e}")
        raise DeltranError("Failed to generate transaction report", ErrorCode.INTERNAL_ERROR)


@router.get("/compliance", response_model=ComplianceReport)
@trace_operation("reports_compliance")
async def get_compliance_report(
    start_date: Optional[str] = Query(None, description="Start date (YYYY-MM-DD)"),
    end_date: Optional[str] = Query(None, description="End date (YYYY-MM-DD)")
):
    """Generate compliance report"""

    try:
        if not start_date:
            start_date = (datetime.utcnow() - timedelta(days=30)).strftime("%Y-%m-%d")
        if not end_date:
            end_date = datetime.utcnow().strftime("%Y-%m-%d")

        period_start = datetime.strptime(start_date, "%Y-%m-%d")
        period_end = datetime.strptime(end_date, "%Y-%m-%d") + timedelta(days=1)

        logger.info(f"Generating compliance report for {start_date} to {end_date}")

        # Get compliance statistics
        compliance_stats = await postgres_client.fetchrow(
            """
            SELECT
                COUNT(*) as total_transactions,
                COUNT(CASE WHEN CAST(amount AS DECIMAL) >= 1000 THEN 1 END) as travel_rule_applicable,
                COUNT(CASE WHEN ra.risk_factors @> '["SANCTIONS_HIT"]' THEN 1 END) as sanctions_hits,
                COUNT(CASE WHEN ra.risk_factors @> '["PEP_MATCH"]' THEN 1 END) as pep_matches,
                COUNT(CASE WHEN ra.recommended_action = 'MANUAL_REVIEW' THEN 1 END) as manual_reviews
            FROM payments p
            LEFT JOIN risk_assessments ra ON p.transaction_id = ra.transaction_id
            WHERE p.created_at >= $1 AND p.created_at < $2
            """,
            period_start,
            period_end
        )

        total_txns = compliance_stats["total_transactions"]
        travel_rule_applicable = compliance_stats["travel_rule_applicable"]
        sanctions_hits = compliance_stats["sanctions_hits"] or 0
        pep_matches = compliance_stats["pep_matches"] or 0
        manual_reviews = compliance_stats["manual_reviews"] or 0

        # Calculate compliance rate
        compliance_issues = sanctions_hits + pep_matches
        compliance_rate = (total_txns - compliance_issues) / total_txns * 100 if total_txns > 0 else 100

        from shared.utils.uuidv7 import generate_uuidv7
        report_id = str(generate_uuidv7())

        return ComplianceReport(
            report_id=report_id,
            period_start=period_start,
            period_end=period_end - timedelta(days=1),
            total_transactions=total_txns,
            travel_rule_applicable=travel_rule_applicable,
            sanctions_hits=sanctions_hits,
            pep_matches=pep_matches,
            manual_reviews=manual_reviews,
            compliance_rate=compliance_rate
        )

    except Exception as e:
        logger.error(f"Failed to generate compliance report: {e}")
        raise DeltranError("Failed to generate compliance report", ErrorCode.INTERNAL_ERROR)


def _calculate_currency_breakdown(transactions) -> Dict[str, float]:
    """Calculate currency breakdown for ISO20022 manifest"""
    breakdown = {}
    for txn in transactions:
        currency = txn["currency"]
        amount = float(txn["amount"])
        breakdown[currency] = breakdown.get(currency, 0) + amount
    return breakdown


def _calculate_merkle_root(transaction_ids: List[str]) -> str:
    """Calculate Merkle root for transaction set (simplified)"""
    import hashlib

    if not transaction_ids:
        return ""

    # Simple hash of all transaction IDs
    combined = "".join(sorted(transaction_ids))
    return hashlib.sha256(combined.encode()).hexdigest()