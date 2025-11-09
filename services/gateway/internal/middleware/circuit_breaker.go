package middleware

import (
	"log"
	"net/http"

	"github.com/afex/hystrix-go/hystrix"
)

// CircuitBreakerMiddleware provides circuit breaker functionality
type CircuitBreakerMiddleware struct {
	commandName string
}

// NewCircuitBreakerMiddleware creates a new circuit breaker middleware
func NewCircuitBreakerMiddleware(commandName string, timeout, maxConcurrent, errorThreshold int, sleepWindow int) *CircuitBreakerMiddleware {
	// Configure hystrix for this command
	hystrix.ConfigureCommand(commandName, hystrix.CommandConfig{
		Timeout:                timeout,
		MaxConcurrentRequests:  maxConcurrent,
		RequestVolumeThreshold: 20,
		SleepWindow:            sleepWindow,
		ErrorPercentThreshold:  errorThreshold,
	})

	return &CircuitBreakerMiddleware{
		commandName: commandName,
	}
}

// Middleware returns the circuit breaker middleware handler
func (cb *CircuitBreakerMiddleware) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Skip circuit breaker for health check
		if r.URL.Path == "/health" {
			next.ServeHTTP(w, r)
			return
		}

		// Execute request through circuit breaker
		err := hystrix.Do(cb.commandName, func() error {
			// Wrap the response writer to capture status code
			wrapper := &responseWrapper{ResponseWriter: w, statusCode: http.StatusOK}
			next.ServeHTTP(wrapper, r)

			// Consider 5xx errors as failures
			if wrapper.statusCode >= 500 {
				return http.ErrAbortHandler
			}
			return nil
		}, func(err error) error {
			// Fallback - circuit breaker is open
			log.Printf("Circuit breaker open for %s: %v", cb.commandName, err)
			http.Error(w, "Service temporarily unavailable. Please try again later.", http.StatusServiceUnavailable)
			return nil
		})

		if err != nil {
			log.Printf("Circuit breaker error for %s: %v", cb.commandName, err)
		}
	})
}

// responseWrapper wraps http.ResponseWriter to capture status code
type responseWrapper struct {
	http.ResponseWriter
	statusCode int
}

// WriteHeader captures the status code
func (rw *responseWrapper) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}

// Write ensures WriteHeader is called
func (rw *responseWrapper) Write(b []byte) (int, error) {
	if rw.statusCode == 0 {
		rw.statusCode = http.StatusOK
	}
	return rw.ResponseWriter.Write(b)
}
