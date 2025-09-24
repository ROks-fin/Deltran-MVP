import json
import logging
from typing import Callable, Optional

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import Response, JSONResponse

from shared.clients.redis_client import redis_client
from shared.utils.uuidv7 import generate_uuidv7

logger = logging.getLogger(__name__)


class IdempotencyMiddleware(BaseHTTPMiddleware):
    """Middleware to handle idempotent requests using Redis"""

    def __init__(self, app, ttl_seconds: int = 3600):
        super().__init__(app)
        self.ttl_seconds = ttl_seconds

    async def dispatch(self, request: Request, call_next: Callable) -> Response:
        """Process request with idempotency checking"""

        # Only apply to POST, PUT, PATCH requests
        if request.method not in ["POST", "PUT", "PATCH"]:
            return await call_next(request)

        # Get idempotency key from header
        idempotency_key = request.headers.get("Idempotency-Key")

        if not idempotency_key:
            # For POST requests, require idempotency key
            if request.method == "POST":
                return JSONResponse(
                    status_code=400,
                    content={
                        "error": {
                            "code": "MISSING_IDEMPOTENCY_KEY",
                            "message": "Idempotency-Key header is required for POST requests"
                        }
                    }
                )
            # For PUT/PATCH, continue without idempotency
            return await call_next(request)

        # Validate idempotency key format (should be UUID)
        try:
            import uuid
            uuid.UUID(idempotency_key)
        except ValueError:
            return JSONResponse(
                status_code=400,
                content={
                    "error": {
                        "code": "INVALID_IDEMPOTENCY_KEY",
                        "message": "Idempotency-Key must be a valid UUID"
                    }
                }
            )

        # Check if we've seen this request before
        cache_key = f"idempotency:{idempotency_key}"

        try:
            cached_response = await redis_client.get(cache_key)

            if cached_response:
                logger.info(f"Returning cached response for idempotency key: {idempotency_key}")

                # Return cached response
                response_data = json.loads(cached_response) if isinstance(cached_response, str) else cached_response

                return JSONResponse(
                    status_code=response_data.get("status_code", 200),
                    content=response_data.get("body"),
                    headers=response_data.get("headers", {})
                )

        except Exception as e:
            logger.error(f"Error checking idempotency cache: {e}")
            # Continue with request if cache check fails

        # Process the request
        response = await call_next(request)

        # Cache successful responses (2xx status codes)
        if 200 <= response.status_code < 300:
            try:
                # Read response body
                body = b""
                async for chunk in response.body_iterator:
                    body += chunk

                # Parse response body
                try:
                    response_body = json.loads(body.decode())
                except json.JSONDecodeError:
                    response_body = body.decode()

                # Prepare cache data
                cache_data = {
                    "status_code": response.status_code,
                    "headers": dict(response.headers),
                    "body": response_body,
                    "cached_at": generate_uuidv7().hex[:8] + "-" +
                                 generate_uuidv7().hex[8:12] + "-" +
                                 generate_uuidv7().hex[12:16] + "-" +
                                 generate_uuidv7().hex[16:20] + "-" +
                                 generate_uuidv7().hex[20:32]
                }

                # Store in cache
                await redis_client.set(cache_key, cache_data, expire=self.ttl_seconds)

                logger.info(f"Cached response for idempotency key: {idempotency_key}")

                # Return response with same body
                return JSONResponse(
                    status_code=response.status_code,
                    content=response_body,
                    headers=dict(response.headers)
                )

            except Exception as e:
                logger.error(f"Error caching idempotent response: {e}")

        return response


def generate_idempotency_key() -> str:
    """Generate a new idempotency key"""
    return str(generate_uuidv7())


def validate_idempotency_key(key: Optional[str]) -> bool:
    """Validate idempotency key format"""
    if not key:
        return False

    try:
        import uuid
        uuid.UUID(key)
        return True
    except ValueError:
        return False