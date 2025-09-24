import os
from typing import Optional, Dict, Any
from opentelemetry import trace
from opentelemetry.exporter.jaeger.thrift import JaegerExporter
from opentelemetry.instrumentation.fastapi import FastAPIInstrumentor
from opentelemetry.instrumentation.asyncpg import AsyncPGInstrumentor
from opentelemetry.instrumentation.redis import RedisInstrumentor
from opentelemetry.instrumentation.requests import RequestsInstrumentor
from opentelemetry.sdk.trace import TracerProvider
from opentelemetry.sdk.trace.export import BatchSpanProcessor
from opentelemetry.sdk.resources import Resource


def setup_tracing(service_name: str, jaeger_endpoint: Optional[str] = None) -> trace.Tracer:
    """Setup OpenTelemetry tracing with Jaeger export"""

    # Create resource
    resource = Resource.create({
        "service.name": service_name,
        "service.version": "1.0.0",
        "deployment.environment": os.getenv("ENVIRONMENT", "development")
    })

    # Setup tracer provider
    trace.set_tracer_provider(TracerProvider(resource=resource))
    tracer = trace.get_tracer(service_name)

    # Setup Jaeger exporter if endpoint provided
    if jaeger_endpoint:
        jaeger_exporter = JaegerExporter(
            endpoint=jaeger_endpoint,
            agent_host_name="localhost",
            agent_port=6831,
        )

        span_processor = BatchSpanProcessor(jaeger_exporter)
        trace.get_tracer_provider().add_span_processor(span_processor)

    # Auto-instrument common libraries
    AsyncPGInstrumentor().instrument()
    RedisInstrumentor().instrument()
    RequestsInstrumentor().instrument()

    return tracer


def instrument_fastapi(app, service_name: str):
    """Instrument FastAPI application"""
    FastAPIInstrumentor.instrument_app(
        app,
        server_request_hook=_server_request_hook,
        client_request_hook=_client_request_hook,
        excluded_urls="health,metrics,docs,openapi.json"
    )


def _server_request_hook(span: trace.Span, scope: Dict[str, Any]):
    """Hook for incoming requests"""
    if span and span.is_recording():
        # Add custom attributes
        headers = scope.get("headers", {})
        for name, value in headers:
            name = name.decode().lower()
            if name in ["user-agent", "x-forwarded-for", "x-real-ip", "idempotency-key"]:
                span.set_attribute(f"http.request.header.{name}", value.decode())


def _client_request_hook(span: trace.Span, request):
    """Hook for outgoing requests"""
    if span and span.is_recording():
        # Add request details
        span.set_attribute("http.client.request_size", len(request.body or ""))


class TraceContext:
    """Context manager for creating spans"""

    def __init__(self, tracer: trace.Tracer, operation_name: str, **attributes):
        self.tracer = tracer
        self.operation_name = operation_name
        self.attributes = attributes
        self.span = None

    def __enter__(self):
        self.span = self.tracer.start_span(self.operation_name)

        # Set attributes
        for key, value in self.attributes.items():
            if value is not None:
                self.span.set_attribute(key, str(value))

        return self.span

    def __exit__(self, exc_type, exc_val, exc_tb):
        if self.span:
            if exc_type is not None:
                self.span.record_exception(exc_val)
                self.span.set_status(trace.Status(trace.StatusCode.ERROR, str(exc_val)))
            else:
                self.span.set_status(trace.Status(trace.StatusCode.OK))

            self.span.end()


def get_current_trace_id() -> Optional[str]:
    """Get current trace ID"""
    span = trace.get_current_span()
    if span and span.get_span_context().is_valid:
        return f"{span.get_span_context().trace_id:032x}"
    return None


def get_current_span_id() -> Optional[str]:
    """Get current span ID"""
    span = trace.get_current_span()
    if span and span.get_span_context().is_valid:
        return f"{span.get_span_context().span_id:016x}"
    return None


def add_span_attributes(**attributes):
    """Add attributes to current span"""
    span = trace.get_current_span()
    if span and span.is_recording():
        for key, value in attributes.items():
            if value is not None:
                span.set_attribute(key, str(value))


def add_span_event(name: str, **attributes):
    """Add event to current span"""
    span = trace.get_current_span()
    if span and span.is_recording():
        span.add_event(name, attributes)


def set_span_error(error: Exception):
    """Set span as error"""
    span = trace.get_current_span()
    if span and span.is_recording():
        span.record_exception(error)
        span.set_status(trace.Status(trace.StatusCode.ERROR, str(error)))


# Decorators
def trace_operation(operation_name: str, **span_attributes):
    """Decorator to trace a function or method"""
    def decorator(func):
        async def async_wrapper(*args, **kwargs):
            tracer = trace.get_tracer(__name__)
            with TraceContext(tracer, operation_name, **span_attributes) as span:
                # Add function details
                span.set_attribute("code.function", func.__name__)
                span.set_attribute("code.module", func.__module__)

                try:
                    result = await func(*args, **kwargs)
                    span.set_attribute("operation.success", True)
                    return result
                except Exception as e:
                    span.set_attribute("operation.success", False)
                    span.record_exception(e)
                    raise

        def sync_wrapper(*args, **kwargs):
            tracer = trace.get_tracer(__name__)
            with TraceContext(tracer, operation_name, **span_attributes) as span:
                # Add function details
                span.set_attribute("code.function", func.__name__)
                span.set_attribute("code.module", func.__module__)

                try:
                    result = func(*args, **kwargs)
                    span.set_attribute("operation.success", True)
                    return result
                except Exception as e:
                    span.set_attribute("operation.success", False)
                    span.record_exception(e)
                    raise

        import asyncio
        if asyncio.iscoroutinefunction(func):
            return async_wrapper
        else:
            return sync_wrapper

    return decorator


# Global tracer instance
_tracer: Optional[trace.Tracer] = None


def get_tracer() -> trace.Tracer:
    """Get global tracer instance"""
    global _tracer
    if _tracer is None:
        _tracer = trace.get_tracer("deltran")
    return _tracer


def init_tracing(service_name: str, jaeger_endpoint: Optional[str] = None):
    """Initialize tracing for the service"""
    global _tracer
    _tracer = setup_tracing(service_name, jaeger_endpoint)