package middleware

import (
	"net/http"
	"sync"
	"time"

	"golang.org/x/time/rate"
)

// RateLimiter provides rate limiting per bank
type RateLimiter struct {
	limiters map[string]*rate.Limiter
	mu       sync.RWMutex
	rps      int
	burst    int
}

// NewRateLimiter creates a new rate limiter
func NewRateLimiter(requestsPerMinute, burst int) *RateLimiter {
	return &RateLimiter{
		limiters: make(map[string]*rate.Limiter),
		rps:      requestsPerMinute,
		burst:    burst,
	}
}

// getLimiter gets or creates a rate limiter for a bank
func (rl *RateLimiter) getLimiter(bankID string) *rate.Limiter {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	limiter, exists := rl.limiters[bankID]
	if !exists {
		// Create new limiter for this bank
		limiter = rate.NewLimiter(rate.Every(time.Minute/time.Duration(rl.rps)), rl.burst)
		rl.limiters[bankID] = limiter
	}

	return limiter
}

// Middleware returns the rate limiting middleware handler
func (rl *RateLimiter) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Skip rate limiting for health check
		if r.URL.Path == "/health" {
			next.ServeHTTP(w, r)
			return
		}

		// Get bank ID from context (set by auth middleware)
		bankID := GetBankIDFromContext(r.Context())
		if bankID == "" {
			// If no bank ID, use IP address as fallback
			bankID = r.RemoteAddr
		}

		// Get limiter for this bank
		limiter := rl.getLimiter(bankID)

		// Check if request is allowed
		if !limiter.Allow() {
			http.Error(w, "Rate limit exceeded. Please try again later.", http.StatusTooManyRequests)
			return
		}

		// Continue with request
		next.ServeHTTP(w, r)
	})
}

// CleanupOldLimiters removes inactive limiters to prevent memory leak
func (rl *RateLimiter) CleanupOldLimiters() {
	ticker := time.NewTicker(1 * time.Hour)
	defer ticker.Stop()

	for range ticker.C {
		rl.mu.Lock()
		// In production, track last access time and remove old entries
		// For MVP, we keep all limiters
		rl.mu.Unlock()
	}
}
