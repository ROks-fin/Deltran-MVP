"""
DelTran Analytics Collector Service
Collects and stores transaction analytics using FastAPI and AsyncPG
"""

from fastapi import FastAPI, BackgroundTasks, HTTPException
from pydantic import BaseModel, Field
from typing import Optional, Dict, Any, List
import asyncpg
import asyncio
from datetime import datetime
import json
import os
from contextlib import asynccontextmanager

# Database configuration
DATABASE_URL = os.getenv(
    "DATABASE_URL",
    "postgresql://postgres:password@localhost:5432/deltran_analytics"
)

# Global database pool
db_pool = None

# Pydantic models
class TransactionEvent(BaseModel):
    transaction_id: str
    event_type: str  # 'started', 'gateway', 'clearing', 'settlement', 'completed', 'failed'
    timestamp: datetime
    service: str
    data: Dict[str, Any] = Field(default_factory=dict)

class PerformanceMetric(BaseModel):
    service: str
    endpoint: str
    latency_ms: int
    status_code: int
    timestamp: datetime = Field(default_factory=datetime.now)

class TransactionCreate(BaseModel):
    transaction_id: str
    sender_id: str
    receiver_id: str
    amount: float
    currency: str = "USD"
    test_run_id: Optional[str] = None
    test_scenario: Optional[str] = None
    load_level: Optional[str] = None

class TransactionMetrics(BaseModel):
    total_transactions: int
    completed: int
    failed: int
    pending: int
    avg_latency: Optional[int]
    median_latency: Optional[int]
    p95_latency: Optional[int]
    p99_latency: Optional[int]
    max_latency: Optional[int]
    total_volume: Optional[float]
    unique_senders: int
    unique_receivers: int

@asynccontextmanager
async def lifespan(app: FastAPI):
    """Lifecycle manager for database connection pool"""
    global db_pool

    # Startup
    print(f"🔌 Connecting to database: {DATABASE_URL}")
    try:
        db_pool = await asyncpg.create_pool(
            DATABASE_URL,
            min_size=5,
            max_size=20,
            command_timeout=60
        )
        print("✅ Database connection pool created")
    except Exception as e:
        print(f"❌ Failed to connect to database: {e}")
        raise

    yield

    # Shutdown
    if db_pool:
        await db_pool.close()
        print("🔌 Database connection pool closed")

# Initialize FastAPI app with lifespan
app = FastAPI(
    title="DelTran Analytics Collector",
    description="Collects and analyzes transaction performance data",
    version="1.0.0",
    lifespan=lifespan
)

# Health check endpoint
@app.get("/health")
async def health_check():
    """Health check endpoint"""
    try:
        if db_pool:
            async with db_pool.acquire() as conn:
                await conn.fetchval('SELECT 1')
            return {
                "status": "healthy",
                "service": "analytics-collector",
                "database": "connected"
            }
        else:
            return {
                "status": "unhealthy",
                "service": "analytics-collector",
                "database": "not_connected"
            }
    except Exception as e:
        raise HTTPException(status_code=503, detail=f"Service unhealthy: {str(e)}")

@app.post("/events/transaction", status_code=201)
async def record_transaction_event(
    event: TransactionEvent,
    background_tasks: BackgroundTasks
):
    """Record transaction lifecycle events"""

    try:
        async with db_pool.acquire() as conn:
            # Check if transaction exists
            existing = await conn.fetchrow(
                "SELECT id FROM transaction_analytics WHERE transaction_id = $1",
                event.transaction_id
            )

            if existing:
                # Update existing transaction
                update_fields = {}

                if event.event_type == "gateway":
                    update_fields["gateway_received_at"] = event.timestamp
                elif event.event_type == "clearing_start":
                    update_fields["clearing_started_at"] = event.timestamp
                elif event.event_type == "clearing_complete":
                    update_fields["clearing_completed_at"] = event.timestamp
                elif event.event_type == "settlement_start":
                    update_fields["settlement_started_at"] = event.timestamp
                elif event.event_type == "settlement_complete":
                    update_fields["settlement_completed_at"] = event.timestamp
                elif event.event_type == "completed":
                    update_fields["status"] = "completed"
                elif event.event_type == "failed":
                    update_fields["status"] = "failed"
                    update_fields["error_message"] = event.data.get("error", "Unknown error")

                # Build dynamic UPDATE query
                if update_fields:
                    set_clause = ", ".join([f"{k} = ${i+2}" for i, k in enumerate(update_fields.keys())])
                    values = [event.transaction_id] + list(update_fields.values())

                    query = f"""
                        UPDATE transaction_analytics
                        SET {set_clause}, raw_response = raw_response || $1::jsonb
                        WHERE transaction_id = ${len(values) + 1}
                    """

                    await conn.execute(
                        query,
                        json.dumps({"event": event.event_type, "data": event.data}),
                        *values,
                        event.transaction_id
                    )
            else:
                # Create new transaction record
                await conn.execute("""
                    INSERT INTO transaction_analytics
                    (transaction_id, timestamp, status, raw_request)
                    VALUES ($1, $2, $3, $4)
                """,
                    event.transaction_id,
                    event.timestamp,
                    event.event_type,
                    json.dumps(event.data)
                )

        return {
            "status": "recorded",
            "transaction_id": event.transaction_id,
            "event_type": event.event_type
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to record event: {str(e)}")

@app.post("/transactions", status_code=201)
async def create_transaction(transaction: TransactionCreate):
    """Create a new transaction record"""

    try:
        async with db_pool.acquire() as conn:
            await conn.execute("""
                INSERT INTO transaction_analytics
                (transaction_id, sender_id, receiver_id, amount, currency,
                 status, test_run_id, test_scenario, load_level, gateway_received_at)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, NOW())
                ON CONFLICT (transaction_id) DO NOTHING
            """,
                transaction.transaction_id,
                transaction.sender_id,
                transaction.receiver_id,
                transaction.amount,
                transaction.currency,
                "pending",
                transaction.test_run_id,
                transaction.test_scenario,
                transaction.load_level
            )

        return {
            "status": "created",
            "transaction_id": transaction.transaction_id
        }

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to create transaction: {str(e)}")

@app.get("/metrics/dashboard", response_model=TransactionMetrics)
async def get_dashboard_metrics():
    """Get real-time metrics for dashboard (last 5 minutes)"""

    try:
        async with db_pool.acquire() as conn:
            row = await conn.fetchrow("SELECT * FROM dashboard_metrics")

            if not row:
                return TransactionMetrics(
                    total_transactions=0,
                    completed=0,
                    failed=0,
                    pending=0,
                    avg_latency=None,
                    median_latency=None,
                    p95_latency=None,
                    p99_latency=None,
                    max_latency=None,
                    total_volume=None,
                    unique_senders=0,
                    unique_receivers=0
                )

            return TransactionMetrics(**dict(row))

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch metrics: {str(e)}")

@app.get("/metrics/performance/{test_run_id}")
async def get_test_performance(test_run_id: str):
    """Get performance metrics for a specific test run"""

    try:
        async with db_pool.acquire() as conn:
            row = await conn.fetchrow("""
                SELECT * FROM test_run_summary
                WHERE test_run_id = $1
            """, test_run_id)

            if not row:
                raise HTTPException(status_code=404, detail="Test run not found")

            return dict(row)

    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch test performance: {str(e)}")

@app.get("/metrics/test-runs")
async def list_test_runs(limit: int = 10):
    """List recent test runs"""

    try:
        async with db_pool.acquire() as conn:
            rows = await conn.fetch("""
                SELECT * FROM test_run_summary
                ORDER BY test_start DESC
                LIMIT $1
            """, limit)

            return [dict(row) for row in rows]

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to list test runs: {str(e)}")

@app.get("/transactions/{transaction_id}")
async def get_transaction(transaction_id: str):
    """Get detailed transaction information"""

    try:
        async with db_pool.acquire() as conn:
            row = await conn.fetchrow("""
                SELECT * FROM transaction_analytics
                WHERE transaction_id = $1
            """, transaction_id)

            if not row:
                raise HTTPException(status_code=404, detail="Transaction not found")

            return dict(row)

    except HTTPException:
        raise
    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to fetch transaction: {str(e)}")

@app.get("/transactions")
async def list_transactions(
    limit: int = 100,
    offset: int = 0,
    status: Optional[str] = None,
    test_run_id: Optional[str] = None
):
    """List transactions with optional filters"""

    try:
        async with db_pool.acquire() as conn:
            query = "SELECT * FROM transaction_analytics WHERE 1=1"
            params = []

            if status:
                params.append(status)
                query += f" AND status = ${len(params)}"

            if test_run_id:
                params.append(test_run_id)
                query += f" AND test_run_id = ${len(params)}"

            query += f" ORDER BY timestamp DESC LIMIT ${len(params) + 1} OFFSET ${len(params) + 2}"
            params.extend([limit, offset])

            rows = await conn.fetch(query, *params)

            return [dict(row) for row in rows]

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to list transactions: {str(e)}")

@app.delete("/transactions/cleanup")
async def cleanup_old_transactions(days: int = 30):
    """Cleanup old transaction data"""

    try:
        async with db_pool.acquire() as conn:
            result = await conn.execute("""
                DELETE FROM transaction_analytics
                WHERE timestamp < NOW() - INTERVAL '$1 days'
            """, days)

            return {
                "status": "cleaned",
                "message": f"Deleted transactions older than {days} days",
                "result": result
            }

    except Exception as e:
        raise HTTPException(status_code=500, detail=f"Failed to cleanup: {str(e)}")

# Run with: uvicorn main:app --host 0.0.0.0 --port 8093 --reload
if __name__ == "__main__":
    import uvicorn

    print("🚀 Starting DelTran Analytics Collector...")
    print(f"📊 Database: {DATABASE_URL}")
    print(f"🌐 Server: http://localhost:8093")
    print(f"📖 Docs: http://localhost:8093/docs")

    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8093,
        reload=True,
        log_level="info"
    )
