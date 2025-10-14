package observability

import (
	"time"

	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
)

// Metrics holds all Prometheus metrics for the gateway
type Metrics struct {
	// HTTP metrics
	HTTPRequestsTotal    *prometheus.CounterVec
	HTTPRequestDuration  *prometheus.HistogramVec
	HTTPResponseSize     *prometheus.HistogramVec

	// Payment metrics
	PaymentsTotal        *prometheus.CounterVec
	PaymentProcessingTime *prometheus.HistogramVec
	PaymentAmount        *prometheus.HistogramVec

	// ISO 20022 validation metrics
	ISO20022ValidationTotal    *prometheus.CounterVec
	ISO20022ValidationDuration *prometheus.HistogramVec
	ISO20022ValidationErrors   *prometheus.CounterVec

	// Sanctions screening metrics
	SanctionsScreeningTotal    *prometheus.CounterVec
	SanctionsScreeningDuration *prometheus.HistogramVec
	SanctionsHitsTotal         *prometheus.CounterVec
	SanctionsMatchScore        *prometheus.HistogramVec

	// Database metrics
	DBConnectionsActive  prometheus.Gauge
	DBQueriesTotal       *prometheus.CounterVec
	DBQueryDuration      *prometheus.HistogramVec

	// Redis metrics
	RedisOperationsTotal *prometheus.CounterVec
	RedisOperationDuration *prometheus.HistogramVec

	// WebSocket metrics
	WSConnectionsActive  prometheus.Gauge
	WSMessagesTotal      *prometheus.CounterVec
	WSConnectionDuration *prometheus.HistogramVec

	// NATS metrics
	NATSMessagesPublished *prometheus.CounterVec
	NATSMessagesConsumed  *prometheus.CounterVec
	NATSPublishDuration   *prometheus.HistogramVec
	NATSErrors            *prometheus.CounterVec

	// System health metrics
	ServiceUptime         prometheus.Gauge
	ServiceHealthy        prometheus.Gauge
	LastHealthCheck       prometheus.Gauge
}

// NewMetrics creates and registers all Prometheus metrics
func NewMetrics(namespace, subsystem string) *Metrics {
	m := &Metrics{
		// HTTP metrics
		HTTPRequestsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "http_requests_total",
				Help:      "Total number of HTTP requests",
			},
			[]string{"method", "path", "status"},
		),
		HTTPRequestDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "http_request_duration_seconds",
				Help:      "HTTP request duration in seconds",
				Buckets:   prometheus.DefBuckets,
			},
			[]string{"method", "path"},
		),
		HTTPResponseSize: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "http_response_size_bytes",
				Help:      "HTTP response size in bytes",
				Buckets:   prometheus.ExponentialBuckets(100, 10, 8),
			},
			[]string{"method", "path"},
		),

		// Payment metrics
		PaymentsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "payments_total",
				Help:      "Total number of payments processed",
			},
			[]string{"status", "currency"},
		),
		PaymentProcessingTime: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "payment_processing_duration_seconds",
				Help:      "Payment processing duration in seconds",
				Buckets:   []float64{.005, .01, .025, .05, .1, .25, .5, 1, 2.5, 5, 10},
			},
			[]string{"currency"},
		),
		PaymentAmount: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "payment_amount",
				Help:      "Payment amount distribution",
				Buckets:   []float64{100, 500, 1000, 5000, 10000, 50000, 100000, 500000, 1000000},
			},
			[]string{"currency"},
		),

		// ISO 20022 validation metrics
		ISO20022ValidationTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "iso20022_validations_total",
				Help:      "Total number of ISO 20022 validations",
			},
			[]string{"result"}, // valid, invalid
		),
		ISO20022ValidationDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "iso20022_validation_duration_seconds",
				Help:      "ISO 20022 validation duration in seconds",
				Buckets:   []float64{.001, .0025, .005, .01, .025, .05, .1},
			},
			[]string{"result"},
		),
		ISO20022ValidationErrors: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "iso20022_validation_errors_total",
				Help:      "Total number of ISO 20022 validation errors by type",
			},
			[]string{"error_code"},
		),

		// Sanctions screening metrics
		SanctionsScreeningTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "sanctions_screenings_total",
				Help:      "Total number of sanctions screenings",
			},
			[]string{"result"}, // hit, clear
		),
		SanctionsScreeningDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "sanctions_screening_duration_seconds",
				Help:      "Sanctions screening duration in seconds",
				Buckets:   []float64{.001, .0025, .005, .01, .025, .05, .1},
			},
			[]string{"result"},
		),
		SanctionsHitsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "sanctions_hits_total",
				Help:      "Total number of sanctions hits",
			},
			[]string{"risk_level", "source"},
		),
		SanctionsMatchScore: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "sanctions_match_score",
				Help:      "Sanctions match score distribution",
				Buckets:   []float64{0.5, 0.6, 0.7, 0.8, 0.9, 0.95, 1.0},
			},
			[]string{"source"},
		),

		// Database metrics
		DBConnectionsActive: promauto.NewGauge(
			prometheus.GaugeOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "db_connections_active",
				Help:      "Number of active database connections",
			},
		),
		DBQueriesTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "db_queries_total",
				Help:      "Total number of database queries",
			},
			[]string{"operation", "table"},
		),
		DBQueryDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "db_query_duration_seconds",
				Help:      "Database query duration in seconds",
				Buckets:   []float64{.001, .005, .01, .025, .05, .1, .25, .5, 1},
			},
			[]string{"operation", "table"},
		),

		// Redis metrics
		RedisOperationsTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "redis_operations_total",
				Help:      "Total number of Redis operations",
			},
			[]string{"operation", "status"},
		),
		RedisOperationDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "redis_operation_duration_seconds",
				Help:      "Redis operation duration in seconds",
				Buckets:   []float64{.0001, .0005, .001, .0025, .005, .01, .025, .05},
			},
			[]string{"operation"},
		),

		// WebSocket metrics
		WSConnectionsActive: promauto.NewGauge(
			prometheus.GaugeOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "websocket_connections_active",
				Help:      "Number of active WebSocket connections",
			},
		),
		WSMessagesTotal: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "websocket_messages_total",
				Help:      "Total number of WebSocket messages",
			},
			[]string{"direction", "type"}, // direction: sent/received, type: payment_update/metrics_update/etc
		),
		WSConnectionDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "websocket_connection_duration_seconds",
				Help:      "WebSocket connection duration in seconds",
				Buckets:   []float64{1, 10, 60, 300, 600, 1800, 3600},
			},
			[]string{},
		),

		// NATS metrics
		NATSMessagesPublished: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "nats_messages_published_total",
				Help:      "Total number of NATS messages published",
			},
			[]string{"subject", "status"},
		),
		NATSMessagesConsumed: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "nats_messages_consumed_total",
				Help:      "Total number of NATS messages consumed",
			},
			[]string{"subject", "status"},
		),
		NATSPublishDuration: promauto.NewHistogramVec(
			prometheus.HistogramOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "nats_publish_duration_seconds",
				Help:      "NATS message publish duration in seconds",
				Buckets:   []float64{.001, .0025, .005, .01, .025, .05, .1},
			},
			[]string{"subject"},
		),
		NATSErrors: promauto.NewCounterVec(
			prometheus.CounterOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "nats_errors_total",
				Help:      "Total number of NATS errors",
			},
			[]string{"operation", "error_type"},
		),

		// System health metrics
		ServiceUptime: promauto.NewGauge(
			prometheus.GaugeOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "service_uptime_seconds",
				Help:      "Service uptime in seconds",
			},
		),
		ServiceHealthy: promauto.NewGauge(
			prometheus.GaugeOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "service_healthy",
				Help:      "Service health status (1 = healthy, 0 = unhealthy)",
			},
		),
		LastHealthCheck: promauto.NewGauge(
			prometheus.GaugeOpts{
				Namespace: namespace,
				Subsystem: subsystem,
				Name:      "last_health_check_timestamp",
				Help:      "Timestamp of last health check",
			},
		),
	}

	// Initialize health to healthy
	m.ServiceHealthy.Set(1)
	m.LastHealthCheck.SetToCurrentTime()

	return m
}

// RecordHTTPRequest records an HTTP request
func (m *Metrics) RecordHTTPRequest(method, path string, status int, duration time.Duration, responseSize int64) {
	m.HTTPRequestsTotal.WithLabelValues(method, path, statusCode(status)).Inc()
	m.HTTPRequestDuration.WithLabelValues(method, path).Observe(duration.Seconds())
	m.HTTPResponseSize.WithLabelValues(method, path).Observe(float64(responseSize))
}

// RecordPayment records a payment
func (m *Metrics) RecordPayment(status, currency string, amount float64, processingTime time.Duration) {
	m.PaymentsTotal.WithLabelValues(status, currency).Inc()
	m.PaymentProcessingTime.WithLabelValues(currency).Observe(processingTime.Seconds())
	m.PaymentAmount.WithLabelValues(currency).Observe(amount)
}

// RecordISO20022Validation records an ISO 20022 validation
func (m *Metrics) RecordISO20022Validation(valid bool, duration time.Duration, errorCodes []string) {
	result := "valid"
	if !valid {
		result = "invalid"
	}

	m.ISO20022ValidationTotal.WithLabelValues(result).Inc()
	m.ISO20022ValidationDuration.WithLabelValues(result).Observe(duration.Seconds())

	for _, code := range errorCodes {
		m.ISO20022ValidationErrors.WithLabelValues(code).Inc()
	}
}

// RecordSanctionsScreening records a sanctions screening
func (m *Metrics) RecordSanctionsScreening(hit bool, riskLevel string, duration time.Duration, matches []SanctionsMatch) {
	result := "clear"
	if hit {
		result = "hit"
	}

	m.SanctionsScreeningTotal.WithLabelValues(result).Inc()
	m.SanctionsScreeningDuration.WithLabelValues(result).Observe(duration.Seconds())

	if hit {
		for _, match := range matches {
			m.SanctionsHitsTotal.WithLabelValues(riskLevel, match.Source).Inc()
			m.SanctionsMatchScore.WithLabelValues(match.Source).Observe(match.MatchScore)
		}
	}
}

// SanctionsMatch represents a sanctions match for metrics
type SanctionsMatch struct {
	Source     string
	MatchScore float64
}

// RecordDBQuery records a database query
func (m *Metrics) RecordDBQuery(operation, table string, duration time.Duration) {
	m.DBQueriesTotal.WithLabelValues(operation, table).Inc()
	m.DBQueryDuration.WithLabelValues(operation, table).Observe(duration.Seconds())
}

// RecordRedisOperation records a Redis operation
func (m *Metrics) RecordRedisOperation(operation, status string, duration time.Duration) {
	m.RedisOperationsTotal.WithLabelValues(operation, status).Inc()
	m.RedisOperationDuration.WithLabelValues(operation).Observe(duration.Seconds())
}

// RecordWSConnection records WebSocket connection metrics
func (m *Metrics) RecordWSConnection(active int) {
	m.WSConnectionsActive.Set(float64(active))
}

// RecordWSMessage records a WebSocket message
func (m *Metrics) RecordWSMessage(direction, messageType string) {
	m.WSMessagesTotal.WithLabelValues(direction, messageType).Inc()
}

// RecordWSDisconnect records WebSocket disconnect duration
func (m *Metrics) RecordWSDisconnect(duration time.Duration) {
	m.WSConnectionDuration.WithLabelValues().Observe(duration.Seconds())
}

// RecordNATSPublish records a NATS publish
func (m *Metrics) RecordNATSPublish(subject, status string, duration time.Duration) {
	m.NATSMessagesPublished.WithLabelValues(subject, status).Inc()
	m.NATSPublishDuration.WithLabelValues(subject).Observe(duration.Seconds())
}

// RecordNATSConsume records a NATS message consumption
func (m *Metrics) RecordNATSConsume(subject, status string) {
	m.NATSMessagesConsumed.WithLabelValues(subject, status).Inc()
}

// RecordNATSError records a NATS error
func (m *Metrics) RecordNATSError(operation, errorType string) {
	m.NATSErrors.WithLabelValues(operation, errorType).Inc()
}

// UpdateServiceHealth updates service health status
func (m *Metrics) UpdateServiceHealth(healthy bool) {
	if healthy {
		m.ServiceHealthy.Set(1)
	} else {
		m.ServiceHealthy.Set(0)
	}
	m.LastHealthCheck.SetToCurrentTime()
}

// statusCode converts HTTP status code to string category
func statusCode(code int) string {
	switch {
	case code >= 200 && code < 300:
		return "2xx"
	case code >= 300 && code < 400:
		return "3xx"
	case code >= 400 && code < 500:
		return "4xx"
	case code >= 500:
		return "5xx"
	default:
		return "unknown"
	}
}

// StartUptimeTracking starts tracking service uptime
func (m *Metrics) StartUptimeTracking(startTime time.Time) {
	go func() {
		ticker := time.NewTicker(10 * time.Second)
		defer ticker.Stop()

		for range ticker.C {
			uptime := time.Since(startTime).Seconds()
			m.ServiceUptime.Set(uptime)
		}
	}()
}
