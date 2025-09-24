import logging
import sys
from typing import Dict, Any, Optional
import json
import traceback
from datetime import datetime


class PIIMaskingFilter(logging.Filter):
    """Filter to mask PII in log messages"""

    PII_PATTERNS = [
        # Account numbers, IBANs
        (r'\b[A-Z]{2}\d{2}[A-Z0-9]{1,30}\b', '***IBAN***'),
        # Credit card numbers
        (r'\b\d{4}[-\s]?\d{4}[-\s]?\d{4}[-\s]?\d{4}\b', '***CARD***'),
        # SSN patterns
        (r'\b\d{3}-\d{2}-\d{4}\b', '***SSN***'),
        # Email addresses
        (r'\b[A-Za-z0-9._%+-]+@[A-Za-z0-9.-]+\.[A-Z|a-z]{2,}\b', '***EMAIL***'),
        # Phone numbers
        (r'\b\+?1?[-.\s]?\(?[0-9]{3}\)?[-.\s]?[0-9]{3}[-.\s]?[0-9]{4}\b', '***PHONE***'),
        # Generic ID patterns
        (r'\b(?:private_id|id_number)["\':\s]*["\']?([^"\'\\,}\s]+)', r'\1***ID***'),
    ]

    def filter(self, record):
        """Apply PII masking to log record"""
        import re

        # Mask PII in the message
        if hasattr(record, 'msg'):
            message = str(record.msg)
            for pattern, replacement in self.PII_PATTERNS:
                message = re.sub(pattern, replacement, message, flags=re.IGNORECASE)
            record.msg = message

        # Mask PII in arguments
        if hasattr(record, 'args') and record.args:
            masked_args = []
            for arg in record.args:
                if isinstance(arg, str):
                    for pattern, replacement in self.PII_PATTERNS:
                        arg = re.sub(pattern, replacement, arg, flags=re.IGNORECASE)
                masked_args.append(arg)
            record.args = tuple(masked_args)

        return True


class StructuredFormatter(logging.Formatter):
    """Structured JSON formatter with trace context"""

    def __init__(self, service_name: str = "deltran"):
        super().__init__()
        self.service_name = service_name

    def format(self, record) -> str:
        """Format log record as structured JSON"""

        # Base log entry
        log_entry = {
            "timestamp": datetime.utcnow().isoformat() + "Z",
            "level": record.levelname,
            "service": self.service_name,
            "logger": record.name,
            "message": record.getMessage(),
        }

        # Add trace context if available
        trace_id = getattr(record, 'trace_id', None)
        span_id = getattr(record, 'span_id', None)
        if trace_id:
            log_entry["trace_id"] = trace_id
        if span_id:
            log_entry["span_id"] = span_id

        # Add request context
        request_id = getattr(record, 'request_id', None)
        user_id = getattr(record, 'user_id', None)
        if request_id:
            log_entry["request_id"] = request_id
        if user_id:
            log_entry["user_id"] = user_id

        # Add extra fields
        extra_fields = {}
        for key, value in record.__dict__.items():
            if key not in ['name', 'msg', 'args', 'levelname', 'levelno', 'pathname',
                          'filename', 'module', 'lineno', 'funcName', 'created',
                          'msecs', 'relativeCreated', 'thread', 'threadName',
                          'processName', 'process', 'trace_id', 'span_id',
                          'request_id', 'user_id']:
                extra_fields[key] = value

        if extra_fields:
            log_entry["extra"] = extra_fields

        # Add exception info if present
        if record.exc_info:
            log_entry["exception"] = {
                "type": record.exc_info[0].__name__,
                "message": str(record.exc_info[1]),
                "traceback": traceback.format_exception(*record.exc_info)
            }

        # Add source location for errors and warnings
        if record.levelno >= logging.WARNING:
            log_entry["source"] = {
                "file": record.pathname,
                "line": record.lineno,
                "function": record.funcName
            }

        return json.dumps(log_entry, default=str)


def setup_logging(service_name: str, log_level: str = "INFO",
                 enable_pii_masking: bool = True) -> logging.Logger:
    """Setup structured logging for the service"""

    # Configure root logger
    root_logger = logging.getLogger()
    root_logger.setLevel(getattr(logging, log_level.upper()))

    # Remove existing handlers
    for handler in root_logger.handlers[:]:
        root_logger.removeHandler(handler)

    # Create console handler
    console_handler = logging.StreamHandler(sys.stdout)
    console_handler.setFormatter(StructuredFormatter(service_name))

    # Add PII masking filter
    if enable_pii_masking:
        console_handler.addFilter(PIIMaskingFilter())

    root_logger.addHandler(console_handler)

    # Configure third-party loggers
    logging.getLogger("asyncpg").setLevel(logging.WARNING)
    logging.getLogger("nats").setLevel(logging.WARNING)
    logging.getLogger("uvicorn.access").setLevel(logging.WARNING)

    # Create service logger
    logger = logging.getLogger(service_name)
    return logger


class LogContext:
    """Context manager for adding context to log records"""

    def __init__(self, **context):
        self.context = context
        self.old_factory = None

    def __enter__(self):
        self.old_factory = logging.getLogRecordFactory()

        def record_factory(*args, **kwargs):
            record = self.old_factory(*args, **kwargs)
            for key, value in self.context.items():
                setattr(record, key, value)
            return record

        logging.setLogRecordFactory(record_factory)
        return self

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.old_factory:
            logging.setLogRecordFactory(self.old_factory)


def get_logger(name: str) -> logging.Logger:
    """Get logger instance"""
    return logging.getLogger(name)


# Convenience functions for common log contexts
def with_trace_context(trace_id: str, span_id: Optional[str] = None):
    """Add trace context to logs"""
    context = {"trace_id": trace_id}
    if span_id:
        context["span_id"] = span_id
    return LogContext(**context)


def with_request_context(request_id: str, user_id: Optional[str] = None):
    """Add request context to logs"""
    context = {"request_id": request_id}
    if user_id:
        context["user_id"] = user_id
    return LogContext(**context)


def with_transaction_context(transaction_id: str, uetr: Optional[str] = None):
    """Add transaction context to logs"""
    context = {"transaction_id": transaction_id}
    if uetr:
        context["uetr"] = uetr
    return LogContext(**context)