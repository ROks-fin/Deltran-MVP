package resilience

import (
	"context"
	"errors"
	"testing"
	"time"
)

// TestCircuitBreakerWithRetry tests circuit breaker and retry working together
func TestCircuitBreakerWithRetry(t *testing.T) {
	cb := NewCircuitBreaker(DefaultConfig("test-service"))
	retryConfig := &RetryConfig{
		MaxAttempts:  2,
		InitialDelay: 10 * time.Millisecond,
		MaxDelay:     100 * time.Millisecond,
		Multiplier:   2.0,
		Jitter:       false,
	}

	callCount := 0
	testErr := errors.New("test error")

	// Function that fails twice, then succeeds
	fn := func(ctx context.Context) error {
		callCount++
		if callCount <= 2 {
			return testErr
		}
		return nil
	}

	// Execute with retry and circuit breaker
	err := RetryContextWithCircuitBreaker(context.Background(), fn, retryConfig, cb)

	if err != nil {
		t.Errorf("Expected success after retries, got error: %v", err)
	}

	if callCount != 3 {
		t.Errorf("Expected 3 calls (1 initial + 2 retries), got %d", callCount)
	}

	// Verify circuit breaker is still closed
	if cb.State() != StateClosed {
		t.Errorf("Expected circuit breaker to be closed, got %s", cb.State())
	}
}

// TestCircuitBreakerOpensOnFailures tests that circuit breaker opens after failures
func TestCircuitBreakerOpensOnFailures(t *testing.T) {
	config := &Config{
		Name:        "test-failing-service",
		MaxRequests: 1,
		Interval:    1 * time.Second,
		Timeout:     100 * time.Millisecond,
		ReadyToTrip: func(counts Counts) bool {
			// Trip immediately if we have 3 failures
			return counts.ConsecutiveFailures >= 3
		},
	}

	cb := NewCircuitBreaker(config)
	testErr := errors.New("persistent error")

	// Execute 3 failing requests
	for i := 0; i < 3; i++ {
		_ = cb.Execute(func() error {
			return testErr
		})
	}

	// Circuit should be open now
	if cb.State() != StateOpen {
		t.Errorf("Expected circuit breaker to be open, got %s", cb.State())
	}

	// Next request should fail immediately with ErrCircuitOpen
	err := cb.Execute(func() error {
		t.Error("Function should not be executed when circuit is open")
		return nil
	})

	if !errors.Is(err, ErrCircuitOpen) {
		t.Errorf("Expected ErrCircuitOpen, got %v", err)
	}
}

// TestCircuitBreakerHalfOpen tests half-open state recovery
func TestCircuitBreakerHalfOpen(t *testing.T) {
	config := &Config{
		Name:        "test-recovery-service",
		MaxRequests: 2,
		Interval:    1 * time.Second,
		Timeout:     50 * time.Millisecond, // Short timeout for test
		ReadyToTrip: func(counts Counts) bool {
			return counts.ConsecutiveFailures >= 2
		},
	}

	cb := NewCircuitBreaker(config)
	testErr := errors.New("error")

	// Trigger open state
	for i := 0; i < 2; i++ {
		_ = cb.Execute(func() error { return testErr })
	}

	if cb.State() != StateOpen {
		t.Fatalf("Expected circuit breaker to be open")
	}

	// Wait for timeout
	time.Sleep(60 * time.Millisecond)

	// Should transition to half-open
	err := cb.Execute(func() error {
		return nil // Success
	})

	if err != nil {
		t.Errorf("Expected success in half-open, got %v", err)
	}

	// One more success should close the circuit
	err = cb.Execute(func() error {
		return nil
	})

	if err != nil {
		t.Errorf("Expected success, got %v", err)
	}

	// Should be closed now
	if cb.State() != StateClosed {
		t.Errorf("Expected circuit breaker to be closed, got %s", cb.State())
	}
}

// TestExponentialBackoff tests exponential backoff calculation
func TestExponentialBackoff(t *testing.T) {
	eb := DefaultExponentialBackoff()

	tests := []struct {
		attempt      int
		minExpected  time.Duration
		maxExpected  time.Duration
	}{
		{0, 50 * time.Millisecond, 150 * time.Millisecond},  // 100ms ±50%
		{1, 150 * time.Millisecond, 250 * time.Millisecond}, // 200ms ±50%
		{2, 300 * time.Millisecond, 500 * time.Millisecond}, // 400ms ±50%
		{10, 20 * time.Second, 40 * time.Second},            // Capped at max
	}

	for _, tt := range tests {
		delay := eb.NextDelay(tt.attempt)
		if delay < tt.minExpected || delay > tt.maxExpected {
			t.Errorf("Attempt %d: expected delay between %v and %v, got %v",
				tt.attempt, tt.minExpected, tt.maxExpected, delay)
		}
	}
}

// TestLinearBackoff tests linear backoff calculation
func TestLinearBackoff(t *testing.T) {
	lb := &LinearBackoff{
		InitialInterval: 100 * time.Millisecond,
		MaxInterval:     5 * time.Second,
		Increment:       200 * time.Millisecond,
		MaxRetries:      5,
		Jitter:          false, // Disable jitter for predictable testing
	}

	tests := []struct {
		attempt  int
		expected time.Duration
	}{
		{0, 100 * time.Millisecond},  // Initial
		{1, 300 * time.Millisecond},  // Initial + 1*increment
		{2, 500 * time.Millisecond},  // Initial + 2*increment
		{25, 5 * time.Second},        // Capped at max (100ms + 200ms*25 = 5100ms -> capped to 5000ms)
		{100, 5 * time.Second},       // Still capped at max
	}

	for _, tt := range tests {
		delay := lb.NextDelay(tt.attempt)
		if delay != tt.expected {
			t.Errorf("Attempt %d: expected %v, got %v",
				tt.attempt, tt.expected, delay)
		}
	}
}

// TestConstantBackoff tests constant backoff
func TestConstantBackoff(t *testing.T) {
	cb := DefaultConstantBackoff()
	cb.Jitter = false

	expected := 1 * time.Second

	for i := 0; i < 5; i++ {
		delay := cb.NextDelay(i)
		if delay != expected {
			t.Errorf("Attempt %d: expected %v, got %v", i, expected, delay)
		}
	}
}

// TestRetryWithContextCancellation tests retry with context cancellation
func TestRetryWithContextCancellation(t *testing.T) {
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	config := &RetryConfig{
		MaxAttempts:  10,
		InitialDelay: 100 * time.Millisecond,
		MaxDelay:     1 * time.Second,
		Multiplier:   2.0,
		Jitter:       false,
	}

	callCount := 0
	testErr := errors.New("test error")

	// Cancel after first attempt
	go func() {
		time.Sleep(50 * time.Millisecond)
		cancel()
	}()

	err := RetryContext(ctx, func(ctx context.Context) error {
		callCount++
		// Check context before returning error
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
			return testErr
		}
	}, config)

	if !errors.Is(err, context.Canceled) {
		t.Errorf("Expected context.Canceled, got %v", err)
	}

	if callCount > 2 {
		t.Errorf("Expected at most 2 calls before cancellation, got %d", callCount)
	}
}

// TestCircuitBreakerManager tests circuit breaker manager
func TestCircuitBreakerManager(t *testing.T) {
	manager := NewCircuitBreakerManager()

	// Get or create circuit breaker
	cb1 := manager.Get("service1", nil)
	if cb1 == nil {
		t.Fatal("Expected circuit breaker, got nil")
	}

	// Get same circuit breaker
	cb2 := manager.Get("service1", nil)
	if cb1 != cb2 {
		t.Error("Expected same circuit breaker instance")
	}

	// Get different circuit breaker
	cb3 := manager.Get("service2", nil)
	if cb1 == cb3 {
		t.Error("Expected different circuit breaker instances")
	}

	// Check GetAll
	all := manager.GetAll()
	if len(all) != 2 {
		t.Errorf("Expected 2 circuit breakers, got %d", len(all))
	}

	// Reset one
	_ = cb1.Execute(func() error { return errors.New("error") })
	counts1 := cb1.Counts()
	if counts1.TotalFailures == 0 {
		t.Error("Expected failure count > 0")
	}

	manager.Reset("service1")
	counts1 = cb1.Counts()
	if counts1.TotalFailures != 0 {
		t.Error("Expected failure count to be reset to 0")
	}

	// ResetAll
	_ = cb3.Execute(func() error { return errors.New("error") })
	manager.ResetAll()

	counts3 := cb3.Counts()
	if counts3.TotalFailures != 0 {
		t.Error("Expected all circuit breakers to be reset")
	}
}

// TestIdempotencyKeyGeneration tests idempotency key generation
func TestIdempotencyKeyGeneration(t *testing.T) {
	// Same inputs should produce same key
	key1 := GenerateKey("prefix", "data1", "data2")
	key2 := GenerateKey("prefix", "data1", "data2")

	if key1 != key2 {
		t.Error("Same inputs should produce same key")
	}

	// Different inputs should produce different keys
	key3 := GenerateKey("prefix", "data1", "data3")
	if key1 == key3 {
		t.Error("Different inputs should produce different keys")
	}

	// Test payment key generation
	paymentKey := GeneratePaymentKey("BANKGB2L", "BANKUS33", "1000.00", "USD", "REF123")
	if len(paymentKey) == 0 {
		t.Error("Payment key should not be empty")
	}
}

// TestRetryableErrors tests selective retry based on error type
func TestRetryableErrors(t *testing.T) {
	retryableErr := errors.New("retryable error")
	nonRetryableErr := errors.New("non-retryable error")

	config := &RetryConfig{
		MaxAttempts:     2,
		InitialDelay:    10 * time.Millisecond,
		MaxDelay:        100 * time.Millisecond,
		Multiplier:      2.0,
		RetryableErrors: []error{retryableErr},
	}

	// Non-retryable error should not be retried
	callCount := 0
	err := Retry(func() error {
		callCount++
		return nonRetryableErr
	}, config)

	if !errors.Is(err, nonRetryableErr) {
		t.Errorf("Expected non-retryable error, got %v", err)
	}

	if callCount != 1 {
		t.Errorf("Expected 1 call (no retries), got %d", callCount)
	}

	// Retryable error should be retried
	callCount = 0
	err = Retry(func() error {
		callCount++
		if callCount <= 2 {
			return retryableErr
		}
		return nil
	}, config)

	if err != nil {
		t.Errorf("Expected success after retries, got %v", err)
	}

	if callCount != 3 {
		t.Errorf("Expected 3 calls (1 initial + 2 retries), got %d", callCount)
	}
}

// BenchmarkCircuitBreakerExecute benchmarks circuit breaker execution
func BenchmarkCircuitBreakerExecute(b *testing.B) {
	cb := NewCircuitBreaker(DefaultConfig("benchmark-service"))

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = cb.Execute(func() error {
			return nil
		})
	}
}

// BenchmarkRetryWithBackoff benchmarks retry with exponential backoff
func BenchmarkRetryWithBackoff(b *testing.B) {
	config := &RetryConfig{
		MaxAttempts:  3,
		InitialDelay: 1 * time.Millisecond,
		MaxDelay:     10 * time.Millisecond,
		Multiplier:   2.0,
		Jitter:       true,
	}

	callCount := 0
	fn := func() error {
		callCount++
		if callCount%2 == 0 {
			return errors.New("error")
		}
		return nil
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = Retry(fn, config)
	}
}

// BenchmarkIdempotencyKeyGeneration benchmarks key generation
func BenchmarkIdempotencyKeyGeneration(b *testing.B) {
	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = GeneratePaymentKey("BANKGB2L", "BANKUS33", "1000.00", "USD", "REF123")
	}
}
