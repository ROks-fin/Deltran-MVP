import logging
from datetime import datetime
from typing import Optional, List, Dict, Any
from enum import Enum

from fastapi import APIRouter, HTTPException, Query
from pydantic import BaseModel

from shared.utils.errors import SettlementError, ErrorCode
from shared.utils.tracing import trace_operation
from shared.clients.nats_client import nats_client
from shared.clients.postgres_client import postgres_client
from gateway.middleware.metrics import record_settlement_metric

logger = logging.getLogger(__name__)
router = APIRouter()


class SettlementWindow(str, Enum):
    INTRADAY = "intraday"
    EOD = "EOD"


class BatchCloseRequest(BaseModel):
    window: SettlementWindow
    force: bool = False


class BatchCloseResponse(BaseModel):
    batch_id: str
    window: str
    total_transactions: int
    total_amount: float
    net_positions: List[Dict[str, Any]]
    closed_at: datetime


class SettlementStatusResponse(BaseModel):
    current_batch: Optional[Dict[str, Any]] = None
    completed_batches: List[Dict[str, Any]] = []
    net_positions: List[Dict[str, Any]] = []


@router.post("/close-batch", response_model=BatchCloseResponse)
@trace_operation("settlement_close_batch")
async def close_settlement_batch(
    window: SettlementWindow = Query(..., description="Settlement window (intraday or EOD)")
):
    """Close current settlement batch and perform netting"""

    try:
        logger.info(f"Closing settlement batch for window: {window}")

        # Get current batch transactions
        batch_query = """
            SELECT
                transaction_id,
                amount,
                currency,
                debtor_account,
                creditor_account,
                settlement_method
            FROM payments
            WHERE status = 'APPROVED'
            AND settlement_batch_id IS NULL
            AND created_at >= CASE
                WHEN $1 = 'intraday' THEN NOW() - INTERVAL '4 hours'
                ELSE DATE_TRUNC('day', NOW())
            END
        """

        transactions = await postgres_client.fetch(batch_query, window.value)

        if not transactions:
            logger.info("No transactions to settle")
            return BatchCloseResponse(
                batch_id="",
                window=window.value,
                total_transactions=0,
                total_amount=0.0,
                net_positions=[],
                closed_at=datetime.utcnow()
            )

        # Generate batch ID
        from shared.utils.uuidv7 import generate_uuidv7
        batch_id = str(generate_uuidv7())

        # Calculate net positions
        net_positions = await _calculate_net_positions(transactions)

        # Store batch in database
        batch_data = {
            "batch_id": batch_id,
            "window": window.value,
            "total_transactions": len(transactions),
            "total_amount": sum(float(t["amount"]) for t in transactions),
            "net_positions": net_positions,
            "closed_at": datetime.utcnow(),
            "status": "CLOSED"
        }

        await postgres_client.insert_returning("settlement_batches", batch_data)

        # Update transactions with batch ID
        transaction_ids = [t["transaction_id"] for t in transactions]
        await postgres_client.execute(
            """
            UPDATE payments
            SET settlement_batch_id = $1,
                status = 'SETTLED',
                updated_at = $2
            WHERE transaction_id = ANY($3)
            """,
            batch_id,
            datetime.utcnow(),
            transaction_ids
        )

        # Publish settlement event
        await nats_client.publish(
            "settlement.batch_closed",
            {
                "batch_id": batch_id,
                "window": window.value,
                "transaction_count": len(transactions),
                "net_positions": net_positions
            }
        )

        # Record metrics
        record_settlement_metric(len(transactions), window.value)

        logger.info(f"Settlement batch closed: {batch_id} with {len(transactions)} transactions")

        return BatchCloseResponse(
            batch_id=batch_id,
            window=window.value,
            total_transactions=len(transactions),
            total_amount=sum(float(t["amount"]) for t in transactions),
            net_positions=net_positions,
            closed_at=datetime.utcnow()
        )

    except Exception as e:
        logger.error(f"Failed to close settlement batch: {e}")
        raise SettlementError("Failed to close settlement batch", ErrorCode.SETTLEMENT_FAILED)


@router.get("/status", response_model=SettlementStatusResponse)
@trace_operation("settlement_status")
async def get_settlement_status():
    """Get current settlement status and net positions"""

    try:
        # Get current batch info
        current_batch_query = """
            SELECT
                COUNT(*) as transaction_count,
                SUM(CAST(amount AS DECIMAL)) as total_amount,
                MIN(created_at) as oldest_transaction
            FROM payments
            WHERE status = 'APPROVED'
            AND settlement_batch_id IS NULL
        """

        current_batch = await postgres_client.fetchrow(current_batch_query)

        # Get recent completed batches
        completed_batches_query = """
            SELECT
                batch_id,
                window,
                total_transactions,
                total_amount,
                closed_at
            FROM settlement_batches
            ORDER BY closed_at DESC
            LIMIT 10
        """

        completed_batches = await postgres_client.fetch(completed_batches_query)

        # Calculate current net positions
        current_positions_query = """
            SELECT
                debtor_account,
                creditor_account,
                currency,
                SUM(CAST(amount AS DECIMAL)) as amount
            FROM payments
            WHERE status = 'APPROVED'
            AND settlement_batch_id IS NULL
            GROUP BY debtor_account, creditor_account, currency
        """

        positions = await postgres_client.fetch(current_positions_query)
        net_positions = await _calculate_net_positions(positions) if positions else []

        return SettlementStatusResponse(
            current_batch={
                "transaction_count": current_batch["transaction_count"] or 0,
                "total_amount": float(current_batch["total_amount"] or 0),
                "oldest_transaction": current_batch["oldest_transaction"]
            } if current_batch["transaction_count"] else None,
            completed_batches=[dict(batch) for batch in completed_batches],
            net_positions=net_positions
        )

    except Exception as e:
        logger.error(f"Failed to get settlement status: {e}")
        raise SettlementError("Failed to get settlement status", ErrorCode.SETTLEMENT_FAILED)


@router.get("/batches/{batch_id}")
@trace_operation("settlement_batch_details")
async def get_batch_details(batch_id: str):
    """Get details of a specific settlement batch"""

    try:
        # Get batch details
        batch = await postgres_client.fetchrow(
            "SELECT * FROM settlement_batches WHERE batch_id = $1",
            batch_id
        )

        if not batch:
            raise HTTPException(status_code=404, detail="Batch not found")

        # Get batch transactions
        transactions = await postgres_client.fetch(
            """
            SELECT
                transaction_id,
                uetr,
                amount,
                currency,
                debtor_account,
                creditor_account,
                status
            FROM payments
            WHERE settlement_batch_id = $1
            """,
            batch_id
        )

        return {
            "batch": dict(batch),
            "transactions": [dict(t) for t in transactions]
        }

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get batch details: {e}")
        raise SettlementError("Failed to get batch details", ErrorCode.SETTLEMENT_FAILED)


async def _calculate_net_positions(transactions) -> List[Dict[str, Any]]:
    """Calculate net positions using simplified netting algorithm"""

    # Group positions by account and currency
    positions = {}

    for txn in transactions:
        debtor = txn["debtor_account"]
        creditor = txn["creditor_account"]
        currency = txn["currency"]
        amount = float(txn["amount"])

        key_debtor = f"{debtor}:{currency}"
        key_creditor = f"{creditor}:{currency}"

        # Debtor position (negative)
        if key_debtor not in positions:
            positions[key_debtor] = {
                "account": debtor,
                "currency": currency,
                "amount": 0.0
            }
        positions[key_debtor]["amount"] -= amount

        # Creditor position (positive)
        if key_creditor not in positions:
            positions[key_creditor] = {
                "account": creditor,
                "currency": currency,
                "amount": 0.0
            }
        positions[key_creditor]["amount"] += amount

    # Filter out zero positions and add settlement instructions
    net_positions = []
    for position in positions.values():
        if abs(position["amount"]) > 0.01:  # Ignore small rounding differences
            position["settlement_instruction"] = (
                "PAY" if position["amount"] < 0 else "RECEIVE"
            )
            position["amount"] = abs(position["amount"])
            net_positions.append(position)

    return net_positions