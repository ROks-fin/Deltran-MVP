package resilience

import (
	"context"
	"errors"
	"testing"
	"time"

	"github.com/redis/go-redis/v9"
)

// setupTestRedis creates test Redis client
func setupTestRedisResilience(t *testing.T) (*redis.Client, func()) {
	redisClient := redis.NewClient(&redis.Options{
		Addr: "localhost:6379",
		DB:   14, // Separate DB for resilience tests
	})

	ctx := context.Background()

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

func TestNewIdempotencyManager(t *testing.T) {
	redisClient := redis.NewClient(&redis.Options{Addr: "localhost:6379"})
	defer redisClient.Close()

	// Test with custom TTL
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)
	if manager == nil {
		t.Fatal("NewIdempotencyManager returned nil")
	}

	if manager.ttl != 1*time.Hour {
		t.Errorf("TTL = %v, want 1h", manager.ttl)
	}

	if manager.prefix != "idempotency:" {
		t.Errorf("Prefix = %s, want idempotency:", manager.prefix)
	}

	// Test with zero TTL (should default to 24h)
	manager2 := NewIdempotencyManager(redisClient, 0)
	if manager2.ttl != 24*time.Hour {
		t.Errorf("Default TTL = %v, want 24h", manager2.ttl)
	}
}

func TestIdempotencyManager_Store_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "test-key-1"
	response := map[string]string{"result": "success"}
	statusCode := 200

	err := manager.Store(ctx, key, response, statusCode)
	if err != nil {
		t.Fatalf("Store failed: %v", err)
	}

	// Verify stored
	exists, err := manager.Exists(ctx, key)
	if err != nil {
		t.Fatalf("Exists check failed: %v", err)
	}

	if !exists {
		t.Error("Key should exist after storing")
	}
}

func TestIdempotencyManager_Get_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "test-key-2"
	response := map[string]interface{}{"payment_id": "pay123", "status": "completed"}
	statusCode := 201

	// Store
	err := manager.Store(ctx, key, response, statusCode)
	if err != nil {
		t.Fatalf("Store failed: %v", err)
	}

	// Get
	result, err := manager.Get(ctx, key)
	if err != nil {
		t.Fatalf("Get failed: %v", err)
	}

	if result == nil {
		t.Fatal("Result should not be nil")
	}

	if result.Key != key {
		t.Errorf("Key = %s, want %s", result.Key, key)
	}

	if result.StatusCode != statusCode {
		t.Errorf("StatusCode = %d, want %d", result.StatusCode, statusCode)
	}

	if result.CreatedAt.IsZero() {
		t.Error("CreatedAt should be set")
	}

	if result.ExpiresAt.IsZero() {
		t.Error("ExpiresAt should be set")
	}
}

func TestIdempotencyManager_Get_NotFound(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	result, err := manager.Get(ctx, "nonexistent-key")

	if err != nil {
		t.Errorf("Get should not return error for missing key, got: %v", err)
	}

	if result != nil {
		t.Error("Result should be nil for missing key")
	}
}

func TestIdempotencyManager_Execute_CacheHit(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "execute-test-1"
	executionCount := 0
	expectedResponse := "result"

	fn := func() (interface{}, int, error) {
		executionCount++
		return expectedResponse, 200, nil
	}

	// First execution - should call function
	result1, status1, err1 := manager.Execute(ctx, key, fn)
	if err1 != nil {
		t.Fatalf("First Execute failed: %v", err1)
	}

	if executionCount != 1 {
		t.Errorf("Function should be called once, was called %d times", executionCount)
	}

	if result1 != expectedResponse {
		t.Errorf("Result = %v, want %v", result1, expectedResponse)
	}

	if status1 != 200 {
		t.Errorf("Status = %d, want 200", status1)
	}

	// Second execution - should return cached result
	result2, status2, err2 := manager.Execute(ctx, key, fn)
	if err2 != nil {
		t.Fatalf("Second Execute failed: %v", err2)
	}

	if executionCount != 1 {
		t.Errorf("Function should still be called only once, was called %d times", executionCount)
	}

	if result2 != result1 {
		t.Error("Second call should return cached result")
	}

	if status2 != status1 {
		t.Error("Second call should return cached status")
	}
}

func TestIdempotencyManager_Execute_FunctionError(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "execute-error-test"
	expectedError := errors.New("function failed")

	fn := func() (interface{}, int, error) {
		return nil, 0, expectedError
	}

	result, status, err := manager.Execute(ctx, key, fn)

	if err != expectedError {
		t.Errorf("Expected error %v, got %v", expectedError, err)
	}

	if result != nil {
		t.Error("Result should be nil on error")
	}

	if status != 0 {
		t.Errorf("Status should be 0 on error, got %d", status)
	}
}

func TestIdempotencyManager_Delete_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "delete-test"

	// Store
	manager.Store(ctx, key, "data", 200)

	// Verify exists
	exists, _ := manager.Exists(ctx, key)
	if !exists {
		t.Fatal("Key should exist before deletion")
	}

	// Delete
	err := manager.Delete(ctx, key)
	if err != nil {
		t.Fatalf("Delete failed: %v", err)
	}

	// Verify deleted
	exists, _ = manager.Exists(ctx, key)
	if exists {
		t.Error("Key should not exist after deletion")
	}
}

func TestIdempotencyManager_Exists_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "exists-test"

	// Check non-existent key
	exists, err := manager.Exists(ctx, key)
	if err != nil {
		t.Fatalf("Exists check failed: %v", err)
	}
	if exists {
		t.Error("Non-existent key should return false")
	}

	// Store key
	manager.Store(ctx, key, "data", 200)

	// Check existing key
	exists, err = manager.Exists(ctx, key)
	if err != nil {
		t.Fatalf("Exists check failed: %v", err)
	}
	if !exists {
		t.Error("Existing key should return true")
	}
}

func TestGenerateKey(t *testing.T) {
	tests := []struct {
		name   string
		prefix string
		data   []string
		want   int // Expected length
	}{
		{
			name:   "with prefix",
			prefix: "payment",
			data:   []string{"BIC1", "BIC2", "100.00", "USD"},
			want:   16 + len("payment-"), // payment- + 16 char hash
		},
		{
			name:   "without prefix",
			prefix: "",
			data:   []string{"data1", "data2"},
			want:   16, // 16 char hash
		},
		{
			name:   "single data element",
			prefix: "test",
			data:   []string{"single"},
			want:   16 + len("test-"),
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			key := GenerateKey(tt.prefix, tt.data...)

			if len(key) != tt.want {
				t.Errorf("Key length = %d, want %d", len(key), tt.want)
			}

			// Test deterministic - same input should produce same key
			key2 := GenerateKey(tt.prefix, tt.data...)
			if key != key2 {
				t.Error("Same input should produce same key")
			}

			// Different input should produce different key
			if len(tt.data) > 0 {
				differentData := make([]string, len(tt.data))
				copy(differentData, tt.data)
				differentData[0] = differentData[0] + "-different"
				key3 := GenerateKey(tt.prefix, differentData...)
				if key == key3 {
					t.Error("Different input should produce different key")
				}
			}
		})
	}
}

func TestGeneratePaymentKey(t *testing.T) {
	key := GeneratePaymentKey("BANKGB2L", "BANKUS33", "1000.00", "USD", "REF123")

	if key == "" {
		t.Fatal("GeneratePaymentKey returned empty string")
	}

	// Should start with "payment-"
	if len(key) < 8 || key[:8] != "payment-" {
		t.Errorf("Key should start with 'payment-', got: %s", key)
	}

	// Should be deterministic
	key2 := GeneratePaymentKey("BANKGB2L", "BANKUS33", "1000.00", "USD", "REF123")
	if key != key2 {
		t.Error("Same payment data should generate same key")
	}

	// Different data should generate different key
	key3 := GeneratePaymentKey("BANKGB2L", "BANKUS33", "2000.00", "USD", "REF123")
	if key == key3 {
		t.Error("Different payment data should generate different key")
	}
}

func TestIdempotencyManager_AcquireLock_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "lock-test-1"
	ttl := 5 * time.Second

	// Acquire lock
	lock, err := manager.AcquireLock(ctx, key, ttl)
	if err != nil {
		t.Fatalf("AcquireLock failed: %v", err)
	}

	if lock == nil {
		t.Fatal("Lock should not be nil")
	}

	if lock.key == "" {
		t.Error("Lock key should be set")
	}

	if lock.token == "" {
		t.Error("Lock token should be set")
	}

	// Try to acquire same lock again - should fail
	_, err = manager.AcquireLock(ctx, key, ttl)
	if err == nil {
		t.Error("Second lock acquisition should fail")
	}

	// Release lock
	err = lock.Release(ctx)
	if err != nil {
		t.Fatalf("Release failed: %v", err)
	}

	// Should be able to acquire again after release
	lock2, err := manager.AcquireLock(ctx, key, ttl)
	if err != nil {
		t.Errorf("Lock acquisition after release failed: %v", err)
	}
	if lock2 != nil {
		lock2.Release(ctx)
	}
}

func TestProcessingLock_Release_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	lock, err := manager.AcquireLock(ctx, "release-test", 5*time.Second)
	if err != nil {
		t.Fatalf("AcquireLock failed: %v", err)
	}

	// Release
	err = lock.Release(ctx)
	if err != nil {
		t.Fatalf("Release failed: %v", err)
	}

	// Double release should not error
	err = lock.Release(ctx)
	if err != nil {
		t.Errorf("Double release should not error: %v", err)
	}
}

func TestProcessingLock_Extend_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	lock, err := manager.AcquireLock(ctx, "extend-test", 1*time.Second)
	if err != nil {
		t.Fatalf("AcquireLock failed: %v", err)
	}
	defer lock.Release(ctx)

	// Extend lock
	err = lock.Extend(ctx, 10*time.Second)
	if err != nil {
		t.Fatalf("Extend failed: %v", err)
	}

	// Lock should still be valid after original TTL
	time.Sleep(1500 * time.Millisecond)

	// Try to acquire - should fail because lock was extended
	_, err = manager.AcquireLock(ctx, "extend-test", 1*time.Second)
	if err == nil {
		t.Error("Lock should still be held after extension")
	}
}

func TestIdempotencyManager_ExecuteWithLock_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "execute-lock-test"
	executed := false

	fn := func() error {
		executed = true
		return nil
	}

	err := manager.ExecuteWithLock(ctx, key, 5*time.Second, fn)
	if err != nil {
		t.Fatalf("ExecuteWithLock failed: %v", err)
	}

	if !executed {
		t.Error("Function should have been executed")
	}

	// Lock should be released after execution
	lock, err := manager.AcquireLock(ctx, key, 1*time.Second)
	if err != nil {
		t.Errorf("Lock should be released after ExecuteWithLock: %v", err)
	}
	if lock != nil {
		lock.Release(ctx)
	}
}

func TestIdempotencyManager_ExecuteWithLock_FunctionError(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	key := "execute-lock-error-test"
	expectedError := errors.New("function error")

	fn := func() error {
		return expectedError
	}

	err := manager.ExecuteWithLock(ctx, key, 5*time.Second, fn)
	if err != expectedError {
		t.Errorf("Expected error %v, got %v", expectedError, err)
	}

	// Lock should still be released even on error
	lock, err := manager.AcquireLock(ctx, key, 1*time.Second)
	if err != nil {
		t.Errorf("Lock should be released even on function error: %v", err)
	}
	if lock != nil {
		lock.Release(ctx)
	}
}

func TestIdempotencyManager_GetStats_WithRedis(t *testing.T) {
	redisClient, cleanup := setupTestRedisResilience(t)
	if redisClient == nil {
		return
	}
	defer cleanup()

	ctx := context.Background()
	manager := NewIdempotencyManager(redisClient, 1*time.Hour)

	// Initially should have 0 keys
	stats, err := manager.GetStats(ctx)
	if err != nil {
		t.Fatalf("GetStats failed: %v", err)
	}

	if stats.TotalKeys != 0 {
		t.Errorf("Initially TotalKeys = %d, want 0", stats.TotalKeys)
	}

	// Store some keys
	manager.Store(ctx, "key1", "data1", 200)
	manager.Store(ctx, "key2", "data2", 200)
	manager.Store(ctx, "key3", "data3", 200)

	// Check stats again
	stats, err = manager.GetStats(ctx)
	if err != nil {
		t.Fatalf("GetStats failed: %v", err)
	}

	if stats.TotalKeys != 3 {
		t.Errorf("TotalKeys = %d, want 3", stats.TotalKeys)
	}
}

func TestIdempotencyResult_Structure(t *testing.T) {
	now := time.Now()

	result := &IdempotencyResult{
		Key:        "test-key",
		Response:   map[string]string{"status": "ok"},
		StatusCode: 200,
		CreatedAt:  now,
		ExpiresAt:  now.Add(24 * time.Hour),
	}

	if result.Key != "test-key" {
		t.Errorf("Key = %s, want test-key", result.Key)
	}

	if result.StatusCode != 200 {
		t.Errorf("StatusCode = %d, want 200", result.StatusCode)
	}

	if result.CreatedAt != now {
		t.Error("CreatedAt mismatch")
	}

	if result.Response == nil {
		t.Error("Response should not be nil")
	}
}

func TestIdempotencyErrors(t *testing.T) {
	if ErrDuplicateRequest == nil {
		t.Error("ErrDuplicateRequest should not be nil")
	}

	if ErrDuplicateRequest.Error() != "duplicate request detected" {
		t.Errorf("ErrDuplicateRequest message = %s", ErrDuplicateRequest.Error())
	}

	if ErrKeyExpired == nil {
		t.Error("ErrKeyExpired should not be nil")
	}

	if ErrKeyExpired.Error() != "idempotency key expired" {
		t.Errorf("ErrKeyExpired message = %s", ErrKeyExpired.Error())
	}
}

func TestIdempotencyStats_Structure(t *testing.T) {
	stats := &IdempotencyStats{
		TotalKeys:   100,
		CacheHits:   80,
		CacheMisses: 20,
		HitRate:     0.8,
	}

	if stats.TotalKeys != 100 {
		t.Errorf("TotalKeys = %d, want 100", stats.TotalKeys)
	}

	if stats.CacheHits != 80 {
		t.Errorf("CacheHits = %d, want 80", stats.CacheHits)
	}

	if stats.CacheMisses != 20 {
		t.Errorf("CacheMisses = %d, want 20", stats.CacheMisses)
	}

	if stats.HitRate != 0.8 {
		t.Errorf("HitRate = %f, want 0.8", stats.HitRate)
	}
}
