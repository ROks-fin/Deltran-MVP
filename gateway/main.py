import os
import ssl
import asyncio
from contextlib import asynccontextmanager

from fastapi import FastAPI, Request, HTTPException, Depends
from fastapi.middleware.cors import CORSMiddleware
from fastapi.middleware.trustedhost import TrustedHostMiddleware
from fastapi.responses import JSONResponse
from fastapi.security import HTTPBearer
import uvicorn

from shared.utils.logging import setup_logging, get_logger
from shared.utils.tracing import init_tracing, instrument_fastapi
from shared.utils.errors import DeltranError, get_http_status
from shared.clients.nats_client import nats_client
from shared.clients.postgres_client import postgres_client
from shared.clients.redis_client import redis_client

from api.payments import router as payments_router
from api.settlement import router as settlement_router
from api.liquidity import router as liquidity_router
from api.risk import router as risk_router
from api.reports import router as reports_router
from middleware.mtls import mTLSMiddleware
from middleware.idempotency import IdempotencyMiddleware
from middleware.metrics import MetricsMiddleware


# Environment configuration
DATABASE_URL = os.getenv("DATABASE_URL", "postgresql://deltran:deltran123@localhost:5432/deltran")
REDIS_URL = os.getenv("REDIS_URL", "redis://localhost:6379")
NATS_URL = os.getenv("NATS_URL", "nats://localhost:4222")
JAEGER_ENDPOINT = os.getenv("JAEGER_ENDPOINT")
LOG_LEVEL = os.getenv("LOG_LEVEL", "INFO")

# mTLS configuration
MTLS_ENABLED = os.getenv("MTLS_ENABLED", "false").lower() == "true"
MTLS_CA_CERT = os.getenv("MTLS_CA_CERT_PATH", "/app/certs/ca.crt")
MTLS_CERT = os.getenv("MTLS_CERT_PATH", "/app/certs/server.crt")
MTLS_KEY = os.getenv("MTLS_KEY_PATH", "/app/certs/server.key")


@asynccontextmanager
async def lifespan(app: FastAPI):
    """Application lifespan manager"""
    logger = get_logger("gateway")

    # Startup
    logger.info("Starting gateway service...")

    try:
        # Initialize clients
        postgres_client.database_url = DATABASE_URL
        redis_client.url = REDIS_URL
        nats_client.url = NATS_URL

        # Connect to services
        await postgres_client.connect()
        await redis_client.connect()
        await nats_client.connect()

        logger.info("All clients connected successfully")

    except Exception as e:
        logger.error(f"Failed to initialize clients: {e}")
        raise

    yield

    # Shutdown
    logger.info("Shutting down gateway service...")
    await postgres_client.disconnect()
    await redis_client.disconnect()
    await nats_client.disconnect()


# Initialize FastAPI app
app = FastAPI(
    title="DelTran Rail Gateway",
    description="High-performance gateway for cross-border rail payments",
    version="1.0.0",
    lifespan=lifespan
)

# Setup logging and tracing
setup_logging("gateway", LOG_LEVEL)
init_tracing("gateway", JAEGER_ENDPOINT)
instrument_fastapi(app, "gateway")

# Security
security = HTTPBearer(auto_error=False)


# Middleware
app.add_middleware(
    CORSMiddleware,
    allow_origins=["*"],  # Configure appropriately for production
    allow_credentials=True,
    allow_methods=["*"],
    allow_headers=["*"],
)

app.add_middleware(
    TrustedHostMiddleware,
    allowed_hosts=["*"]  # Configure appropriately for production
)

# Custom middleware
app.add_middleware(MetricsMiddleware)
app.add_middleware(IdempotencyMiddleware)

if MTLS_ENABLED:
    app.add_middleware(
        mTLSMiddleware,
        ca_cert_path=MTLS_CA_CERT,
        verify_client=True
    )


# Exception handlers
@app.exception_handler(DeltranError)
async def deltran_error_handler(request: Request, exc: DeltranError):
    """Handle DeltranError exceptions"""
    logger = get_logger("gateway")
    logger.error(f"DeltranError: {exc.code} - {exc.message}", extra={
        "error_code": exc.code,
        "transaction_id": exc.transaction_id,
        "trace_id": exc.trace_id,
        "details": exc.details
    })

    return JSONResponse(
        status_code=get_http_status(exc.code),
        content=exc.to_dict()
    )


@app.exception_handler(HTTPException)
async def http_exception_handler(request: Request, exc: HTTPException):
    """Handle HTTP exceptions"""
    return JSONResponse(
        status_code=exc.status_code,
        content={
            "error": {
                "code": "HTTP_ERROR",
                "message": exc.detail,
            }
        }
    )


@app.exception_handler(Exception)
async def general_exception_handler(request: Request, exc: Exception):
    """Handle general exceptions"""
    logger = get_logger("gateway")
    logger.exception("Unhandled exception", exc_info=exc)

    return JSONResponse(
        status_code=500,
        content={
            "error": {
                "code": "INTERNAL_ERROR",
                "message": "An internal error occurred",
            }
        }
    )


# Health check endpoints
@app.get("/health")
async def health_check():
    """Health check endpoint"""
    checks = {
        "postgres": await postgres_client.health_check(),
        "redis": await redis_client.health_check(),
        "nats": nats_client.health_check(),
    }

    all_healthy = all(
        check.get("status") == "healthy" if isinstance(check, dict) else check
        for check in checks.values()
    )

    status = "healthy" if all_healthy else "unhealthy"
    status_code = 200 if all_healthy else 503

    return JSONResponse(
        status_code=status_code,
        content={
            "status": status,
            "timestamp": asyncio.get_event_loop().time(),
            "checks": checks
        }
    )


@app.get("/ready")
async def readiness_check():
    """Readiness check endpoint"""
    return {"status": "ready"}


@app.get("/")
async def root():
    """Root endpoint"""
    return {
        "service": "DelTran Rail Gateway",
        "version": "1.0.0",
        "status": "running"
    }


# API Routes
app.include_router(payments_router, prefix="/payments", tags=["payments"])
app.include_router(settlement_router, prefix="/settlement", tags=["settlement"])
app.include_router(liquidity_router, prefix="/liquidity", tags=["liquidity"])
app.include_router(risk_router, prefix="/risk", tags=["risk"])
app.include_router(reports_router, prefix="/reports", tags=["reports"])


if __name__ == "__main__":
    # SSL context for mTLS if enabled
    ssl_context = None
    if MTLS_ENABLED and os.path.exists(MTLS_CERT) and os.path.exists(MTLS_KEY):
        ssl_context = ssl.create_default_context(ssl.Purpose.CLIENT_AUTH)
        ssl_context.load_cert_chain(MTLS_CERT, MTLS_KEY)
        if os.path.exists(MTLS_CA_CERT):
            ssl_context.load_verify_locations(MTLS_CA_CERT)
            ssl_context.verify_mode = ssl.CERT_REQUIRED

    uvicorn.run(
        "main:app",
        host="0.0.0.0",
        port=8000,
        ssl_keyfile=MTLS_KEY if MTLS_ENABLED else None,
        ssl_certfile=MTLS_CERT if MTLS_ENABLED else None,
        ssl_ca_certs=MTLS_CA_CERT if MTLS_ENABLED else None,
        reload=False,
        log_config=None,  # Use our custom logging
    )