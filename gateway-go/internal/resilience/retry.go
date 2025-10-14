package resilience

import (
	"context"
	"errors"
	"fmt"
	"math"
	"math/rand"
	"time"
)

var (
	ErrMaxRetriesExceeded = errors.New("maximum retries exceeded")
)

// RetryConfig holds retry configuration
type RetryConfig struct {
	// MaxAttempts is the maximum number of retry attempts (0 = no retries, 1 = original + 1 retry)
	MaxAttempts int

	// InitialDelay is the initial delay before first retry
	InitialDelay time.Duration

	// MaxDelay is the maximum delay between retries
	MaxDelay time.Duration

	// Multiplier is the backoff multiplier (typically 2 for exponential)
	Multiplier float64

	// Jitter adds randomness to prevent thundering herd
	Jitter bool

	// RetryableErrors is a list of errors that should trigger retry
	// If nil, all errors trigger retry
	RetryableErrors []error

	// OnRetry is called before each retry attempt
	OnRetry func(attempt int, err error, delay time.Duration)
}

// DefaultRetryConfig returns default retry configuration
func DefaultRetryConfig() *RetryConfig {
	return &RetryConfig{
		MaxAttempts:  3,
		InitialDelay: 100 * time.Millisecond,
		MaxDelay:     10 * time.Second,
		Multiplier:   2.0,
		Jitter:       true,
		OnRetry: func(attempt int, err error, delay time.Duration) {
			fmt.Printf("Retry attempt %d after error: %v (delay: %v)\n", attempt, err, delay)
		},
	}
}

// Retry executes a function with retries
func Retry(fn func() error, config *RetryConfig) error {
	return RetryContext(context.Background(), func(ctx context.Context) error {
		return fn()
	}, config)
}

// RetryContext executes a function with retries and context
func RetryContext(ctx context.Context, fn func(context.Context) error, config *RetryConfig) error {
	if config == nil {
		config = DefaultRetryConfig()
	}

	var lastErr error

	for attempt := 0; attempt <= config.MaxAttempts; attempt++ {
		// Check context before attempting
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		// Execute function
		err := fn(ctx)

		// Success
		if err == nil {
			return nil
		}

		lastErr = err

		// Check if error is retryable
		if !isRetryable(err, config.RetryableErrors) {
			return err
		}

		// No more retries
		if attempt >= config.MaxAttempts {
			break
		}

		// Calculate delay
		delay := calculateDelay(attempt, config)

		// Call OnRetry callback
		if config.OnRetry != nil {
			config.OnRetry(attempt+1, err, delay)
		}

		// Wait before retry
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(delay):
		}
	}

	return fmt.Errorf("%w: %v", ErrMaxRetriesExceeded, lastErr)
}

// calculateDelay calculates delay for exponential backoff
func calculateDelay(attempt int, config *RetryConfig) time.Duration {
	// Exponential backoff: delay = initialDelay * (multiplier ^ attempt)
	delay := float64(config.InitialDelay) * math.Pow(config.Multiplier, float64(attempt))

	// Cap at max delay
	if delay > float64(config.MaxDelay) {
		delay = float64(config.MaxDelay)
	}

	// Add jitter if enabled
	if config.Jitter {
		delay = addJitter(delay)
	}

	return time.Duration(delay)
}

// addJitter adds random jitter to delay (±25%)
func addJitter(delay float64) float64 {
	jitter := delay * 0.25 // 25% jitter
	return delay + (rand.Float64()*2-1)*jitter
}

// isRetryable checks if error should trigger retry
func isRetryable(err error, retryableErrors []error) bool {
	// If no specific errors defined, all errors are retryable
	if len(retryableErrors) == 0 {
		return true
	}

	// Check if error matches any retryable error
	for _, retryErr := range retryableErrors {
		if errors.Is(err, retryErr) {
			return true
		}
	}

	return false
}

// RetryPolicy defines retry policy
type RetryPolicy struct {
	config *RetryConfig
}

// NewRetryPolicy creates a new retry policy
func NewRetryPolicy(config *RetryConfig) *RetryPolicy {
	if config == nil {
		config = DefaultRetryConfig()
	}
	return &RetryPolicy{config: config}
}

// Execute executes function with retry policy
func (p *RetryPolicy) Execute(fn func() error) error {
	return Retry(fn, p.config)
}

// ExecuteContext executes function with retry policy and context
func (p *RetryPolicy) ExecuteContext(ctx context.Context, fn func(context.Context) error) error {
	return RetryContext(ctx, fn, p.config)
}

// RetryWithCircuitBreaker combines retry with circuit breaker
func RetryWithCircuitBreaker(fn func() error, retryConfig *RetryConfig, cb *CircuitBreaker) error {
	return RetryContextWithCircuitBreaker(context.Background(), func(ctx context.Context) error {
		return fn()
	}, retryConfig, cb)
}

// RetryContextWithCircuitBreaker combines retry with circuit breaker and context
func RetryContextWithCircuitBreaker(ctx context.Context, fn func(context.Context) error, retryConfig *RetryConfig, cb *CircuitBreaker) error {
	if retryConfig == nil {
		retryConfig = DefaultRetryConfig()
	}

	var lastErr error

	for attempt := 0; attempt <= retryConfig.MaxAttempts; attempt++ {
		// Check context
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		// Execute with circuit breaker
		err := cb.ExecuteContext(ctx, fn)

		// Success
		if err == nil {
			return nil
		}

		lastErr = err

		// Circuit breaker open - don't retry
		if errors.Is(err, ErrCircuitOpen) {
			return err
		}

		// Check if retryable
		if !isRetryable(err, retryConfig.RetryableErrors) {
			return err
		}

		// No more retries
		if attempt >= retryConfig.MaxAttempts {
			break
		}

		// Calculate delay
		delay := calculateDelay(attempt, retryConfig)

		// Call callback
		if retryConfig.OnRetry != nil {
			retryConfig.OnRetry(attempt+1, err, delay)
		}

		// Wait
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(delay):
		}
	}

	return fmt.Errorf("%w: %v", ErrMaxRetriesExceeded, lastErr)
}

// ExponentialBackoff implements exponential backoff strategy
type ExponentialBackoff struct {
	InitialInterval time.Duration
	MaxInterval     time.Duration
	Multiplier      float64
	MaxRetries      int
	Jitter          bool
}

// DefaultExponentialBackoff returns default exponential backoff configuration
func DefaultExponentialBackoff() *ExponentialBackoff {
	return &ExponentialBackoff{
		InitialInterval: 100 * time.Millisecond,
		MaxInterval:     30 * time.Second,
		Multiplier:      2.0,
		MaxRetries:      5,
		Jitter:          true,
	}
}

// NextDelay calculates next delay for given attempt
func (eb *ExponentialBackoff) NextDelay(attempt int) time.Duration {
	if attempt < 0 {
		attempt = 0
	}

	delay := float64(eb.InitialInterval) * math.Pow(eb.Multiplier, float64(attempt))

	if delay > float64(eb.MaxInterval) {
		delay = float64(eb.MaxInterval)
	}

	if eb.Jitter {
		delay = addJitter(delay)
	}

	return time.Duration(delay)
}

// ShouldRetry checks if should retry based on attempt count
func (eb *ExponentialBackoff) ShouldRetry(attempt int) bool {
	return attempt < eb.MaxRetries
}

// LinearBackoff implements linear backoff strategy
type LinearBackoff struct {
	InitialInterval time.Duration
	MaxInterval     time.Duration
	Increment       time.Duration
	MaxRetries      int
	Jitter          bool
}

// DefaultLinearBackoff returns default linear backoff configuration
func DefaultLinearBackoff() *LinearBackoff {
	return &LinearBackoff{
		InitialInterval: 100 * time.Millisecond,
		MaxInterval:     5 * time.Second,
		Increment:       200 * time.Millisecond,
		MaxRetries:      5,
		Jitter:          true,
	}
}

// NextDelay calculates next delay for given attempt (linear)
func (lb *LinearBackoff) NextDelay(attempt int) time.Duration {
	if attempt < 0 {
		attempt = 0
	}

	delay := float64(lb.InitialInterval) + float64(lb.Increment)*float64(attempt)

	if delay > float64(lb.MaxInterval) {
		delay = float64(lb.MaxInterval)
	}

	if lb.Jitter {
		delay = addJitter(delay)
	}

	return time.Duration(delay)
}

// ShouldRetry checks if should retry based on attempt count
func (lb *LinearBackoff) ShouldRetry(attempt int) bool {
	return attempt < lb.MaxRetries
}

// ConstantBackoff implements constant backoff strategy
type ConstantBackoff struct {
	Interval   time.Duration
	MaxRetries int
	Jitter     bool
}

// DefaultConstantBackoff returns default constant backoff configuration
func DefaultConstantBackoff() *ConstantBackoff {
	return &ConstantBackoff{
		Interval:   1 * time.Second,
		MaxRetries: 3,
		Jitter:     false,
	}
}

// NextDelay returns constant delay
func (cb *ConstantBackoff) NextDelay(attempt int) time.Duration {
	delay := float64(cb.Interval)

	if cb.Jitter {
		delay = addJitter(delay)
	}

	return time.Duration(delay)
}

// ShouldRetry checks if should retry based on attempt count
func (cb *ConstantBackoff) ShouldRetry(attempt int) bool {
	return attempt < cb.MaxRetries
}

// BackoffStrategy interface for different backoff strategies
type BackoffStrategy interface {
	NextDelay(attempt int) time.Duration
	ShouldRetry(attempt int) bool
}

// RetryWithBackoff executes function with custom backoff strategy
func RetryWithBackoff(ctx context.Context, fn func(context.Context) error, strategy BackoffStrategy, onRetry func(attempt int, err error, delay time.Duration)) error {
	var lastErr error
	attempt := 0

	for {
		// Check context
		select {
		case <-ctx.Done():
			return ctx.Err()
		default:
		}

		// Execute function
		err := fn(ctx)

		// Success
		if err == nil {
			return nil
		}

		lastErr = err

		// Check if should retry
		if !strategy.ShouldRetry(attempt) {
			break
		}

		// Calculate delay
		delay := strategy.NextDelay(attempt)

		// Call callback
		if onRetry != nil {
			onRetry(attempt+1, err, delay)
		}

		// Wait
		select {
		case <-ctx.Done():
			return ctx.Err()
		case <-time.After(delay):
		}

		attempt++
	}

	return fmt.Errorf("%w: %v", ErrMaxRetriesExceeded, lastErr)
}
