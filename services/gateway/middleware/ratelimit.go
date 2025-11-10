package middleware

import (
	"encoding/json"
	"fmt"
	"net/http"
	"sync"
	"time"
)

// RateLimiter implements token bucket rate limiting
type RateLimiter struct {
	visitors map[string]*Visitor
	mu       sync.RWMutex
	rate     int           // requests per window
	window   time.Duration // time window
}

// Visitor tracks rate limit state for a visitor
type Visitor struct {
	lastSeen time.Time
	tokens   int
}

// NewRateLimiter creates a new rate limiter
func NewRateLimiter(rate int, window time.Duration) *RateLimiter {
	rl := &RateLimiter{
		visitors: make(map[string]*Visitor),
		rate:     rate,
		window:   window,
	}

	// Start cleanup goroutine
	go rl.cleanupVisitors()

	return rl
}

// cleanupVisitors removes stale visitors
func (rl *RateLimiter) cleanupVisitors() {
	ticker := time.NewTicker(time.Minute)
	defer ticker.Stop()

	for range ticker.C {
		rl.mu.Lock()
		for ip, visitor := range rl.visitors {
			if time.Since(visitor.lastSeen) > rl.window*2 {
				delete(rl.visitors, ip)
			}
		}
		rl.mu.Unlock()
	}
}

// getVisitor gets or creates a visitor
func (rl *RateLimiter) getVisitor(ip string) *Visitor {
	rl.mu.Lock()
	defer rl.mu.Unlock()

	visitor, exists := rl.visitors[ip]
	if !exists {
		visitor = &Visitor{
			tokens:   rl.rate,
			lastSeen: time.Now(),
		}
		rl.visitors[ip] = visitor
		return visitor
	}

	// Reset tokens if window has passed
	if time.Since(visitor.lastSeen) > rl.window {
		visitor.tokens = rl.rate
		visitor.lastSeen = time.Now()
	}

	return visitor
}

// Middleware returns the rate limiting middleware
func (rl *RateLimiter) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Get client IP
		ip := getClientIP(r)
		visitor := rl.getVisitor(ip)

		// Check if rate limit exceeded
		if visitor.tokens <= 0 {
			w.Header().Set("Content-Type", "application/json")
			w.Header().Set("X-RateLimit-Limit", fmt.Sprintf("%d", rl.rate))
			w.Header().Set("X-RateLimit-Remaining", "0")
			w.Header().Set("X-RateLimit-Reset", fmt.Sprintf("%d", time.Now().Add(rl.window).Unix()))
			w.Header().Set("Retry-After", fmt.Sprintf("%d", int(rl.window.Seconds())))

			w.WriteHeader(http.StatusTooManyRequests)
			json.NewEncoder(w).Encode(map[string]interface{}{
				"error": "Rate limit exceeded",
				"retry_after": rl.window.Seconds(),
				"limit": rl.rate,
			})
			return
		}

		// Consume a token
		visitor.tokens--
		visitor.lastSeen = time.Now()

		// Add rate limit headers
		w.Header().Set("X-RateLimit-Limit", fmt.Sprintf("%d", rl.rate))
		w.Header().Set("X-RateLimit-Remaining", fmt.Sprintf("%d", visitor.tokens))
		w.Header().Set("X-RateLimit-Reset", fmt.Sprintf("%d", time.Now().Add(rl.window).Unix()))

		next.ServeHTTP(w, r)
	})
}

// TieredRateLimiter implements tier-based rate limiting
type TieredRateLimiter struct {
	limiters map[string]*RateLimiter
	mu       sync.RWMutex
}

// NewTieredRateLimiter creates a new tiered rate limiter
func NewTieredRateLimiter() *TieredRateLimiter {
	return &TieredRateLimiter{
		limiters: map[string]*RateLimiter{
			"anonymous": NewRateLimiter(10, time.Minute),     // 10 req/min
			"basic":     NewRateLimiter(100, time.Minute),    // 100 req/min
			"premium":   NewRateLimiter(1000, time.Minute),   // 1000 req/min
			"admin":     NewRateLimiter(10000, time.Minute),  // 10000 req/min
		},
	}
}

// Middleware returns the tiered rate limiting middleware
func (trl *TieredRateLimiter) Middleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Determine user tier from role
		tier := "anonymous"
		role := r.Header.Get("X-User-Role")

		switch role {
		case "admin":
			tier = "admin"
		case "premium":
			tier = "premium"
		case "user", "basic":
			tier = "basic"
		default:
			tier = "anonymous"
		}

		// Get limiter for tier
		trl.mu.RLock()
		limiter, exists := trl.limiters[tier]
		trl.mu.RUnlock()

		if !exists {
			limiter = trl.limiters["anonymous"]
		}

		// Get client IP
		ip := getClientIP(r)
		visitor := limiter.getVisitor(ip)

		// Check if rate limit exceeded
		if visitor.tokens <= 0 {
			w.Header().Set("Content-Type", "application/json")
			w.Header().Set("X-RateLimit-Tier", tier)
			w.Header().Set("X-RateLimit-Limit", fmt.Sprintf("%d", limiter.rate))
			w.Header().Set("X-RateLimit-Remaining", "0")
			w.Header().Set("X-RateLimit-Reset", fmt.Sprintf("%d", time.Now().Add(limiter.window).Unix()))
			w.Header().Set("Retry-After", fmt.Sprintf("%d", int(limiter.window.Seconds())))

			w.WriteHeader(http.StatusTooManyRequests)
			json.NewEncoder(w).Encode(map[string]interface{}{
				"error": "Rate limit exceeded",
				"tier":  tier,
				"retry_after": limiter.window.Seconds(),
				"limit": limiter.rate,
			})
			return
		}

		// Consume a token
		visitor.tokens--
		visitor.lastSeen = time.Now()

		// Add rate limit headers
		w.Header().Set("X-RateLimit-Tier", tier)
		w.Header().Set("X-RateLimit-Limit", fmt.Sprintf("%d", limiter.rate))
		w.Header().Set("X-RateLimit-Remaining", fmt.Sprintf("%d", visitor.tokens))
		w.Header().Set("X-RateLimit-Reset", fmt.Sprintf("%d", time.Now().Add(limiter.window).Unix()))

		next.ServeHTTP(w, r)
	})
}

// getClientIP extracts the client IP from the request
func getClientIP(r *http.Request) string {
	// Check X-Forwarded-For header
	xff := r.Header.Get("X-Forwarded-For")
	if xff != "" {
		// Take the first IP if multiple
		if idx := len(xff); idx > 0 {
			if commaIdx := 0; commaIdx < idx {
				for i, c := range xff {
					if c == ',' {
						return xff[:i]
					}
				}
			}
		}
		return xff
	}

	// Check X-Real-IP header
	xri := r.Header.Get("X-Real-IP")
	if xri != "" {
		return xri
	}

	// Fall back to RemoteAddr
	return r.RemoteAddr
}
