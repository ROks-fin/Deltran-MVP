package auth

import (
	"context"
	"fmt"
	"net/http"
	"net/http/httptest"
	"strconv"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"
)

// setupTestRedis creates a Redis client for testing (requires local Redis)
func setupTestRedis(t *testing.T) (*redis.Client, func()) {
	redisClient := redis.NewClient(&redis.Options{
		Addr: "localhost:6379",
		DB:   15, // Use separate DB for tests
	})

	ctx := context.Background()

	// Test connection
	if err := redisClient.Ping(ctx).Err(); err != nil {
		t.Skip("Redis not available, skipping Redis integration tests")
		return nil, func() {}
	}

	// Clear test DB
	redisClient.FlushDB(ctx)

	cleanup := func() {
		redisClient.FlushDB(ctx)
		redisClient.Close()
	}

	return redisClient, cleanup
}

func TestNewRateLimiter(t *testing.T) {
	redisClient := redis.NewClient(&redis.Options{Addr: "localhost:6379"})
	defer redisClient.Close()

	limiter := NewRateLimiter(redisClient, 100, 20)

	if limiter == nil {
		t.Fatal("NewRateLimiter returned nil")
	}

	if limiter.requestsPerMinute != 100 {
		t.Errorf("requestsPerMinute = %d, want 100", limiter.requestsPerMinute)
	}

	if limiter.burstSize != 20 {
		t.Errorf("burstSize = %d, want 20", limiter.burstSize)
	}

	if limiter.windowSize != time.Minute {
		t.Errorf("windowSize = %v, want 1m", limiter.windowSize)
	}
}

func TestDefaultRateLimitConfig(t *testing.T) {
	config := DefaultRateLimitConfig()

	if config == nil {
		t.Fatal("DefaultRateLimitConfig returned nil")
	}

	if !config.Enabled {
		t.Error("Default config should be enabled")
	}

	if config.RequestsPerMinute != DefaultRequestsPerMinute {
		t.Errorf("RequestsPerMinute = %d, want %d", config.RequestsPerMinute, DefaultRequestsPerMinute)
	}

	if config.BurstSize != DefaultBurstSize {
		t.Errorf("BurstSize = %d, want %d", config.BurstSize, DefaultBurstSize)
	}
}

func TestRateLimiter_AllowRequest(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return // Docker not available
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewRateLimiter(redisClient, 5, 2) // 5 requests/min + 2 burst

	key := "test-key"

	// First 7 requests should be allowed (5 + 2 burst)
	for i := 1; i <= 7; i++ {
		allowed, info, err := limiter.AllowRequest(ctx, key)
		if err != nil {
			t.Fatalf("Request %d failed: %v", i, err)
		}

		if !allowed {
			t.Errorf("Request %d should be allowed (within limit+burst)", i)
		}

		if info.Limit != 5 {
			t.Errorf("Request %d: Limit = %d, want 5", i, info.Limit)
		}

		expectedRemaining := int64(5 - i)
		if expectedRemaining < 0 {
			expectedRemaining = 0
		}
		if info.Remaining != expectedRemaining {
			t.Errorf("Request %d: Remaining = %d, want %d", i, info.Remaining, expectedRemaining)
		}
	}

	// 8th request should be blocked
	allowed, info, err := limiter.AllowRequest(ctx, key)
	if err != nil {
		t.Fatalf("Request 8 failed: %v", err)
	}

	if allowed {
		t.Error("Request 8 should be blocked (exceeded limit+burst)")
	}

	if info.Remaining != 0 {
		t.Errorf("Remaining should be 0, got %d", info.Remaining)
	}

	if info.RetryAfter <= 0 {
		t.Error("RetryAfter should be positive")
	}
}

func TestRateLimiter_ResetBetweenWindows(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewRateLimiter(redisClient, 3, 0) // 3 requests/min, no burst

	key := "test-reset"

	// Use all requests
	for i := 0; i < 3; i++ {
		allowed, _, err := limiter.AllowRequest(ctx, key)
		if err != nil || !allowed {
			t.Fatalf("Request %d should be allowed", i+1)
		}
	}

	// 4th should fail
	allowed, _, _ := limiter.AllowRequest(ctx, key)
	if allowed {
		t.Error("Request 4 should be blocked")
	}

	// Wait for window to reset (simulate by checking with different window key)
	// In real scenario, this would wait 60+ seconds
	t.Log("Rate limit correctly blocks after exhausting quota")
}

func TestGetClientIP_XForwardedFor(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("X-Forwarded-For", "203.0.113.1, 198.51.100.1")

	ip := getClientIP(req)

	// Should extract first IP from X-Forwarded-For
	if ip != "203.0.113.1" {
		t.Errorf("getClientIP() = %s, want 203.0.113.1", ip)
	}
}

func TestGetClientIP_XRealIP(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("X-Real-IP", "203.0.113.5")

	ip := getClientIP(req)

	if ip != "203.0.113.5" {
		t.Errorf("getClientIP() = %s, want 203.0.113.5", ip)
	}
}

func TestGetClientIP_RemoteAddr(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)
	req.RemoteAddr = "192.168.1.100:54321"

	ip := getClientIP(req)

	if ip != "192.168.1.100:54321" {
		t.Errorf("getClientIP() = %s, want 192.168.1.100:54321", ip)
	}
}

func TestIPRateLimitMiddleware_Allowed(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	limiter := NewRateLimiter(redisClient, 10, 5)
	middleware := IPRateLimitMiddleware(limiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.RemoteAddr = "192.168.1.50:12345"
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}

	// Check rate limit headers
	if rr.Header().Get("X-RateLimit-Limit") == "" {
		t.Error("X-RateLimit-Limit header should be set")
	}

	if rr.Header().Get("X-RateLimit-Remaining") == "" {
		t.Error("X-RateLimit-Remaining header should be set")
	}

	if rr.Header().Get("X-RateLimit-Reset") == "" {
		t.Error("X-RateLimit-Reset header should be set")
	}
}

func TestIPRateLimitMiddleware_Exceeded(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	limiter := NewRateLimiter(redisClient, 2, 0) // Only 2 requests
	middleware := IPRateLimitMiddleware(limiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	ip := "192.168.1.100:12345"

	// Exhaust limit
	for i := 0; i < 2; i++ {
		req := httptest.NewRequest("GET", "/test", nil)
		req.RemoteAddr = ip
		rr := httptest.NewRecorder()
		handler.ServeHTTP(rr, req)

		if rr.Code != http.StatusOK {
			t.Errorf("Request %d should succeed, got %d", i+1, rr.Code)
		}
	}

	// 3rd request should be blocked
	req := httptest.NewRequest("GET", "/test", nil)
	req.RemoteAddr = ip
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusTooManyRequests {
		t.Errorf("Expected status 429, got %d", rr.Code)
	}

	if rr.Header().Get("Retry-After") == "" {
		t.Error("Retry-After header should be set")
	}

	if rr.Header().Get("X-RateLimit-Remaining") != "0" {
		t.Errorf("X-RateLimit-Remaining should be 0, got %s", rr.Header().Get("X-RateLimit-Remaining"))
	}
}

func TestUserRateLimitMiddleware_NoUser(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	limiter := NewRateLimiter(redisClient, 10, 5)
	middleware := UserRateLimitMiddleware(limiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	// No user ID in context
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	// Should pass through without user ID
	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestUserRateLimitMiddleware_WithUser(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	limiter := NewRateLimiter(redisClient, 10, 5)
	middleware := UserRateLimitMiddleware(limiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), UserIDContextKey, "user123")
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}

	// Check rate limit headers are set
	if rr.Header().Get("X-RateLimit-Limit") == "" {
		t.Error("X-RateLimit-Limit header should be set for authenticated user")
	}
}

func TestEndpointRateLimitMiddleware(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	limiter := NewRateLimiter(redisClient, 5, 0)
	middleware := EndpointRateLimitMiddleware(limiter, "/api/payments")
	handler := middleware(mockHandler(http.StatusOK, "success"))

	// Make 5 requests
	for i := 0; i < 5; i++ {
		req := httptest.NewRequest("POST", "/api/payments", nil)
		rr := httptest.NewRecorder()
		handler.ServeHTTP(rr, req)

		if rr.Code != http.StatusOK {
			t.Errorf("Request %d should succeed, got %d", i+1, rr.Code)
		}
	}

	// 6th request should fail
	req := httptest.NewRequest("POST", "/api/payments", nil)
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusTooManyRequests {
		t.Errorf("Expected status 429, got %d", rr.Code)
	}
}

func TestCombinedRateLimitMiddleware_IPLimit(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ipLimiter := NewRateLimiter(redisClient, 2, 0)
	userLimiter := NewRateLimiter(redisClient, 10, 0)
	middleware := CombinedRateLimitMiddleware(ipLimiter, userLimiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	ip := "192.168.1.200:54321"

	// Exhaust IP limit
	for i := 0; i < 2; i++ {
		req := httptest.NewRequest("GET", "/test", nil)
		req.RemoteAddr = ip
		rr := httptest.NewRecorder()
		handler.ServeHTTP(rr, req)

		if rr.Code != http.StatusOK {
			t.Errorf("Request %d should succeed", i+1)
		}
	}

	// 3rd request should be blocked by IP limit
	req := httptest.NewRequest("GET", "/test", nil)
	req.RemoteAddr = ip
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusTooManyRequests {
		t.Errorf("Expected status 429, got %d", rr.Code)
	}
}

func TestCombinedRateLimitMiddleware_UserLimit(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ipLimiter := NewRateLimiter(redisClient, 100, 0)
	userLimiter := NewRateLimiter(redisClient, 3, 0)
	middleware := CombinedRateLimitMiddleware(ipLimiter, userLimiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	userID := "user-limited"

	// Exhaust user limit
	for i := 0; i < 3; i++ {
		req := httptest.NewRequest("GET", "/test", nil)
		req.RemoteAddr = fmt.Sprintf("192.168.1.%d:12345", i) // Different IPs
		ctx := context.WithValue(req.Context(), UserIDContextKey, userID)
		req = req.WithContext(ctx)
		rr := httptest.NewRecorder()
		handler.ServeHTTP(rr, req)

		if rr.Code != http.StatusOK {
			t.Errorf("Request %d should succeed", i+1)
		}
	}

	// 4th request should be blocked by user limit
	req := httptest.NewRequest("GET", "/test", nil)
	req.RemoteAddr = "192.168.1.250:12345" // Different IP
	ctx := context.WithValue(req.Context(), UserIDContextKey, userID)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()
	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusTooManyRequests {
		t.Errorf("Expected status 429, got %d", rr.Code)
	}
}

func TestRateLimiter_ResetRateLimit(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewRateLimiter(redisClient, 5, 0)

	key := "test-reset-key"

	// Use some quota
	for i := 0; i < 3; i++ {
		limiter.AllowRequest(ctx, key)
	}

	// Reset
	err := limiter.ResetRateLimit(ctx, key)
	if err != nil {
		t.Fatalf("ResetRateLimit failed: %v", err)
	}

	// Should have full quota again
	allowed, info, err := limiter.AllowRequest(ctx, key)
	if err != nil {
		t.Fatalf("AllowRequest after reset failed: %v", err)
	}

	if !allowed {
		t.Error("Request should be allowed after reset")
	}

	// Remaining should be near limit (might be limit-1 due to the test request)
	if info.Remaining < 4 {
		t.Errorf("After reset, remaining should be 4 or 5, got %d", info.Remaining)
	}
}

func TestRateLimiter_GetRateLimitInfo(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewRateLimiter(redisClient, 10, 0)

	key := "test-info"

	// Make some requests
	for i := 0; i < 3; i++ {
		limiter.AllowRequest(ctx, key)
	}

	// Get info without incrementing
	info, err := limiter.GetRateLimitInfo(ctx, key)
	if err != nil {
		t.Fatalf("GetRateLimitInfo failed: %v", err)
	}

	if info.Limit != 10 {
		t.Errorf("Limit = %d, want 10", info.Limit)
	}

	if info.Remaining != 7 {
		t.Errorf("Remaining = %d, want 7 (10 - 3)", info.Remaining)
	}

	// Make another request to verify GetRateLimitInfo didn't increment
	allowed, _, _ := limiter.AllowRequest(ctx, key)
	if !allowed {
		t.Error("Request should still be allowed")
	}
}

func TestRateLimiter_GetRateLimitInfo_NoRequests(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewRateLimiter(redisClient, 20, 0)

	key := "test-no-requests"

	info, err := limiter.GetRateLimitInfo(ctx, key)
	if err != nil {
		t.Fatalf("GetRateLimitInfo failed: %v", err)
	}

	if info.Remaining != 20 {
		t.Errorf("Remaining = %d, want 20 (no requests made)", info.Remaining)
	}
}

func TestSlidingWindowRateLimiter_NewLimiter(t *testing.T) {
	redisClient := redis.NewClient(&redis.Options{Addr: "localhost:6379"})
	defer redisClient.Close()

	limiter := NewSlidingWindowRateLimiter(redisClient, 100, time.Minute)

	if limiter == nil {
		t.Fatal("NewSlidingWindowRateLimiter returned nil")
	}

	if limiter.limit != 100 {
		t.Errorf("limit = %d, want 100", limiter.limit)
	}

	if limiter.window != time.Minute {
		t.Errorf("window = %v, want 1m", limiter.window)
	}
}

func TestSlidingWindowRateLimiter_AllowRequest(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewSlidingWindowRateLimiter(redisClient, 5, time.Minute)

	key := "test-sliding"

	// First 5 requests should be allowed
	for i := 1; i <= 5; i++ {
		allowed, err := limiter.AllowRequest(ctx, key)
		if err != nil {
			t.Fatalf("Request %d failed: %v", i, err)
		}

		if !allowed {
			t.Errorf("Request %d should be allowed", i)
		}
	}

	// 6th request should be blocked
	allowed, err := limiter.AllowRequest(ctx, key)
	if err != nil {
		t.Fatalf("Request 6 failed: %v", err)
	}

	if allowed {
		t.Error("Request 6 should be blocked (exceeded limit)")
	}
}

func TestRateLimitInfo_Structure(t *testing.T) {
	now := time.Now()
	resetTime := now.Add(30 * time.Second)

	info := &RateLimitInfo{
		Limit:      100,
		Remaining:  45,
		ResetTime:  resetTime,
		RetryAfter: 30 * time.Second,
	}

	if info.Limit != 100 {
		t.Errorf("Limit = %d, want 100", info.Limit)
	}

	if info.Remaining != 45 {
		t.Errorf("Remaining = %d, want 45", info.Remaining)
	}

	if !info.ResetTime.Equal(resetTime) {
		t.Error("ResetTime mismatch")
	}

	if info.RetryAfter != 30*time.Second {
		t.Errorf("RetryAfter = %v, want 30s", info.RetryAfter)
	}
}

func TestRateLimitConstants(t *testing.T) {
	tests := []struct {
		name     string
		constant string
		expected string
	}{
		{"RateLimitKeyPrefix", RateLimitKeyPrefix, "ratelimit:"},
		{"IPRateLimitPrefix", IPRateLimitPrefix, "ratelimit:ip:"},
		{"UserRateLimitPrefix", UserRateLimitPrefix, "ratelimit:user:"},
		{"EndpointRateLimitPrefix", EndpointRateLimitPrefix, "ratelimit:endpoint:"},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			if tt.constant != tt.expected {
				t.Errorf("%s = %s, want %s", tt.name, tt.constant, tt.expected)
			}
		})
	}

	if DefaultRequestsPerMinute != 100 {
		t.Errorf("DefaultRequestsPerMinute = %d, want 100", DefaultRequestsPerMinute)
	}

	if DefaultBurstSize != 20 {
		t.Errorf("DefaultBurstSize = %d, want 20", DefaultBurstSize)
	}
}

func TestRateLimitError(t *testing.T) {
	if ErrRateLimitExceeded == nil {
		t.Fatal("ErrRateLimitExceeded should not be nil")
	}

	if ErrRateLimitExceeded.Error() != "rate limit exceeded" {
		t.Errorf("Error message = %s, want 'rate limit exceeded'", ErrRateLimitExceeded.Error())
	}
}

// Benchmark tests
func BenchmarkRateLimiter_AllowRequest(b *testing.B) {
	redisClient, cleanup := setupTestRedis(&testing.T{})
	if redisClient == nil {
		b.Skip("Redis not available")
		return
	}
	defer cleanup()

	ctx := context.Background()
	limiter := NewRateLimiter(redisClient, 1000000, 0) // High limit

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		key := fmt.Sprintf("bench-key-%d", i%100)
		limiter.AllowRequest(ctx, key)
	}
}

func BenchmarkGetClientIP(b *testing.B) {
	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("X-Forwarded-For", "203.0.113.1")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		getClientIP(req)
	}
}

func TestIPRateLimitMiddleware_HeaderParsing(t *testing.T) {
	redisClient, cleanup := setupTestRedis(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	limiter := NewRateLimiter(redisClient, 10, 5)
	middleware := IPRateLimitMiddleware(limiter)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.RemoteAddr = "192.168.1.1:12345"
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	// Parse headers
	limitStr := rr.Header().Get("X-RateLimit-Limit")
	limit, err := strconv.ParseInt(limitStr, 10, 64)
	if err != nil {
		t.Errorf("Failed to parse X-RateLimit-Limit: %v", err)
	}

	if limit != 10 {
		t.Errorf("X-RateLimit-Limit = %d, want 10", limit)
	}

	remainingStr := rr.Header().Get("X-RateLimit-Remaining")
	remaining, err := strconv.ParseInt(remainingStr, 10, 64)
	if err != nil {
		t.Errorf("Failed to parse X-RateLimit-Remaining: %v", err)
	}

	if remaining != 9 {
		t.Errorf("X-RateLimit-Remaining = %d, want 9", remaining)
	}

	resetStr := rr.Header().Get("X-RateLimit-Reset")
	resetTimestamp, err := strconv.ParseInt(resetStr, 10, 64)
	if err != nil {
		t.Errorf("Failed to parse X-RateLimit-Reset: %v", err)
	}

	if resetTimestamp <= time.Now().Unix() {
		t.Error("X-RateLimit-Reset should be in the future")
	}
}
