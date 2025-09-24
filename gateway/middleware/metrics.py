import time
import logging
from typing import Callable

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import Response
from prometheus_client import Counter, Histogram, Gauge, generate_latest, CONTENT_TYPE_LATEST

logger = logging.getLogger(__name__)

# Prometheus metrics
REQUEST_COUNT = Counter(
    'http_requests_total',
    'Total HTTP requests',
    ['method', 'endpoint', 'status_code', 'service']
)

REQUEST_DURATION = Histogram(
    'http_request_duration_seconds',
    'HTTP request duration in seconds',
    ['method', 'endpoint', 'service'],
    buckets=[0.01, 0.025, 0.05, 0.1, 0.25, 0.5, 1.0, 2.5, 5.0, 10.0]
)

ACTIVE_REQUESTS = Gauge(
    'http_active_requests',
    'Number of active HTTP requests',
    ['service']
)

# Payment metrics
PAYMENT_COUNT = Counter(
    'payments_total',
    'Total payment transactions',
    ['status', 'currency', 'settlement_method']
)

PAYMENT_AMOUNT = Histogram(
    'payment_amount_usd',
    'Payment amounts in USD',
    buckets=[10, 50, 100, 500, 1000, 5000, 10000, 50000, 100000, 1000000]
)

# Settlement metrics
SETTLEMENT_BATCH_SIZE = Histogram(
    'settlement_batch_size',
    'Settlement batch sizes',
    ['window'],
    buckets=[10, 50, 100, 500, 1000, 5000, 10000]
)

# Risk metrics
RISK_SCORE = Histogram(
    'risk_score',
    'Transaction risk scores',
    buckets=[0, 10, 20, 30, 40, 50, 60, 70, 80, 90, 100]
)

# Compliance metrics
COMPLIANCE_CHECK_COUNT = Counter(
    'compliance_checks_total',
    'Total compliance checks',
    ['check_type', 'status']
)


class MetricsMiddleware(BaseHTTPMiddleware):
    """Middleware to collect HTTP metrics"""

    def __init__(self, app, service_name: str = "gateway"):
        super().__init__(app)
        self.service_name = service_name

    async def dispatch(self, request: Request, call_next: Callable) -> Response:
        """Collect metrics for HTTP requests"""

        # Skip metrics collection for metrics endpoint
        if request.url.path == "/metrics":
            return await call_next(request)

        method = request.method
        path = request.url.path

        # Normalize path for metrics (remove IDs, etc.)
        endpoint = self._normalize_endpoint(path)

        # Start timing
        start_time = time.time()

        # Increment active requests
        ACTIVE_REQUESTS.labels(service=self.service_name).inc()

        try:
            # Process request
            response = await call_next(request)

            # Record metrics
            duration = time.time() - start_time
            status_code = response.status_code

            REQUEST_COUNT.labels(
                method=method,
                endpoint=endpoint,
                status_code=status_code,
                service=self.service_name
            ).inc()

            REQUEST_DURATION.labels(
                method=method,
                endpoint=endpoint,
                service=self.service_name
            ).observe(duration)

            return response

        except Exception as e:
            # Record error metrics
            duration = time.time() - start_time

            REQUEST_COUNT.labels(
                method=method,
                endpoint=endpoint,
                status_code=500,
                service=self.service_name
            ).inc()

            REQUEST_DURATION.labels(
                method=method,
                endpoint=endpoint,
                service=self.service_name
            ).observe(duration)

            raise

        finally:
            # Decrement active requests
            ACTIVE_REQUESTS.labels(service=self.service_name).dec()

    def _normalize_endpoint(self, path: str) -> str:
        """Normalize endpoint path for metrics"""
        # Replace UUIDs and other IDs with placeholders
        import re

        # Replace UUIDs
        path = re.sub(
            r'[0-9a-f]{8}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{4}-[0-9a-f]{12}',
            '{id}',
            path,
            flags=re.IGNORECASE
        )

        # Replace numeric IDs
        path = re.sub(r'/\d+', '/{id}', path)

        # Normalize common patterns
        normalizations = {
            '/payments/{id}/status': '/payments/{id}/status',
            '/settlement/close-batch': '/settlement/close-batch',
            '/liquidity/quotes': '/liquidity/quotes',
            '/risk/mode': '/risk/mode',
            '/reports/proof-of-reserves': '/reports/proof-of-reserves',
            '/reports/proof-of-settlement': '/reports/proof-of-settlement',
        }

        return normalizations.get(path, path)


# Metric collection functions
def record_payment_metric(amount_usd: float, currency: str, status: str, settlement_method: str):
    """Record payment metrics"""
    PAYMENT_COUNT.labels(
        status=status,
        currency=currency,
        settlement_method=settlement_method
    ).inc()

    PAYMENT_AMOUNT.observe(amount_usd)


def record_settlement_metric(batch_size: int, window: str):
    """Record settlement metrics"""
    SETTLEMENT_BATCH_SIZE.labels(window=window).observe(batch_size)


def record_risk_metric(risk_score: float):
    """Record risk metrics"""
    RISK_SCORE.observe(risk_score)


def record_compliance_metric(check_type: str, status: str):
    """Record compliance metrics"""
    COMPLIANCE_CHECK_COUNT.labels(
        check_type=check_type,
        status=status
    ).inc()


# Metrics endpoint handler
async def metrics_handler():
    """Handler for /metrics endpoint"""
    return Response(
        generate_latest(),
        media_type=CONTENT_TYPE_LATEST
    )