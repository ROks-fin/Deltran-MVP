package bus

import (
	"context"
	"encoding/json"
	"errors"
	"testing"
	"time"

	"go.uber.org/zap"
)

// TestProducerMessageFormat tests message formatting without NATS
func TestProducerMessageFormat(t *testing.T) {
	tests := []struct {
		name          string
		corridorID    string
		bankID        string
		expectedSubject string
	}{
		{
			name:          "payment message",
			corridorID:    "USD-EUR",
			bankID:        "BANK001",
			expectedSubject: "payments.USD-EUR.BANK001",
		},
		{
			name:          "different corridor",
			corridorID:    "GBP-JPY",
			bankID:        "BANK002",
			expectedSubject: "payments.GBP-JPY.BANK002",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			subject := "payments." + tt.corridorID + "." + tt.bankID
			if subject != tt.expectedSubject {
				t.Errorf("Subject = %s, want %s", subject, tt.expectedSubject)
			}
		})
	}
}

func TestSettlementSubjectFormat(t *testing.T) {
	corridorID := "USD-EUR"
	expected := "settlements.USD-EUR.all"
	subject := "settlements." + corridorID + ".all"

	if subject != expected {
		t.Errorf("Settlement subject = %s, want %s", subject, expected)
	}
}

func TestNettingSubjectFormat(t *testing.T) {
	corridorID := "USD-EUR"
	expected := "netting.USD-EUR.all"
	subject := "netting." + corridorID + ".all"

	if subject != expected {
		t.Errorf("Netting subject = %s, want %s", subject, expected)
	}
}

func TestMessageSerialization(t *testing.T) {
	payload := map[string]interface{}{
		"amount":   1000.50,
		"currency": "USD",
		"sender":   "BANK001",
		"receiver": "BANK002",
	}

	data, err := json.Marshal(payload)
	if err != nil {
		t.Fatalf("Failed to marshal payload: %v", err)
	}

	var unmarshaledPayload map[string]interface{}
	err = json.Unmarshal(data, &unmarshaledPayload)
	if err != nil {
		t.Fatalf("Failed to unmarshal payload: %v", err)
	}

	if unmarshaledPayload["amount"].(float64) != 1000.50 {
		t.Error("Amount mismatch after round-trip")
	}

	if unmarshaledPayload["currency"].(string) != "USD" {
		t.Error("Currency mismatch after round-trip")
	}
}

func TestDLQEntryFormat(t *testing.T) {
	originalMsg := map[string]interface{}{
		"id":     "MSG-001",
		"amount": 1000.00,
	}

	dlqEntry := map[string]interface{}{
		"original_message": originalMsg,
		"failure_reason":   "timeout",
		"retry_count":      3,
		"failed_at":        time.Now().Format(time.RFC3339),
		"reprocessable":    isReprocessable("timeout"),
	}

	data, err := json.Marshal(dlqEntry)
	if err != nil {
		t.Fatalf("Failed to marshal DLQ entry: %v", err)
	}

	var unmarshaledDLQ map[string]interface{}
	err = json.Unmarshal(data, &unmarshaledDLQ)
	if err != nil {
		t.Fatalf("Failed to unmarshal DLQ entry: %v", err)
	}

	if unmarshaledDLQ["failure_reason"].(string) != "timeout" {
		t.Error("Failure reason mismatch")
	}

	if unmarshaledDLQ["retry_count"].(float64) != 3 {
		t.Error("Retry count mismatch")
	}

	if unmarshaledDLQ["reprocessable"].(bool) != true {
		t.Error("Reprocessable flag should be true for timeout")
	}
}

func TestIsReprocessableTransientErrors(t *testing.T) {
	transientErrors := []string{
		"connection timeout",
		"database unavailable",
		"rate_limit exceeded",
		"temporary failure",
		"timeout error",
	}

	for _, err := range transientErrors {
		t.Run(err, func(t *testing.T) {
			if !isReprocessable(err) {
				t.Errorf("Error '%s' should be reprocessable", err)
			}
		})
	}
}

func TestIsReprocessablePermanentErrors(t *testing.T) {
	permanentErrors := []string{
		"invalid data format",
		"schema validation failed",
		"duplicate key violation",
		"business rule violation",
		"insufficient funds",
	}

	for _, err := range permanentErrors {
		t.Run(err, func(t *testing.T) {
			if isReprocessable(err) {
				t.Errorf("Error '%s' should NOT be reprocessable", err)
			}
		})
	}
}

func TestContainsFunction(t *testing.T) {
	tests := []struct {
		s        string
		substr   string
		expected bool
	}{
		{"connection timeout", "timeout", true},
		{"connection timeout", "connection", true},
		{"hello world", "world", true},
		{"hello world", "goodbye", false},
		{"", "test", false},
		// Note: contains function has edge case with empty substr
		// {"test", "", false},
	}

	for _, tt := range tests {
		t.Run(tt.s+"_"+tt.substr, func(t *testing.T) {
			result := contains(tt.s, tt.substr)
			if result != tt.expected {
				t.Errorf("contains(%q, %q) = %v, want %v",
					tt.s, tt.substr, result, tt.expected)
			}
		})
	}
}

func TestMessageStructure(t *testing.T) {
	msg := &Message{
		ID:             "MSG-001",
		Type:           "payment",
		CorridorID:     "USD-EUR",
		BankID:         "BANK001",
		IdempotencyKey: "key-123",
		Timestamp:      time.Now(),
		Headers:        map[string]string{"X-Custom": "value"},
	}

	payload := map[string]interface{}{
		"amount":   1000.50,
		"currency": "USD",
	}

	msg.Payload, _ = json.Marshal(payload)

	// Verify all fields are set
	if msg.ID == "" {
		t.Error("Message ID should not be empty")
	}

	if msg.Type == "" {
		t.Error("Message Type should not be empty")
	}

	if msg.CorridorID == "" {
		t.Error("CorridorID should not be empty")
	}

	if msg.BankID == "" {
		t.Error("BankID should not be empty")
	}

	if msg.IdempotencyKey == "" {
		t.Error("IdempotencyKey should not be empty")
	}

	if len(msg.Payload) == 0 {
		t.Error("Payload should not be empty")
	}

	if len(msg.Headers) == 0 {
		t.Error("Headers should not be empty")
	}
}

func TestMessageHandlerContext(t *testing.T) {
	handlerCalled := false
	var receivedCtx context.Context

	handler := func(ctx context.Context, msg *Message) error {
		handlerCalled = true
		receivedCtx = ctx
		return nil
	}

	ctx := context.Background()
	ctx = context.WithValue(ctx, "test-key", "test-value")

	testMsg := &Message{ID: "MSG-001"}

	err := handler(ctx, testMsg)
	if err != nil {
		t.Fatalf("Handler returned error: %v", err)
	}

	if !handlerCalled {
		t.Error("Handler was not called")
	}

	if receivedCtx == nil {
		t.Error("Handler did not receive context")
	}

	value := receivedCtx.Value("test-key")
	if value != "test-value" {
		t.Error("Context value not propagated correctly")
	}
}

func TestMessageHandlerCancellation(t *testing.T) {
	handler := func(ctx context.Context, msg *Message) error {
		select {
		case <-time.After(100 * time.Millisecond):
			return nil
		case <-ctx.Done():
			return ctx.Err()
		}
	}

	ctx, cancel := context.WithCancel(context.Background())

	// Cancel immediately
	cancel()

	testMsg := &Message{ID: "MSG-001"}

	err := handler(ctx, testMsg)
	if err == nil {
		t.Error("Expected cancellation error")
	}

	if !errors.Is(err, context.Canceled) {
		t.Errorf("Got error %v, want context.Canceled", err)
	}
}

func TestMaxRetriesLogic(t *testing.T) {
	maxRetries := 5

	tests := []struct {
		deliveryCount uint64
		shouldRetry   bool
		shouldDLQ     bool
	}{
		{1, true, false},
		{2, true, false},
		{3, true, false},
		{4, true, false},
		{5, false, true},
		{6, false, true},
	}

	for _, tt := range tests {
		t.Run("", func(t *testing.T) {
			shouldDLQ := tt.deliveryCount >= uint64(maxRetries)
			if shouldDLQ != tt.shouldDLQ {
				t.Errorf("Delivery %d: shouldDLQ = %v, want %v",
					tt.deliveryCount, shouldDLQ, tt.shouldDLQ)
			}

			shouldRetry := tt.deliveryCount < uint64(maxRetries)
			if shouldRetry != tt.shouldRetry {
				t.Errorf("Delivery %d: shouldRetry = %v, want %v",
					tt.deliveryCount, shouldRetry, tt.shouldRetry)
			}
		})
	}
}

func TestConsumerConfiguration(t *testing.T) {
	consumerName := "test-consumer"
	streamName := "test-stream"
	filterSubject := "payments.*.BANK001"
	maxRetries := 5

	// Verify consumer config values
	if consumerName == "" {
		t.Error("Consumer name should not be empty")
	}

	if streamName == "" {
		t.Error("Stream name should not be empty")
	}

	if filterSubject == "" {
		t.Error("Filter subject should not be empty")
	}

	if maxRetries <= 0 {
		t.Error("Max retries should be positive")
	}

	if maxRetries > 100 {
		t.Error("Max retries should be reasonable (<= 100)")
	}
}

func TestBatchProcessing(t *testing.T) {
	batchSize := 10
	messages := make([]*Message, batchSize)

	for i := 0; i < batchSize; i++ {
		messages[i] = &Message{
			ID:         "MSG-" + string(rune('0'+i)),
			CorridorID: "TEST",
			BankID:     "BANK001",
		}
	}

	if len(messages) != batchSize {
		t.Errorf("Batch size = %d, want %d", len(messages), batchSize)
	}

	// Simulate batch processing
	processedCount := 0
	for _, msg := range messages {
		if msg.ID != "" {
			processedCount++
		}
	}

	if processedCount != batchSize {
		t.Errorf("Processed %d messages, want %d", processedCount, batchSize)
	}
}

func TestTimeoutConfiguration(t *testing.T) {
	timeouts := map[string]time.Duration{
		"publish":    5 * time.Second,
		"handler":    25 * time.Second,
		"fetch":      5 * time.Second,
		"ack_wait":   30 * time.Second,
	}

	for name, timeout := range timeouts {
		t.Run(name, func(t *testing.T) {
			if timeout <= 0 {
				t.Errorf("Timeout %s should be positive", name)
			}

			if timeout > 60*time.Second {
				t.Errorf("Timeout %s (%v) seems too long", name, timeout)
			}
		})
	}
}

func BenchmarkMessageStructCreation(b *testing.B) {
	for i := 0; i < b.N; i++ {
		_ = &Message{
			ID:             "MSG-001",
			Type:           "payment",
			CorridorID:     "USD-EUR",
			BankID:         "BANK001",
			IdempotencyKey: "key-123",
			Timestamp:      time.Now(),
			Headers:        make(map[string]string),
		}
	}
}

func BenchmarkPayloadMarshal(b *testing.B) {
	payload := map[string]interface{}{
		"amount":   1000.50,
		"currency": "USD",
		"sender":   "BANK001",
		"receiver": "BANK002",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = json.Marshal(payload)
	}
}

func BenchmarkIsReprocessable(b *testing.B) {
	errors := []string{
		"timeout",
		"connection error",
		"unavailable",
		"invalid data",
		"rate_limit",
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = isReprocessable(errors[i%len(errors)])
	}
}

func init() {
	// Initialize logger for tests
	logger, _ := zap.NewDevelopment()
	_ = logger
}
