package auth

import (
	"context"
	"fmt"
	"net/http"
	"strconv"
	"time"

	"github.com/redis/go-redis/v9"
)

const (
	// Rate limit key prefixes
	RateLimitKeyPrefix = "ratelimit:"
	IPRateLimitPrefix  = "ratelimit:ip:"
	UserRateLimitPrefix = "ratelimit:user:"
	EndpointRateLimitPrefix = "ratelimit:endpoint:"

	// Default rate limits
	DefaultRequestsPerMinute = 100
	DefaultBurstSize         = 20
)

var (
	ErrRateLimitExceeded = fmt.Errorf("rate limit exceeded")
)

// RateLimiter implements token bucket rate limiting with Redis
type RateLimiter struct {
	redis            *redis.Client
	requestsPerMinute int
	burstSize        int
	windowSize       time.Duration
}

// NewRateLimiter creates a new rate limiter
func NewRateLimiter(redisClient *redis.Client, requestsPerMinute, burstSize int) *RateLimiter {
	return &RateLimiter{
		redis:            redisClient,
		requestsPerMinute: requestsPerMinute,
		burstSize:        burstSize,
		windowSize:       time.Minute,
	}
}

// RateLimitConfig represents rate limit configuration
type RateLimitConfig struct {
	Enabled           bool `json:"enabled"`
	RequestsPerMinute int  `json:"requests_per_minute"`
	BurstSize         int  `json:"burst_size"`
}

// DefaultRateLimitConfig returns default configuration
func DefaultRateLimitConfig() *RateLimitConfig {
	return &RateLimitConfig{
		Enabled:           true,
		RequestsPerMinute: DefaultRequestsPerMinute,
		BurstSize:         DefaultBurstSize,
	}
}

// AllowRequest checks if request is allowed under rate limit
func (rl *RateLimiter) AllowRequest(ctx context.Context, key string) (bool, *RateLimitInfo, error) {
	now := time.Now().Unix()
	windowKey := fmt.Sprintf("%s%s:%d", RateLimitKeyPrefix, key, now/60) // 1-minute window

	// Increment counter
	pipe := rl.redis.Pipeline()
	incrCmd := pipe.Incr(ctx, windowKey)
	pipe.Expire(ctx, windowKey, rl.windowSize+time.Second) // Small buffer
	_, err := pipe.Exec(ctx)
	if err != nil {
		return false, nil, fmt.Errorf("failed to increment rate limit: %w", err)
	}

	count := incrCmd.Val()

	// Check limit
	limit := int64(rl.requestsPerMinute)
	remaining := limit - count
	if remaining < 0 {
		remaining = 0
	}

	resetTime := time.Unix((now/60+1)*60, 0)

	info := &RateLimitInfo{
		Limit:     limit,
		Remaining: remaining,
		ResetTime: resetTime,
		RetryAfter: time.Until(resetTime),
	}

	// Allow burst
	if count <= int64(rl.requestsPerMinute+rl.burstSize) {
		return true, info, nil
	}

	return false, info, nil
}

// RateLimitInfo contains rate limit information
type RateLimitInfo struct {
	Limit      int64         `json:"limit"`
	Remaining  int64         `json:"remaining"`
	ResetTime  time.Time     `json:"reset_time"`
	RetryAfter time.Duration `json:"retry_after"`
}

// IPRateLimitMiddleware creates middleware for IP-based rate limiting
func IPRateLimitMiddleware(limiter *RateLimiter) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			ip := getClientIP(r)
			key := fmt.Sprintf("%s%s", IPRateLimitPrefix, ip)

			allowed, info, err := limiter.AllowRequest(r.Context(), key)
			if err != nil {
				// Log error but don't block request on rate limit errors
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}

			// Set rate limit headers
			w.Header().Set("X-RateLimit-Limit", strconv.FormatInt(info.Limit, 10))
			w.Header().Set("X-RateLimit-Remaining", strconv.FormatInt(info.Remaining, 10))
			w.Header().Set("X-RateLimit-Reset", strconv.FormatInt(info.ResetTime.Unix(), 10))

			if !allowed {
				w.Header().Set("Retry-After", strconv.FormatInt(int64(info.RetryAfter.Seconds()), 10))
				writeError(w, "rate limit exceeded", http.StatusTooManyRequests)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// UserRateLimitMiddleware creates middleware for user-based rate limiting
func UserRateLimitMiddleware(limiter *RateLimiter) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Get user ID from context (set by JWT middleware)
			userID, ok := GetUserID(r)
			if !ok {
				// No user ID, skip user-based rate limiting
				next.ServeHTTP(w, r)
				return
			}

			key := fmt.Sprintf("%s%s", UserRateLimitPrefix, userID)

			allowed, info, err := limiter.AllowRequest(r.Context(), key)
			if err != nil {
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}

			// Set rate limit headers
			w.Header().Set("X-RateLimit-Limit", strconv.FormatInt(info.Limit, 10))
			w.Header().Set("X-RateLimit-Remaining", strconv.FormatInt(info.Remaining, 10))
			w.Header().Set("X-RateLimit-Reset", strconv.FormatInt(info.ResetTime.Unix(), 10))

			if !allowed {
				w.Header().Set("Retry-After", strconv.FormatInt(int64(info.RetryAfter.Seconds()), 10))
				writeError(w, "rate limit exceeded", http.StatusTooManyRequests)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// EndpointRateLimitMiddleware creates middleware for endpoint-based rate limiting
func EndpointRateLimitMiddleware(limiter *RateLimiter, endpoint string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			key := fmt.Sprintf("%s%s", EndpointRateLimitPrefix, endpoint)

			allowed, info, err := limiter.AllowRequest(r.Context(), key)
			if err != nil {
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}

			w.Header().Set("X-RateLimit-Limit", strconv.FormatInt(info.Limit, 10))
			w.Header().Set("X-RateLimit-Remaining", strconv.FormatInt(info.Remaining, 10))
			w.Header().Set("X-RateLimit-Reset", strconv.FormatInt(info.ResetTime.Unix(), 10))

			if !allowed {
				w.Header().Set("Retry-After", strconv.FormatInt(int64(info.RetryAfter.Seconds()), 10))
				writeError(w, "rate limit exceeded for this endpoint", http.StatusTooManyRequests)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// CombinedRateLimitMiddleware combines IP and user rate limiting
func CombinedRateLimitMiddleware(ipLimiter, userLimiter *RateLimiter) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Check IP rate limit
			ip := getClientIP(r)
			ipKey := fmt.Sprintf("%s%s", IPRateLimitPrefix, ip)

			allowed, info, err := ipLimiter.AllowRequest(r.Context(), ipKey)
			if err != nil {
				http.Error(w, "Internal server error", http.StatusInternalServerError)
				return
			}

			if !allowed {
				w.Header().Set("X-RateLimit-Limit", strconv.FormatInt(info.Limit, 10))
				w.Header().Set("X-RateLimit-Remaining", strconv.FormatInt(info.Remaining, 10))
				w.Header().Set("X-RateLimit-Reset", strconv.FormatInt(info.ResetTime.Unix(), 10))
				w.Header().Set("Retry-After", strconv.FormatInt(int64(info.RetryAfter.Seconds()), 10))
				writeError(w, "IP rate limit exceeded", http.StatusTooManyRequests)
				return
			}

			// Check user rate limit if authenticated
			if userID, ok := GetUserID(r); ok {
				userKey := fmt.Sprintf("%s%s", UserRateLimitPrefix, userID)

				allowed, info, err := userLimiter.AllowRequest(r.Context(), userKey)
				if err != nil {
					http.Error(w, "Internal server error", http.StatusInternalServerError)
					return
				}

				if !allowed {
					w.Header().Set("X-RateLimit-Limit", strconv.FormatInt(info.Limit, 10))
					w.Header().Set("X-RateLimit-Remaining", strconv.FormatInt(info.Remaining, 10))
					w.Header().Set("X-RateLimit-Reset", strconv.FormatInt(info.ResetTime.Unix(), 10))
					w.Header().Set("Retry-After", strconv.FormatInt(int64(info.RetryAfter.Seconds()), 10))
					writeError(w, "user rate limit exceeded", http.StatusTooManyRequests)
					return
				}
			}

			next.ServeHTTP(w, r)
		})
	}
}

// getClientIP extracts client IP from request
func getClientIP(r *http.Request) string {
	// Check X-Forwarded-For header (load balancer)
	if xff := r.Header.Get("X-Forwarded-For"); xff != "" {
		// Take first IP if multiple (comma-separated)
		for i, c := range xff {
			if c == ',' {
				return xff[:i]
			}
		}
		return xff
	}

	// Check X-Real-IP header (nginx)
	if xrip := r.Header.Get("X-Real-IP"); xrip != "" {
		return xrip
	}

	// Fallback to RemoteAddr
	return r.RemoteAddr
}

// ResetRateLimit resets rate limit for a key (admin function)
func (rl *RateLimiter) ResetRateLimit(ctx context.Context, key string) error {
	pattern := fmt.Sprintf("%s%s:*", RateLimitKeyPrefix, key)

	// Scan and delete all matching keys
	iter := rl.redis.Scan(ctx, 0, pattern, 100).Iterator()
	for iter.Next(ctx) {
		if err := rl.redis.Del(ctx, iter.Val()).Err(); err != nil {
			return fmt.Errorf("failed to delete rate limit key: %w", err)
		}
	}

	if err := iter.Err(); err != nil {
		return fmt.Errorf("failed to scan rate limit keys: %w", err)
	}

	return nil
}

// GetRateLimitInfo gets current rate limit info without incrementing
func (rl *RateLimiter) GetRateLimitInfo(ctx context.Context, key string) (*RateLimitInfo, error) {
	now := time.Now().Unix()
	windowKey := fmt.Sprintf("%s%s:%d", RateLimitKeyPrefix, key, now/60)

	count, err := rl.redis.Get(ctx, windowKey).Int64()
	if err != nil {
		if err == redis.Nil {
			count = 0
		} else {
			return nil, fmt.Errorf("failed to get rate limit: %w", err)
		}
	}

	limit := int64(rl.requestsPerMinute)
	remaining := limit - count
	if remaining < 0 {
		remaining = 0
	}

	resetTime := time.Unix((now/60+1)*60, 0)

	return &RateLimitInfo{
		Limit:     limit,
		Remaining: remaining,
		ResetTime: resetTime,
		RetryAfter: time.Until(resetTime),
	}, nil
}

// SlidingWindowRateLimiter implements sliding window rate limiting
type SlidingWindowRateLimiter struct {
	redis  *redis.Client
	limit  int
	window time.Duration
}

// NewSlidingWindowRateLimiter creates a new sliding window rate limiter
func NewSlidingWindowRateLimiter(redisClient *redis.Client, limit int, window time.Duration) *SlidingWindowRateLimiter {
	return &SlidingWindowRateLimiter{
		redis:  redisClient,
		limit:  limit,
		window: window,
	}
}

// AllowRequest checks if request is allowed (sliding window algorithm)
func (swrl *SlidingWindowRateLimiter) AllowRequest(ctx context.Context, key string) (bool, error) {
	now := time.Now()
	windowStart := now.Add(-swrl.window)

	rateLimitKey := RateLimitKeyPrefix + key

	// Use sorted set with timestamps as scores
	pipe := swrl.redis.Pipeline()

	// Remove old entries
	pipe.ZRemRangeByScore(ctx, rateLimitKey, "0", strconv.FormatInt(windowStart.UnixNano(), 10))

	// Count current entries
	countCmd := pipe.ZCard(ctx, rateLimitKey)

	// Add current request
	pipe.ZAdd(ctx, rateLimitKey, redis.Z{
		Score:  float64(now.UnixNano()),
		Member: fmt.Sprintf("%d", now.UnixNano()),
	})

	// Set expiration
	pipe.Expire(ctx, rateLimitKey, swrl.window+time.Second)

	_, err := pipe.Exec(ctx)
	if err != nil {
		return false, fmt.Errorf("failed to execute rate limit pipeline: %w", err)
	}

	count := countCmd.Val()

	return count < int64(swrl.limit), nil
}
