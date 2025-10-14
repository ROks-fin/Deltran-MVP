package observability

import (
	"context"
	"fmt"
	"io"
	"time"

	"github.com/rs/zerolog/log"
	"go.opentelemetry.io/otel"
	"go.opentelemetry.io/otel/attribute"
	"go.opentelemetry.io/otel/codes"
	"go.opentelemetry.io/otel/exporters/otlp/otlptrace/otlptracegrpc"
	"go.opentelemetry.io/otel/propagation"
	"go.opentelemetry.io/otel/sdk/resource"
	tracesdk "go.opentelemetry.io/otel/sdk/trace"
	semconv "go.opentelemetry.io/otel/semconv/v1.17.0"
	"go.opentelemetry.io/otel/trace"
)

// TracerConfig holds tracer configuration
type TracerConfig struct {
	ServiceName    string
	ServiceVersion string
	Environment    string
	JaegerEndpoint string
	Enabled        bool
	SampleRate     float64 // 0.0 - 1.0
}

// InitTracer initializes OpenTelemetry tracing with OTLP exporter
func InitTracer(config TracerConfig) (trace.TracerProvider, io.Closer, error) {
	if !config.Enabled {
		log.Info().Msg("Distributed tracing is disabled")
		return trace.NewNoopTracerProvider(), io.NopCloser(nil), nil
	}

	ctx := context.Background()

	// Create OTLP exporter (compatible with Jaeger, Grafana Tempo, etc.)
	exp, err := otlptracegrpc.New(ctx,
		otlptracegrpc.WithEndpoint(config.JaegerEndpoint),
		otlptracegrpc.WithInsecure(), // Use WithTLSCredentials() for production
	)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create OTLP exporter: %w", err)
	}

	// Create resource with service information
	res, err := resource.Merge(
		resource.Default(),
		resource.NewWithAttributes(
			semconv.SchemaURL,
			semconv.ServiceName(config.ServiceName),
			semconv.ServiceVersion(config.ServiceVersion),
			attribute.String("environment", config.Environment),
		),
	)
	if err != nil {
		return nil, nil, fmt.Errorf("failed to create resource: %w", err)
	}

	// Create sampler based on sample rate
	var sampler tracesdk.Sampler
	if config.SampleRate >= 1.0 {
		sampler = tracesdk.AlwaysSample()
	} else if config.SampleRate <= 0.0 {
		sampler = tracesdk.NeverSample()
	} else {
		sampler = tracesdk.TraceIDRatioBased(config.SampleRate)
	}

	// Create trace provider
	tp := tracesdk.NewTracerProvider(
		tracesdk.WithBatcher(exp),
		tracesdk.WithResource(res),
		tracesdk.WithSampler(sampler),
	)

	// Set global tracer provider
	otel.SetTracerProvider(tp)

	// Set global propagator for context propagation
	otel.SetTextMapPropagator(propagation.NewCompositeTextMapPropagator(
		propagation.TraceContext{},
		propagation.Baggage{},
	))

	log.Info().
		Str("service", config.ServiceName).
		Str("endpoint", config.JaegerEndpoint).
		Float64("sample_rate", config.SampleRate).
		Msg("Distributed tracing initialized with OTLP")

	// Return closer for graceful shutdown
	closer := &tracerCloser{tp: tp}
	return tp, closer, nil
}

// tracerCloser implements io.Closer for tracer provider
type tracerCloser struct {
	tp *tracesdk.TracerProvider
}

func (c *tracerCloser) Close() error {
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()
	return c.tp.Shutdown(ctx)
}

// Tracer wraps OpenTelemetry tracer with convenience methods
type Tracer struct {
	tracer trace.Tracer
}

// NewTracer creates a new tracer
func NewTracer(name string) *Tracer {
	return &Tracer{
		tracer: otel.Tracer(name),
	}
}

// StartSpan starts a new span
func (t *Tracer) StartSpan(ctx context.Context, name string, attrs ...attribute.KeyValue) (context.Context, trace.Span) {
	return t.tracer.Start(ctx, name, trace.WithAttributes(attrs...))
}

// StartSpanWithKind starts a new span with specific kind
func (t *Tracer) StartSpanWithKind(ctx context.Context, name string, kind trace.SpanKind, attrs ...attribute.KeyValue) (context.Context, trace.Span) {
	return t.tracer.Start(ctx, name,
		trace.WithSpanKind(kind),
		trace.WithAttributes(attrs...),
	)
}

// AddEvent adds an event to the current span
func AddEvent(ctx context.Context, name string, attrs ...attribute.KeyValue) {
	span := trace.SpanFromContext(ctx)
	if span.IsRecording() {
		span.AddEvent(name, trace.WithAttributes(attrs...))
	}
}

// SetAttributes sets attributes on the current span
func SetAttributes(ctx context.Context, attrs ...attribute.KeyValue) {
	span := trace.SpanFromContext(ctx)
	if span.IsRecording() {
		span.SetAttributes(attrs...)
	}
}

// RecordError records an error on the current span
func RecordError(ctx context.Context, err error, attrs ...attribute.KeyValue) {
	span := trace.SpanFromContext(ctx)
	if span.IsRecording() {
		span.RecordError(err, trace.WithAttributes(attrs...))
	}
}

// SetStatus sets the status of the current span
func SetStatus(ctx context.Context, code codes.Code, description string) {
	span := trace.SpanFromContext(ctx)
	if span.IsRecording() {
		span.SetStatus(code, description)
	}
}

// Common attribute keys
var (
	// Payment attributes
	AttrPaymentID        = attribute.Key("payment.id")
	AttrPaymentReference = attribute.Key("payment.reference")
	AttrPaymentCurrency  = attribute.Key("payment.currency")
	AttrPaymentAmount    = attribute.Key("payment.amount")
	AttrPaymentStatus    = attribute.Key("payment.status")
	AttrSenderBIC        = attribute.Key("payment.sender_bic")
	AttrReceiverBIC      = attribute.Key("payment.receiver_bic")

	// Validation attributes
	AttrValidationResult = attribute.Key("validation.result")
	AttrValidationErrors = attribute.Key("validation.errors")

	// Sanctions attributes
	AttrSanctionsHit       = attribute.Key("sanctions.hit")
	AttrSanctionsRiskLevel = attribute.Key("sanctions.risk_level")
	AttrSanctionsMatches   = attribute.Key("sanctions.matches")
	AttrSanctionsSource    = attribute.Key("sanctions.source")

	// Database attributes
	AttrDBOperation = attribute.Key("db.operation")
	AttrDBTable     = attribute.Key("db.table")
	AttrDBQuery     = attribute.Key("db.query")

	// Redis attributes
	AttrRedisOperation = attribute.Key("redis.operation")
	AttrRedisKey       = attribute.Key("redis.key")

	// NATS attributes
	AttrNATSSubject = attribute.Key("nats.subject")
	AttrNATSReply   = attribute.Key("nats.reply")

	// HTTP attributes
	AttrHTTPMethod     = attribute.Key("http.method")
	AttrHTTPURL        = attribute.Key("http.url")
	AttrHTTPStatusCode = attribute.Key("http.status_code")
	AttrHTTPUserAgent  = attribute.Key("http.user_agent")
)

// Helper functions for common span operations

// TracePaymentProcessing creates a span for payment processing
func TracePaymentProcessing(ctx context.Context, tracer *Tracer, paymentID, reference, currency string, amount float64) (context.Context, trace.Span) {
	return tracer.StartSpan(ctx, "payment.process",
		AttrPaymentID.String(paymentID),
		AttrPaymentReference.String(reference),
		AttrPaymentCurrency.String(currency),
		AttrPaymentAmount.Float64(amount),
	)
}

// TraceISO20022Validation creates a span for ISO 20022 validation
func TraceISO20022Validation(ctx context.Context, tracer *Tracer, messageType string) (context.Context, trace.Span) {
	return tracer.StartSpan(ctx, "iso20022.validate",
		attribute.String("message.type", messageType),
	)
}

// TraceSanctionsScreening creates a span for sanctions screening
func TraceSanctionsScreening(ctx context.Context, tracer *Tracer, entity string) (context.Context, trace.Span) {
	return tracer.StartSpan(ctx, "sanctions.screen",
		attribute.String("entity.type", entity),
	)
}

// TraceDBQuery creates a span for database query
func TraceDBQuery(ctx context.Context, tracer *Tracer, operation, table string) (context.Context, trace.Span) {
	return tracer.StartSpanWithKind(ctx, "db.query", trace.SpanKindClient,
		AttrDBOperation.String(operation),
		AttrDBTable.String(table),
		semconv.DBSystemPostgreSQL,
	)
}

// TraceRedisOperation creates a span for Redis operation
func TraceRedisOperation(ctx context.Context, tracer *Tracer, operation, key string) (context.Context, trace.Span) {
	return tracer.StartSpanWithKind(ctx, "redis."+operation, trace.SpanKindClient,
		AttrRedisOperation.String(operation),
		AttrRedisKey.String(key),
		semconv.DBSystemRedis,
	)
}

// TraceNATSPublish creates a span for NATS publish
func TraceNATSPublish(ctx context.Context, tracer *Tracer, subject string) (context.Context, trace.Span) {
	return tracer.StartSpanWithKind(ctx, "nats.publish", trace.SpanKindProducer,
		AttrNATSSubject.String(subject),
		semconv.MessagingSystemNats,
	)
}

// TraceNATSConsume creates a span for NATS consume
func TraceNATSConsume(ctx context.Context, tracer *Tracer, subject string) (context.Context, trace.Span) {
	return tracer.StartSpanWithKind(ctx, "nats.consume", trace.SpanKindConsumer,
		AttrNATSSubject.String(subject),
		semconv.MessagingSystemNats,
	)
}
