from enum import Enum
from typing import Optional, Dict, Any


class ErrorCode(str, Enum):
    # Generic errors
    INTERNAL_ERROR = "INTERNAL_ERROR"
    VALIDATION_ERROR = "VALIDATION_ERROR"
    NOT_FOUND = "NOT_FOUND"
    UNAUTHORIZED = "UNAUTHORIZED"
    FORBIDDEN = "FORBIDDEN"
    CONFLICT = "CONFLICT"
    RATE_LIMITED = "RATE_LIMITED"

    # Payment errors
    INSUFFICIENT_FUNDS = "INSUFFICIENT_FUNDS"
    INVALID_ACCOUNT = "INVALID_ACCOUNT"
    INVALID_AMOUNT = "INVALID_AMOUNT"
    PAYMENT_EXPIRED = "PAYMENT_EXPIRED"
    PAYMENT_CANCELLED = "PAYMENT_CANCELLED"
    DUPLICATE_PAYMENT = "DUPLICATE_PAYMENT"

    # Settlement errors
    SETTLEMENT_FAILED = "SETTLEMENT_FAILED"
    BATCH_CLOSED = "BATCH_CLOSED"
    NETTING_ERROR = "NETTING_ERROR"
    LIQUIDITY_UNAVAILABLE = "LIQUIDITY_UNAVAILABLE"

    # Risk errors
    RISK_THRESHOLD_EXCEEDED = "RISK_THRESHOLD_EXCEEDED"
    RISK_ASSESSMENT_FAILED = "RISK_ASSESSMENT_FAILED"
    HIGH_RISK_TRANSACTION = "HIGH_RISK_TRANSACTION"

    # Compliance errors
    SANCTIONS_VIOLATION = "SANCTIONS_VIOLATION"
    PEP_VIOLATION = "PEP_VIOLATION"
    TRAVEL_RULE_VIOLATION = "TRAVEL_RULE_VIOLATION"
    INCOMPLETE_KYC = "INCOMPLETE_KYC"
    COMPLIANCE_CHECK_FAILED = "COMPLIANCE_CHECK_FAILED"

    # Ledger errors
    BLOCK_VALIDATION_FAILED = "BLOCK_VALIDATION_FAILED"
    CONSENSUS_FAILED = "CONSENSUS_FAILED"
    INVALID_SIGNATURE = "INVALID_SIGNATURE"
    DOUBLE_SPEND = "DOUBLE_SPEND"

    # External service errors
    EXTERNAL_SERVICE_ERROR = "EXTERNAL_SERVICE_ERROR"
    TIMEOUT_ERROR = "TIMEOUT_ERROR"
    NETWORK_ERROR = "NETWORK_ERROR"


class DeltranError(Exception):
    """Base exception class for Deltran errors"""

    def __init__(
        self,
        message: str,
        code: ErrorCode = ErrorCode.INTERNAL_ERROR,
        details: Optional[Dict[str, Any]] = None,
        transaction_id: Optional[str] = None,
        trace_id: Optional[str] = None
    ):
        self.message = message
        self.code = code
        self.details = details or {}
        self.transaction_id = transaction_id
        self.trace_id = trace_id
        super().__init__(message)

    def to_dict(self) -> Dict[str, Any]:
        """Convert error to dictionary"""
        error_dict = {
            "error": {
                "code": self.code,
                "message": self.message,
            }
        }

        if self.details:
            error_dict["error"]["details"] = self.details

        if self.transaction_id:
            error_dict["transaction_id"] = self.transaction_id

        if self.trace_id:
            error_dict["trace_id"] = self.trace_id

        return error_dict


class ValidationError(DeltranError):
    """Validation error"""

    def __init__(self, message: str, field: Optional[str] = None, **kwargs):
        details = kwargs.get("details", {})
        if field:
            details["field"] = field
        kwargs["details"] = details
        super().__init__(message, ErrorCode.VALIDATION_ERROR, **kwargs)


class PaymentError(DeltranError):
    """Payment-related error"""

    def __init__(self, message: str, code: ErrorCode = ErrorCode.INTERNAL_ERROR, **kwargs):
        super().__init__(message, code, **kwargs)


class SettlementError(DeltranError):
    """Settlement-related error"""

    def __init__(self, message: str, code: ErrorCode = ErrorCode.SETTLEMENT_FAILED, **kwargs):
        super().__init__(message, code, **kwargs)


class RiskError(DeltranError):
    """Risk-related error"""

    def __init__(self, message: str, code: ErrorCode = ErrorCode.RISK_ASSESSMENT_FAILED, **kwargs):
        super().__init__(message, code, **kwargs)


class ComplianceError(DeltranError):
    """Compliance-related error"""

    def __init__(self, message: str, code: ErrorCode = ErrorCode.COMPLIANCE_CHECK_FAILED, **kwargs):
        super().__init__(message, code, **kwargs)


class LedgerError(DeltranError):
    """Ledger-related error"""

    def __init__(self, message: str, code: ErrorCode = ErrorCode.BLOCK_VALIDATION_FAILED, **kwargs):
        super().__init__(message, code, **kwargs)


class ExternalServiceError(DeltranError):
    """External service error"""

    def __init__(self, message: str, service: str, **kwargs):
        details = kwargs.get("details", {})
        details["service"] = service
        kwargs["details"] = details
        super().__init__(message, ErrorCode.EXTERNAL_SERVICE_ERROR, **kwargs)


class TimeoutError(DeltranError):
    """Timeout error"""

    def __init__(self, message: str, timeout_seconds: float, **kwargs):
        details = kwargs.get("details", {})
        details["timeout_seconds"] = timeout_seconds
        kwargs["details"] = details
        super().__init__(message, ErrorCode.TIMEOUT_ERROR, **kwargs)


# HTTP status code mapping
ERROR_STATUS_MAPPING = {
    ErrorCode.VALIDATION_ERROR: 400,
    ErrorCode.NOT_FOUND: 404,
    ErrorCode.UNAUTHORIZED: 401,
    ErrorCode.FORBIDDEN: 403,
    ErrorCode.CONFLICT: 409,
    ErrorCode.RATE_LIMITED: 429,
    ErrorCode.INSUFFICIENT_FUNDS: 400,
    ErrorCode.INVALID_ACCOUNT: 400,
    ErrorCode.INVALID_AMOUNT: 400,
    ErrorCode.PAYMENT_EXPIRED: 410,
    ErrorCode.PAYMENT_CANCELLED: 409,
    ErrorCode.DUPLICATE_PAYMENT: 409,
    ErrorCode.SETTLEMENT_FAILED: 500,
    ErrorCode.BATCH_CLOSED: 409,
    ErrorCode.NETTING_ERROR: 500,
    ErrorCode.LIQUIDITY_UNAVAILABLE: 503,
    ErrorCode.RISK_THRESHOLD_EXCEEDED: 403,
    ErrorCode.RISK_ASSESSMENT_FAILED: 500,
    ErrorCode.HIGH_RISK_TRANSACTION: 403,
    ErrorCode.SANCTIONS_VIOLATION: 403,
    ErrorCode.PEP_VIOLATION: 403,
    ErrorCode.TRAVEL_RULE_VIOLATION: 400,
    ErrorCode.INCOMPLETE_KYC: 400,
    ErrorCode.COMPLIANCE_CHECK_FAILED: 500,
    ErrorCode.BLOCK_VALIDATION_FAILED: 500,
    ErrorCode.CONSENSUS_FAILED: 500,
    ErrorCode.INVALID_SIGNATURE: 400,
    ErrorCode.DOUBLE_SPEND: 409,
    ErrorCode.EXTERNAL_SERVICE_ERROR: 502,
    ErrorCode.TIMEOUT_ERROR: 504,
    ErrorCode.NETWORK_ERROR: 502,
    ErrorCode.INTERNAL_ERROR: 500,
}


def get_http_status(error_code: ErrorCode) -> int:
    """Get HTTP status code for error code"""
    return ERROR_STATUS_MAPPING.get(error_code, 500)


# Common error instances
def insufficient_funds(transaction_id: str = None) -> PaymentError:
    return PaymentError(
        "Insufficient funds for transaction",
        ErrorCode.INSUFFICIENT_FUNDS,
        transaction_id=transaction_id
    )


def invalid_account(account: str) -> ValidationError:
    return ValidationError(
        f"Invalid account: {account}",
        field="account",
        details={"account": account}
    )


def sanctions_violation(entity: str) -> ComplianceError:
    return ComplianceError(
        f"Sanctions violation detected for entity: {entity}",
        ErrorCode.SANCTIONS_VIOLATION,
        details={"entity": entity}
    )


def risk_threshold_exceeded(score: float, threshold: float) -> RiskError:
    return RiskError(
        f"Risk score {score} exceeds threshold {threshold}",
        ErrorCode.RISK_THRESHOLD_EXCEEDED,
        details={"score": score, "threshold": threshold}
    )


def settlement_batch_closed(batch_id: str) -> SettlementError:
    return SettlementError(
        f"Settlement batch {batch_id} is already closed",
        ErrorCode.BATCH_CLOSED,
        details={"batch_id": batch_id}
    )


def external_service_timeout(service: str, timeout: float) -> ExternalServiceError:
    return ExternalServiceError(
        f"Timeout calling {service} after {timeout}s",
        service=service,
        details={"timeout_seconds": timeout}
    )