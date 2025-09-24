import ssl
import logging
from typing import Callable

from starlette.middleware.base import BaseHTTPMiddleware
from starlette.requests import Request
from starlette.responses import Response, JSONResponse
from cryptography import x509
from cryptography.hazmat.backends import default_backend

logger = logging.getLogger(__name__)


class mTLSMiddleware(BaseHTTPMiddleware):
    """Middleware to handle mutual TLS authentication"""

    def __init__(self, app, ca_cert_path: str, verify_client: bool = True):
        super().__init__(app)
        self.ca_cert_path = ca_cert_path
        self.verify_client = verify_client
        self.ca_cert = self._load_ca_cert()

    def _load_ca_cert(self):
        """Load CA certificate"""
        try:
            with open(self.ca_cert_path, 'rb') as f:
                return x509.load_pem_x509_certificate(f.read(), default_backend())
        except Exception as e:
            logger.warning(f"Could not load CA certificate: {e}")
            return None

    async def dispatch(self, request: Request, call_next: Callable) -> Response:
        """Process request with mTLS verification"""

        # Skip mTLS for health checks and public endpoints
        if request.url.path in ["/health", "/ready", "/", "/docs", "/openapi.json"]:
            return await call_next(request)

        if not self.verify_client or not self.ca_cert:
            return await call_next(request)

        # Extract client certificate from request
        client_cert = self._extract_client_certificate(request)

        if not client_cert:
            logger.warning("No client certificate provided")
            return JSONResponse(
                status_code=401,
                content={
                    "error": {
                        "code": "MISSING_CLIENT_CERTIFICATE",
                        "message": "Client certificate required for this endpoint"
                    }
                }
            )

        # Verify client certificate
        if not self._verify_client_certificate(client_cert):
            logger.warning("Invalid client certificate")
            return JSONResponse(
                status_code=403,
                content={
                    "error": {
                        "code": "INVALID_CLIENT_CERTIFICATE",
                        "message": "Client certificate verification failed"
                    }
                }
            )

        # Add certificate info to request state
        request.state.client_cert = client_cert
        request.state.client_subject = client_cert.subject.rfc4514_string()

        return await call_next(request)

    def _extract_client_certificate(self, request: Request):
        """Extract client certificate from request"""
        try:
            # Try to get certificate from TLS connection
            if hasattr(request, 'scope') and 'tls' in request.scope:
                tls_info = request.scope['tls']
                if 'client_cert_der' in tls_info:
                    cert_der = tls_info['client_cert_der']
                    return x509.load_der_x509_certificate(cert_der, default_backend())

            # Try to get certificate from headers (for proxy scenarios)
            cert_header = request.headers.get('X-Client-Cert')
            if cert_header:
                # Assuming PEM format in header
                cert_pem = cert_header.replace(' ', '\n')
                cert_pem = f"-----BEGIN CERTIFICATE-----\n{cert_pem}\n-----END CERTIFICATE-----"
                return x509.load_pem_x509_certificate(cert_pem.encode(), default_backend())

        except Exception as e:
            logger.error(f"Error extracting client certificate: {e}")

        return None

    def _verify_client_certificate(self, client_cert) -> bool:
        """Verify client certificate against CA"""
        try:
            # Check if certificate is signed by our CA
            ca_public_key = self.ca_cert.public_key()
            client_public_key = client_cert.public_key()

            # Verify signature (simplified - production should use proper chain validation)
            try:
                ca_public_key.verify(
                    client_cert.signature,
                    client_cert.tbs_certificate_bytes,
                    client_cert.signature_algorithm_oid._name
                )
            except Exception:
                logger.warning("Certificate signature verification failed")
                return False

            # Check certificate validity period
            import datetime
            now = datetime.datetime.utcnow()
            if now < client_cert.not_valid_before or now > client_cert.not_valid_after:
                logger.warning("Certificate is expired or not yet valid")
                return False

            # Additional checks could be added here:
            # - Certificate revocation list (CRL) check
            # - Online Certificate Status Protocol (OCSP) check
            # - Subject/CN validation

            logger.info(f"Client certificate verified for subject: {client_cert.subject.rfc4514_string()}")
            return True

        except Exception as e:
            logger.error(f"Certificate verification failed: {e}")
            return False


def get_client_certificate_info(request: Request) -> dict:
    """Extract client certificate information from request"""
    if not hasattr(request.state, 'client_cert'):
        return {}

    cert = request.state.client_cert
    return {
        "subject": cert.subject.rfc4514_string(),
        "issuer": cert.issuer.rfc4514_string(),
        "serial_number": str(cert.serial_number),
        "not_valid_before": cert.not_valid_before.isoformat(),
        "not_valid_after": cert.not_valid_after.isoformat(),
        "signature_algorithm": cert.signature_algorithm_oid._name,
    }