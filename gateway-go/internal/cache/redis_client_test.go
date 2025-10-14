package cache

import (
	"context"
	"testing"
	"time"

	"github.com/alicebob/miniredis/v2"
	"github.com/redis/go-redis/v9"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

func setupTestRedis(t *testing.T) (*RedisClient, *miniredis.Miniredis) {
	mr := miniredis.RunT(t)

	client := redis.NewClient(&redis.Options{
		Addr: mr.Addr(),
	})

	redisClient := &RedisClient{
		client: client,
		ctx:    context.Background(),
	}

	return redisClient, mr
}

func TestSessionCache(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	session := &Session{
		UserID:      "user123",
		Email:       "test@example.com",
		Role:        "admin",
		BankID:      "bank456",
		Permissions: []string{"read", "write"},
		CreatedAt:   time.Now(),
		ExpiresAt:   time.Now().Add(1 * time.Hour),
	}

	t.Run("Store and Get Session", func(t *testing.T) {
		err := rc.StoreSession("token123", session, 1*time.Hour)
		require.NoError(t, err)

		retrieved, err := rc.GetSession("token123")
		require.NoError(t, err)
		assert.Equal(t, session.UserID, retrieved.UserID)
		assert.Equal(t, session.Email, retrieved.Email)
		assert.Equal(t, session.Role, retrieved.Role)
	})

	t.Run("Revoke Session", func(t *testing.T) {
		err := rc.StoreSession("token456", session, 1*time.Hour)
		require.NoError(t, err)

		err = rc.RevokeSession("token456")
		require.NoError(t, err)

		_, err = rc.GetSession("token456")
		assert.Error(t, err)
	})

	t.Run("Get Non-existent Session", func(t *testing.T) {
		_, err := rc.GetSession("nonexistent")
		assert.Error(t, err)
	})
}

func TestRateLimiting(t *testing.T) {
	rc, mr := setupTestRedis(t)
	defer rc.Close()

	t.Run("Allow Within Limit", func(t *testing.T) {
		mr.FlushAll()

		allowed, err := rc.CheckRateLimit("user1", "/api/payments", 5, 1*time.Minute)
		require.NoError(t, err)
		assert.True(t, allowed)
	})

	t.Run("Deny When Limit Exceeded", func(t *testing.T) {
		mr.FlushAll()

		// Make 5 requests (limit)
		for i := 0; i < 5; i++ {
			allowed, err := rc.CheckRateLimit("user2", "/api/payments", 5, 1*time.Minute)
			require.NoError(t, err)
			// First 5 should succeed
			if i < 5 {
				assert.True(t, allowed, "Request %d should be allowed", i+1)
			}
			time.Sleep(1 * time.Millisecond) // Ensure unique timestamps
		}

		// 6th request should be denied
		allowed, err := rc.CheckRateLimit("user2", "/api/payments", 5, 1*time.Minute)
		require.NoError(t, err)
		assert.False(t, allowed, "6th request should be denied")
	})

	t.Run("Get Rate Limit Info", func(t *testing.T) {
		mr.FlushAll()

		// Make 3 requests
		for i := 0; i < 3; i++ {
			allowed, err := rc.CheckRateLimit("user3", "/api/settlements", 10, 1*time.Minute)
			require.NoError(t, err)
			assert.True(t, allowed, "Request %d should be allowed", i+1)
			time.Sleep(1 * time.Millisecond) // Ensure unique timestamps
		}

		count, err := rc.GetRateLimitInfo("user3", "/api/settlements", 1*time.Minute)
		require.NoError(t, err)
		// Should have 3 requests in the window
		assert.GreaterOrEqual(t, count, int64(3), "Should have at least 3 requests")
	})
}

func TestFXRateCache(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	rate := &FXRate{
		From:      "USD",
		To:        "EUR",
		Rate:      0.92,
		Timestamp: time.Now(),
		Source:    "Bloomberg",
	}

	t.Run("Store and Get FX Rate", func(t *testing.T) {
		err := rc.StoreFXRate(rate, 5*time.Minute)
		require.NoError(t, err)

		retrieved, err := rc.GetFXRate("USD", "EUR")
		require.NoError(t, err)
		assert.Equal(t, rate.Rate, retrieved.Rate)
		assert.Equal(t, rate.From, retrieved.From)
		assert.Equal(t, rate.To, retrieved.To)
	})

	t.Run("Get Non-existent FX Rate", func(t *testing.T) {
		_, err := rc.GetFXRate("JPY", "GBP")
		assert.Error(t, err)
	})
}

func TestBankLimitsCache(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	limits := &BankLimits{
		BankID:           "bank123",
		DailyLimit:       1000000.00,
		TransactionLimit: 50000.00,
		UsedToday:        250000.00,
		UpdatedAt:        time.Now(),
	}

	t.Run("Store and Get Bank Limits", func(t *testing.T) {
		err := rc.StoreBankLimits(limits, 10*time.Minute)
		require.NoError(t, err)

		retrieved, err := rc.GetBankLimits("bank123")
		require.NoError(t, err)
		assert.Equal(t, limits.BankID, retrieved.BankID)
		assert.Equal(t, limits.DailyLimit, retrieved.DailyLimit)
		assert.Equal(t, limits.UsedToday, retrieved.UsedToday)
	})

	t.Run("Increment Bank Usage", func(t *testing.T) {
		err := rc.IncrementBankUsage("bank456", 10000.50)
		require.NoError(t, err)

		usage, err := rc.GetBankUsage("bank456")
		require.NoError(t, err)
		assert.Equal(t, 10000.50, usage)

		// Increment again
		err = rc.IncrementBankUsage("bank456", 5000.25)
		require.NoError(t, err)

		usage, err = rc.GetBankUsage("bank456")
		require.NoError(t, err)
		assert.Equal(t, 15000.75, usage)
	})
}

func TestComplianceCache(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	check := &ComplianceCheck{
		EntityHash: "hash123",
		Status:     "pass",
		RiskScore:  25.5,
		CheckedAt:  time.Now(),
		ExpiresAt:  time.Now().Add(24 * time.Hour),
	}

	t.Run("Store and Get Compliance Check", func(t *testing.T) {
		err := rc.StoreComplianceCheck(check, 24*time.Hour)
		require.NoError(t, err)

		retrieved, err := rc.GetComplianceCheck("hash123")
		require.NoError(t, err)
		assert.Equal(t, check.Status, retrieved.Status)
		assert.Equal(t, check.RiskScore, retrieved.RiskScore)
	})
}

func TestIdempotencyCache(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	response := &IdempotentResponse{
		StatusCode: 200,
		Body:       []byte(`{"success": true}`),
		Headers: map[string]string{
			"Content-Type": "application/json",
		},
		Timestamp: time.Now(),
	}

	t.Run("Store and Get Idempotent Response", func(t *testing.T) {
		err := rc.StoreIdempotentResponse("key123", response, 24*time.Hour)
		require.NoError(t, err)

		retrieved, err := rc.GetIdempotentResponse("key123")
		require.NoError(t, err)
		assert.Equal(t, response.StatusCode, retrieved.StatusCode)
		assert.Equal(t, response.Body, retrieved.Body)
	})
}

func TestDistributedLock(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	t.Run("Acquire and Release Lock", func(t *testing.T) {
		acquired, err := rc.AcquireLock("resource1", 10*time.Second)
		require.NoError(t, err)
		assert.True(t, acquired)

		// Try to acquire same lock
		acquired, err = rc.AcquireLock("resource1", 10*time.Second)
		require.NoError(t, err)
		assert.False(t, acquired)

		// Release lock
		err = rc.ReleaseLock("resource1")
		require.NoError(t, err)

		// Should be able to acquire again
		acquired, err = rc.AcquireLock("resource1", 10*time.Second)
		require.NoError(t, err)
		assert.True(t, acquired)
	})
}

func TestGenericCacheOperations(t *testing.T) {
	rc, _ := setupTestRedis(t)
	defer rc.Close()

	t.Run("Set and Get", func(t *testing.T) {
		data := map[string]interface{}{
			"key1": "value1",
			"key2": 123,
		}

		err := rc.Set("test_key", data, 1*time.Minute)
		require.NoError(t, err)

		var retrieved map[string]interface{}
		err = rc.Get("test_key", &retrieved)
		require.NoError(t, err)
		assert.Equal(t, "value1", retrieved["key1"])
	})

	t.Run("Delete", func(t *testing.T) {
		err := rc.Set("delete_key", "value", 1*time.Minute)
		require.NoError(t, err)

		err = rc.Delete("delete_key")
		require.NoError(t, err)

		exists, err := rc.Exists("delete_key")
		require.NoError(t, err)
		assert.False(t, exists)
	})

	t.Run("Exists", func(t *testing.T) {
		err := rc.Set("exists_key", "value", 1*time.Minute)
		require.NoError(t, err)

		exists, err := rc.Exists("exists_key")
		require.NoError(t, err)
		assert.True(t, exists)

		exists, err = rc.Exists("nonexistent_key")
		require.NoError(t, err)
		assert.False(t, exists)
	})

	t.Run("Ping", func(t *testing.T) {
		err := rc.Ping()
		require.NoError(t, err)
	})
}

func BenchmarkRateLimiting(b *testing.B) {
	mr := miniredis.RunT(&testing.T{})
	defer mr.Close()

	client := redis.NewClient(&redis.Options{
		Addr: mr.Addr(),
	})

	rc := &RedisClient{
		client: client,
		ctx:    context.Background(),
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rc.CheckRateLimit("user", "/api/test", 1000, 1*time.Minute)
	}
}

func BenchmarkSessionCache(b *testing.B) {
	mr := miniredis.RunT(&testing.T{})
	defer mr.Close()

	client := redis.NewClient(&redis.Options{
		Addr: mr.Addr(),
	})

	rc := &RedisClient{
		client: client,
		ctx:    context.Background(),
	}

	session := &Session{
		UserID:      "user123",
		Email:       "test@example.com",
		Role:        "admin",
		Permissions: []string{"read", "write"},
		CreatedAt:   time.Now(),
		ExpiresAt:   time.Now().Add(1 * time.Hour),
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		rc.StoreSession("token", session, 1*time.Hour)
		rc.GetSession("token")
	}
}
