import logging
from datetime import datetime
from typing import Optional
from uuid import UUID

from fastapi import APIRouter, HTTPException, Request, Depends, Header
from pydantic import BaseModel

from shared.models.ctr import CTRCreateRequest, CTRResponse, CTRStatusResponse, TransactionStatus
from shared.utils.uuidv7 import generate_uuidv7, generate_uetr
from shared.utils.errors import ValidationError, PaymentError, ErrorCode
from shared.utils.tracing import trace_operation, get_current_trace_id
from shared.clients.nats_client import nats_client
from shared.clients.postgres_client import postgres_client
from shared.clients.redis_client import redis_client
from gateway.middleware.metrics import record_payment_metric

logger = logging.getLogger(__name__)
router = APIRouter()


class PaymentInitiateRequest(BaseModel):
    amount: str
    currency: str
    debtor_account: str
    creditor_account: str
    payment_purpose: Optional[str] = "TRADE"
    settlement_method: Optional[str] = "PVP"


@router.post("/initiate", response_model=CTRResponse)
@trace_operation("payment_initiate")
async def initiate_payment(
    request: PaymentInitiateRequest,
    http_request: Request,
    idempotency_key: str = Header(..., alias="Idempotency-Key")
):
    """Initiate a new payment transaction"""

    try:
        # Generate transaction identifiers
        transaction_id = generate_uuidv7()
        uetr = generate_uetr()
        trace_id = get_current_trace_id()

        logger.info(f"Initiating payment: {transaction_id}")

        # Validate request
        if not request.amount or float(request.amount) <= 0:
            raise ValidationError("Amount must be positive", field="amount")

        if len(request.currency) != 3:
            raise ValidationError("Currency must be 3-letter ISO code", field="currency")

        # TODO: Validate account formats (IBAN, BIC, etc.)

        # Create payment record in database
        payment_data = {
            "transaction_id": str(transaction_id),
            "uetr": str(uetr),
            "amount": request.amount,
            "currency": request.currency,
            "debtor_account": request.debtor_account,
            "creditor_account": request.creditor_account,
            "payment_purpose": request.payment_purpose,
            "settlement_method": request.settlement_method,
            "status": TransactionStatus.INITIATED,
            "created_at": datetime.utcnow(),
            "trace_id": trace_id,
            "idempotency_key": idempotency_key
        }

        # Store in database
        await postgres_client.insert_returning(
            "payments",
            payment_data
        )

        # Publish to NATS for processing
        await nats_client.publish(
            "payment.initiated",
            {
                "transaction_id": str(transaction_id),
                "uetr": str(uetr),
                "payment_data": payment_data
            }
        )

        # Record metrics
        record_payment_metric(
            amount_usd=float(request.amount),
            currency=request.currency,
            status="initiated",
            settlement_method=request.settlement_method
        )

        logger.info(f"Payment initiated successfully: {transaction_id}")

        return CTRResponse(
            transaction_id=transaction_id,
            uetr=uetr,
            status=TransactionStatus.INITIATED,
            timestamp=datetime.utcnow(),
            message="Payment initiated successfully"
        )

    except ValidationError:
        raise
    except Exception as e:
        logger.error(f"Failed to initiate payment: {e}")
        raise PaymentError("Failed to initiate payment", ErrorCode.INTERNAL_ERROR)


@router.get("/{payment_id}/status", response_model=CTRStatusResponse)
@trace_operation("payment_status")
async def get_payment_status(payment_id: UUID):
    """Get payment status and details"""

    try:
        logger.info(f"Getting payment status: {payment_id}")

        # Query database for payment
        payment = await postgres_client.fetchrow(
            "SELECT * FROM payments WHERE transaction_id = $1",
            str(payment_id)
        )

        if not payment:
            raise HTTPException(status_code=404, detail="Payment not found")

        # Get settlement details if available
        settlement_details = None
        if payment["status"] in [TransactionStatus.SETTLED, TransactionStatus.COMPLETED]:
            settlement_details = await postgres_client.fetchrow(
                "SELECT * FROM settlement_details WHERE transaction_id = $1",
                str(payment_id)
            )

        # Get ledger proof if available
        ledger_proof = None
        if payment["status"] == TransactionStatus.COMPLETED:
            ledger_proof = await postgres_client.fetchrow(
                "SELECT * FROM ledger_proofs WHERE transaction_id = $1",
                str(payment_id)
            )

        return CTRStatusResponse(
            transaction_id=UUID(payment["transaction_id"]),
            uetr=UUID(payment["uetr"]),
            status=payment["status"],
            current_step=payment.get("current_step"),
            settlement_details=dict(settlement_details) if settlement_details else None,
            ledger_proof=dict(ledger_proof) if ledger_proof else None,
            estimated_completion=payment.get("estimated_completion")
        )

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get payment status: {e}")
        raise PaymentError("Failed to get payment status", ErrorCode.INTERNAL_ERROR)


@router.post("/{payment_id}/cancel")
@trace_operation("payment_cancel")
async def cancel_payment(payment_id: UUID):
    """Cancel a payment transaction"""

    try:
        logger.info(f"Cancelling payment: {payment_id}")

        # Check if payment exists and can be cancelled
        payment = await postgres_client.fetchrow(
            "SELECT status FROM payments WHERE transaction_id = $1",
            str(payment_id)
        )

        if not payment:
            raise HTTPException(status_code=404, detail="Payment not found")

        if payment["status"] in [TransactionStatus.SETTLED, TransactionStatus.COMPLETED]:
            raise PaymentError(
                "Cannot cancel completed payment",
                ErrorCode.PAYMENT_CANCELLED
            )

        # Update status to cancelled
        await postgres_client.execute(
            "UPDATE payments SET status = $1, updated_at = $2 WHERE transaction_id = $3",
            TransactionStatus.CANCELLED,
            datetime.utcnow(),
            str(payment_id)
        )

        # Publish cancellation event
        await nats_client.publish(
            "payment.cancelled",
            {
                "transaction_id": str(payment_id),
                "cancelled_at": datetime.utcnow().isoformat()
            }
        )

        logger.info(f"Payment cancelled: {payment_id}")

        return {"message": "Payment cancelled successfully"}

    except HTTPException:
        raise
    except PaymentError:
        raise
    except Exception as e:
        logger.error(f"Failed to cancel payment: {e}")
        raise PaymentError("Failed to cancel payment", ErrorCode.INTERNAL_ERROR)