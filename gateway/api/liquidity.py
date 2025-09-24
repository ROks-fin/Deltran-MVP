import logging
from datetime import datetime, timedelta
from typing import Optional, List, Dict, Any
import asyncio
import random

from fastapi import APIRouter, Query, HTTPException
from pydantic import BaseModel

from shared.utils.errors import ErrorCode, ExternalServiceError
from shared.utils.tracing import trace_operation
from shared.clients.redis_client import redis_client
from shared.clients.nats_client import nats_client

logger = logging.getLogger(__name__)
router = APIRouter()


class LiquidityQuote(BaseModel):
    quote_id: str
    from_currency: str
    to_currency: str
    amount: float
    mid_rate: float
    applied_rate: float
    spread: float
    source: str
    ttl_seconds: int
    expires_at: datetime
    utility_score: float


class LiquidityQuoteRequest(BaseModel):
    from_currency: str
    to_currency: str
    amount: float
    settlement_method: Optional[str] = "PVP"
    max_sources: Optional[int] = 3


class LiquidityQuoteResponse(BaseModel):
    quotes: List[LiquidityQuote]
    best_quote: Optional[LiquidityQuote] = None
    request_id: str
    generated_at: datetime
    sla_ms: int


# Mock liquidity providers
LIQUIDITY_PROVIDERS = {
    "treasury": {
        "name": "Treasury Desk",
        "currencies": ["USD", "EUR", "GBP", "JPY", "CHF"],
        "base_spread": 0.002,
        "latency_ms": 50,
        "utility_score": 0.9
    },
    "fund": {
        "name": "Investment Fund",
        "currencies": ["USD", "AED", "INR", "SGD", "HKD"],
        "base_spread": 0.003,
        "latency_ms": 80,
        "utility_score": 0.8
    },
    "p2p": {
        "name": "P2P Network",
        "currencies": ["USD", "EUR", "AED", "INR"],
        "base_spread": 0.001,
        "latency_ms": 120,
        "utility_score": 0.7
    },
    "mm": {
        "name": "Market Maker",
        "currencies": ["USD", "EUR", "GBP", "JPY", "AED", "INR"],
        "base_spread": 0.0015,
        "latency_ms": 30,
        "utility_score": 0.95
    }
}

# Mock exchange rates (in production, would fetch from real sources)
MOCK_RATES = {
    ("USD", "EUR"): 0.85,
    ("USD", "GBP"): 0.75,
    ("USD", "JPY"): 110.0,
    ("USD", "AED"): 3.67,
    ("USD", "INR"): 83.0,
    ("AED", "INR"): 22.6,
    ("EUR", "GBP"): 0.88,
    ("EUR", "USD"): 1.18,
    ("GBP", "USD"): 1.33,
    ("JPY", "USD"): 0.009,
    ("AED", "USD"): 0.27,
    ("INR", "USD"): 0.012,
    ("INR", "AED"): 0.044,
}


@router.get("/quotes", response_model=LiquidityQuoteResponse)
@trace_operation("liquidity_quotes", max_sla_ms=150)
async def get_liquidity_quotes(
    from_currency: str = Query(..., description="Source currency (3-letter ISO code)"),
    to_currency: str = Query(..., description="Target currency (3-letter ISO code)"),
    amount: float = Query(..., gt=0, description="Amount to convert"),
    settlement_method: str = Query("PVP", description="Settlement method"),
    max_sources: int = Query(3, ge=1, le=5, description="Maximum number of quote sources")
):
    """Get real-time liquidity quotes with SLA ≤150ms"""

    start_time = datetime.utcnow()
    from shared.utils.uuidv7 import generate_uuidv7
    request_id = str(generate_uuidv7())

    try:
        logger.info(f"Getting liquidity quotes: {from_currency}/{to_currency} amount={amount}")

        # Validate currencies
        if from_currency == to_currency:
            raise HTTPException(status_code=400, detail="From and to currencies cannot be the same")

        # Check cache first
        cache_key = f"liquidity:{from_currency}:{to_currency}:{amount}:{settlement_method}"
        cached_quotes = await redis_client.cache_get(cache_key)

        if cached_quotes:
            logger.info(f"Returning cached liquidity quotes for {request_id}")
            cached_quotes["request_id"] = request_id
            return LiquidityQuoteResponse(**cached_quotes)

        # Get quotes from multiple providers concurrently
        quote_tasks = []
        for provider_id, provider in LIQUIDITY_PROVIDERS.items():
            if (from_currency in provider["currencies"] and
                to_currency in provider["currencies"]):
                quote_tasks.append(
                    _get_provider_quote(provider_id, provider, from_currency, to_currency, amount)
                )

        # Wait for quotes with timeout to meet SLA
        try:
            quotes = await asyncio.wait_for(
                asyncio.gather(*quote_tasks[:max_sources], return_exceptions=True),
                timeout=0.120  # 120ms to leave buffer for processing
            )
        except asyncio.TimeoutError:
            logger.warning("Quote timeout - returning partial results")
            quotes = []

        # Filter successful quotes
        valid_quotes = [q for q in quotes if isinstance(q, LiquidityQuote)]

        if not valid_quotes:
            raise ExternalServiceError(
                "No liquidity providers available",
                service="liquidity_providers"
            )

        # Find best quote (highest utility score)
        best_quote = max(valid_quotes, key=lambda q: q.utility_score)

        # Calculate SLA timing
        sla_ms = int((datetime.utcnow() - start_time).total_seconds() * 1000)

        response = LiquidityQuoteResponse(
            quotes=valid_quotes,
            best_quote=best_quote,
            request_id=request_id,
            generated_at=datetime.utcnow(),
            sla_ms=sla_ms
        )

        # Cache the response for 30 seconds
        await redis_client.cache_set(cache_key, response.dict(), expire=30)

        # Publish quote event for monitoring
        await nats_client.publish(
            "liquidity.quote_generated",
            {
                "request_id": request_id,
                "from_currency": from_currency,
                "to_currency": to_currency,
                "amount": amount,
                "quote_count": len(valid_quotes),
                "sla_ms": sla_ms
            }
        )

        logger.info(f"Generated {len(valid_quotes)} quotes in {sla_ms}ms for {request_id}")

        return response

    except HTTPException:
        raise
    except ExternalServiceError:
        raise
    except Exception as e:
        sla_ms = int((datetime.utcnow() - start_time).total_seconds() * 1000)
        logger.error(f"Failed to get liquidity quotes: {e} (took {sla_ms}ms)")
        raise ExternalServiceError(
            "Failed to get liquidity quotes",
            service="liquidity_service"
        )


@router.get("/quotes/{quote_id}")
@trace_operation("liquidity_quote_details")
async def get_quote_details(quote_id: str):
    """Get details of a specific quote"""

    try:
        # Check if quote exists in cache
        quote_data = await redis_client.get(f"quote:{quote_id}")

        if not quote_data:
            raise HTTPException(status_code=404, detail="Quote not found or expired")

        return quote_data

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to get quote details: {e}")
        raise ExternalServiceError(
            "Failed to get quote details",
            service="liquidity_service"
        )


@router.post("/quotes/{quote_id}/execute")
@trace_operation("liquidity_execute_quote")
async def execute_quote(quote_id: str):
    """Execute a liquidity quote"""

    try:
        # Get quote details
        quote_data = await redis_client.get(f"quote:{quote_id}")

        if not quote_data:
            raise HTTPException(status_code=404, detail="Quote not found or expired")

        # Check if quote is still valid
        quote = LiquidityQuote(**quote_data)
        if datetime.utcnow() > quote.expires_at:
            raise HTTPException(status_code=410, detail="Quote has expired")

        # In production, this would execute the actual trade
        # For now, just simulate execution

        execution_result = {
            "execution_id": str(generate_uuidv7()),
            "quote_id": quote_id,
            "executed_rate": quote.applied_rate,
            "executed_at": datetime.utcnow(),
            "status": "EXECUTED"
        }

        # Remove quote from cache (can only be executed once)
        await redis_client.delete(f"quote:{quote_id}")

        # Publish execution event
        await nats_client.publish(
            "liquidity.quote_executed",
            execution_result
        )

        logger.info(f"Quote executed: {quote_id}")

        return execution_result

    except HTTPException:
        raise
    except Exception as e:
        logger.error(f"Failed to execute quote: {e}")
        raise ExternalServiceError(
            "Failed to execute quote",
            service="liquidity_service"
        )


async def _get_provider_quote(
    provider_id: str,
    provider: Dict[str, Any],
    from_currency: str,
    to_currency: str,
    amount: float
) -> LiquidityQuote:
    """Get quote from a specific liquidity provider"""

    try:
        # Simulate provider latency
        await asyncio.sleep(provider["latency_ms"] / 1000.0)

        # Get base rate
        rate_key = (from_currency, to_currency)
        reverse_rate_key = (to_currency, from_currency)

        if rate_key in MOCK_RATES:
            mid_rate = MOCK_RATES[rate_key]
        elif reverse_rate_key in MOCK_RATES:
            mid_rate = 1.0 / MOCK_RATES[reverse_rate_key]
        else:
            # Generate synthetic rate
            mid_rate = random.uniform(0.5, 2.0)

        # Apply spread and randomness
        base_spread = provider["base_spread"]
        spread = base_spread * (1 + random.uniform(-0.2, 0.2))  # ±20% variation
        applied_rate = mid_rate * (1 - spread)

        # Generate quote
        from shared.utils.uuidv7 import generate_uuidv7
        quote_id = str(generate_uuidv7())
        ttl_seconds = 30  # 30 second TTL
        expires_at = datetime.utcnow() + timedelta(seconds=ttl_seconds)

        quote = LiquidityQuote(
            quote_id=quote_id,
            from_currency=from_currency,
            to_currency=to_currency,
            amount=amount,
            mid_rate=mid_rate,
            applied_rate=applied_rate,
            spread=spread,
            source=provider["name"],
            ttl_seconds=ttl_seconds,
            expires_at=expires_at,
            utility_score=provider["utility_score"] * random.uniform(0.9, 1.1)
        )

        # Cache quote for potential execution
        await redis_client.set(f"quote:{quote_id}", quote.dict(), expire=ttl_seconds)

        return quote

    except Exception as e:
        logger.error(f"Provider {provider_id} failed: {e}")
        raise e