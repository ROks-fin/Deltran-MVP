package middleware

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"time"
)

// AnalyticsCollector sends analytics to the collector service
type AnalyticsCollector struct {
	collectorURL string
	httpClient   *http.Client
}

// NewAnalyticsCollector creates a new analytics collector
func NewAnalyticsCollector(collectorURL string) *AnalyticsCollector {
	return &AnalyticsCollector{
		collectorURL: collectorURL,
		httpClient: &http.Client{
			Timeout: 5 * time.Second,
		},
	}
}

// TransactionEvent represents a transaction event for analytics
type TransactionEvent struct {
	TransactionID string                 `json:"transaction_id"`
	EventType     string                 `json:"event_type"`
	Timestamp     time.Time              `json:"timestamp"`
	Service       string                 `json:"service"`
	Data          map[string]interface{} `json:"data"`
}

// responseWriter wraps http.ResponseWriter to capture status code
type responseWriter struct {
	http.ResponseWriter
	statusCode int
	bytes      int
}

func newResponseWriter(w http.ResponseWriter) *responseWriter {
	return &responseWriter{
		ResponseWriter: w,
		statusCode:     http.StatusOK,
	}
}

func (rw *responseWriter) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}

func (rw *responseWriter) Write(b []byte) (int, error) {
	n, err := rw.ResponseWriter.Write(b)
	rw.bytes += n
	return n, err
}

// Middleware returns the analytics middleware
func (ac *AnalyticsCollector) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Skip analytics for health checks
		if r.URL.Path == "/health" {
			next.ServeHTTP(w, r)
			return
		}

		start := time.Now()

		// Wrap response writer to capture status code
		wrapped := newResponseWriter(w)

		// Get transaction ID if exists
		txID := r.Header.Get("X-Transaction-ID")
		if txID == "" {
			txID = fmt.Sprintf("TXN-%d", time.Now().UnixNano())
			r.Header.Set("X-Transaction-ID", txID)
		}

		// Record gateway received event
		go ac.recordEvent(TransactionEvent{
			TransactionID: txID,
			EventType:     "gateway",
			Timestamp:     start,
			Service:       "gateway",
			Data: map[string]interface{}{
				"method":     r.Method,
				"path":       r.URL.Path,
				"user_agent": r.UserAgent(),
				"remote_addr": r.RemoteAddr,
			},
		})

		// Process request
		next.ServeHTTP(wrapped, r)

		// Calculate duration
		duration := time.Since(start)

		// Record completion event
		go ac.recordEvent(TransactionEvent{
			TransactionID: txID,
			EventType:     determineEventType(wrapped.statusCode),
			Timestamp:     time.Now(),
			Service:       "gateway",
			Data: map[string]interface{}{
				"status_code":   wrapped.statusCode,
				"duration_ms":   duration.Milliseconds(),
				"response_size": wrapped.bytes,
				"success":       wrapped.statusCode < 400,
			},
		})
	})
}

// recordEvent sends an event to the analytics collector
func (ac *AnalyticsCollector) recordEvent(event TransactionEvent) {
	payload, err := json.Marshal(event)
	if err != nil {
		fmt.Printf("Failed to marshal analytics event: %v\n", err)
		return
	}

	req, err := http.NewRequest("POST", ac.collectorURL+"/events/transaction", bytes.NewBuffer(payload))
	if err != nil {
		fmt.Printf("Failed to create analytics request: %v\n", err)
		return
	}

	req.Header.Set("Content-Type", "application/json")

	resp, err := ac.httpClient.Do(req)
	if err != nil {
		// Don't log errors to avoid noise - analytics is best-effort
		return
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		// Analytics failure shouldn't affect main flow
		return
	}
}

// determineEventType determines the event type based on status code
func determineEventType(statusCode int) string {
	if statusCode >= 200 && statusCode < 300 {
		return "completed"
	}
	if statusCode >= 400 {
		return "failed"
	}
	return "processing"
}

// CreateTransaction creates a transaction record in analytics
func (ac *AnalyticsCollector) CreateTransaction(txID, senderID, receiverID, currency string, amount float64) error {
	payload := map[string]interface{}{
		"transaction_id": txID,
		"sender_id":      senderID,
		"receiver_id":    receiverID,
		"amount":         amount,
		"currency":       currency,
		"test_run_id":    nil,
		"test_scenario":  nil,
		"load_level":     nil,
	}

	data, err := json.Marshal(payload)
	if err != nil {
		return err
	}

	req, err := http.NewRequest("POST", ac.collectorURL+"/transactions", bytes.NewBuffer(data))
	if err != nil {
		return err
	}

	req.Header.Set("Content-Type", "application/json")

	resp, err := ac.httpClient.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusCreated && resp.StatusCode != http.StatusOK {
		return fmt.Errorf("analytics returned status %d", resp.StatusCode)
	}

	return nil
}
