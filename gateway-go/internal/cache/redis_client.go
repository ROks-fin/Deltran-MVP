package cache

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/redis/go-redis/v9"
)

// RedisClient wraps Redis functionality for caching
type RedisClient struct {
	client *redis.Client
	ctx    context.Context
}

// CacheConfig holds Redis cache configuration
type CacheConfig struct {
	Addr     string
	Password string
	DB       int
	PoolSize int
}

// NewRedisClient creates a new Redis client
func NewRedisClient(config CacheConfig) (*RedisClient, error) {
	client := redis.NewClient(&redis.Options{
		Addr:         config.Addr,
		Password:     config.Password,
		DB:           config.DB,
		PoolSize:     config.PoolSize,
		MinIdleConns: 10,
		MaxRetries:   3,
		DialTimeout:  5 * time.Second,
		ReadTimeout:  3 * time.Second,
		WriteTimeout: 3 * time.Second,
		PoolTimeout:  4 * time.Second,
	})

	// Ping to verify connection
	ctx := context.Background()
	if err := client.Ping(ctx).Err(); err != nil {
		return nil, fmt.Errorf("failed to connect to Redis: %w", err)
	}

	return &RedisClient{
		client: client,
		ctx:    ctx,
	}, nil
}

// Close closes the Redis connection
func (r *RedisClient) Close() error {
	return r.client.Close()
}

// ========================================
// SESSION CACHE
// ========================================

// Session represents cached session data
type Session struct {
	UserID      string    `json:"user_id"`
	Email       string    `json:"email"`
	Role        string    `json:"role"`
	BankID      string    `json:"bank_id,omitempty"`
	Permissions []string  `json:"permissions"`
	CreatedAt   time.Time `json:"created_at"`
	ExpiresAt   time.Time `json:"expires_at"`
}

// StoreSession stores a session in cache
func (r *RedisClient) StoreSession(tokenHash string, session *Session, ttl time.Duration) error {
	key := fmt.Sprintf("session:%s", tokenHash)
	data, err := json.Marshal(session)
	if err != nil {
		return fmt.Errorf("failed to marshal session: %w", err)
	}

	return r.client.Set(r.ctx, key, data, ttl).Err()
}

// GetSession retrieves a session from cache
func (r *RedisClient) GetSession(tokenHash string) (*Session, error) {
	key := fmt.Sprintf("session:%s", tokenHash)
	data, err := r.client.Get(r.ctx, key).Bytes()
	if err == redis.Nil {
		return nil, fmt.Errorf("session not found")
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get session: %w", err)
	}

	var session Session
	if err := json.Unmarshal(data, &session); err != nil {
		return nil, fmt.Errorf("failed to unmarshal session: %w", err)
	}

	return &session, nil
}

// RevokeSession removes a session from cache
func (r *RedisClient) RevokeSession(tokenHash string) error {
	key := fmt.Sprintf("session:%s", tokenHash)
	return r.client.Del(r.ctx, key).Err()
}

// ========================================
// RATE LIMITING
// ========================================

// CheckRateLimit checks if rate limit is exceeded using sliding window
func (r *RedisClient) CheckRateLimit(identifier, endpoint string, limit int, window time.Duration) (bool, error) {
	now := time.Now()
	nowUnix := now.UnixNano()
	windowStart := now.Add(-window).UnixNano()
	key := fmt.Sprintf("ratelimit:%s:%s", identifier, endpoint)

	// Use pipeline for atomic operation
	pipe := r.client.Pipeline()

	// Remove old entries
	pipe.ZRemRangeByScore(r.ctx, key, "0", fmt.Sprintf("%d", windowStart))

	// Add current request with unique member (nanosecond timestamp)
	member := fmt.Sprintf("%d", nowUnix)
	pipe.ZAdd(r.ctx, key, redis.Z{
		Score:  float64(nowUnix),
		Member: member,
	})

	// Count requests in window
	zcard := pipe.ZCard(r.ctx, key)

	// Set expiry
	pipe.Expire(r.ctx, key, window*2)

	// Execute pipeline
	_, err := pipe.Exec(r.ctx)
	if err != nil {
		return false, err
	}

	// Check if limit exceeded
	count, err := zcard.Result()
	if err != nil {
		return false, err
	}

	if count > int64(limit) {
		// Remove the request we just added since we exceeded the limit
		r.client.ZRem(r.ctx, key, member)
		return false, nil // Rate limit exceeded
	}

	return true, nil
}

// GetRateLimitInfo returns current rate limit usage
func (r *RedisClient) GetRateLimitInfo(identifier, endpoint string, window time.Duration) (int64, error) {
	now := time.Now()
	windowStart := now.Add(-window).UnixNano()
	key := fmt.Sprintf("ratelimit:%s:%s", identifier, endpoint)

	count, err := r.client.ZCount(r.ctx, key, fmt.Sprintf("%d", windowStart), "+inf").Result()
	if err != nil {
		return 0, err
	}

	return count, nil
}

// ========================================
// FX RATE CACHE
// ========================================

// FXRate represents a foreign exchange rate
type FXRate struct {
	From      string    `json:"from"`
	To        string    `json:"to"`
	Rate      float64   `json:"rate"`
	Timestamp time.Time `json:"timestamp"`
	Source    string    `json:"source"`
}

// StoreFXRate caches an FX rate
func (r *RedisClient) StoreFXRate(rate *FXRate, ttl time.Duration) error {
	key := fmt.Sprintf("fx:%s:%s", rate.From, rate.To)
	data, err := json.Marshal(rate)
	if err != nil {
		return fmt.Errorf("failed to marshal FX rate: %w", err)
	}

	return r.client.Set(r.ctx, key, data, ttl).Err()
}

// GetFXRate retrieves an FX rate from cache
func (r *RedisClient) GetFXRate(from, to string) (*FXRate, error) {
	key := fmt.Sprintf("fx:%s:%s", from, to)
	data, err := r.client.Get(r.ctx, key).Bytes()
	if err == redis.Nil {
		return nil, fmt.Errorf("FX rate not found")
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get FX rate: %w", err)
	}

	var rate FXRate
	if err := json.Unmarshal(data, &rate); err != nil {
		return nil, fmt.Errorf("failed to unmarshal FX rate: %w", err)
	}

	return &rate, nil
}

// ========================================
// BANK LIMITS CACHE
// ========================================

// BankLimits represents cached bank limits
type BankLimits struct {
	BankID         string    `json:"bank_id"`
	DailyLimit     float64   `json:"daily_limit"`
	TransactionLimit float64 `json:"transaction_limit"`
	UsedToday      float64   `json:"used_today"`
	UpdatedAt      time.Time `json:"updated_at"`
}

// StoreBankLimits caches bank limits
func (r *RedisClient) StoreBankLimits(limits *BankLimits, ttl time.Duration) error {
	key := fmt.Sprintf("limits:%s", limits.BankID)
	data, err := json.Marshal(limits)
	if err != nil {
		return fmt.Errorf("failed to marshal bank limits: %w", err)
	}

	return r.client.Set(r.ctx, key, data, ttl).Err()
}

// GetBankLimits retrieves bank limits from cache
func (r *RedisClient) GetBankLimits(bankID string) (*BankLimits, error) {
	key := fmt.Sprintf("limits:%s", bankID)
	data, err := r.client.Get(r.ctx, key).Bytes()
	if err == redis.Nil {
		return nil, fmt.Errorf("bank limits not found")
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get bank limits: %w", err)
	}

	var limits BankLimits
	if err := json.Unmarshal(data, &limits); err != nil {
		return nil, fmt.Errorf("failed to unmarshal bank limits: %w", err)
	}

	return &limits, nil
}

// IncrementBankUsage atomically increments daily usage
func (r *RedisClient) IncrementBankUsage(bankID string, amount float64) error {
	key := fmt.Sprintf("usage:%s:%s", bankID, time.Now().Format("2006-01-02"))

	pipe := r.client.Pipeline()
	pipe.IncrByFloat(r.ctx, key, amount)
	pipe.Expire(r.ctx, key, 48*time.Hour) // Keep for 2 days

	_, err := pipe.Exec(r.ctx)
	return err
}

// GetBankUsage gets current daily usage
func (r *RedisClient) GetBankUsage(bankID string) (float64, error) {
	key := fmt.Sprintf("usage:%s:%s", bankID, time.Now().Format("2006-01-02"))
	return r.client.Get(r.ctx, key).Float64()
}

// ========================================
// COMPLIANCE CACHE
// ========================================

// ComplianceCheck represents cached compliance check result
type ComplianceCheck struct {
	EntityHash string    `json:"entity_hash"`
	Status     string    `json:"status"`
	RiskScore  float64   `json:"risk_score"`
	CheckedAt  time.Time `json:"checked_at"`
	ExpiresAt  time.Time `json:"expires_at"`
}

// StoreComplianceCheck caches a compliance check
func (r *RedisClient) StoreComplianceCheck(check *ComplianceCheck, ttl time.Duration) error {
	key := fmt.Sprintf("compliance:%s", check.EntityHash)
	data, err := json.Marshal(check)
	if err != nil {
		return fmt.Errorf("failed to marshal compliance check: %w", err)
	}

	return r.client.Set(r.ctx, key, data, ttl).Err()
}

// GetComplianceCheck retrieves a compliance check from cache
func (r *RedisClient) GetComplianceCheck(entityHash string) (*ComplianceCheck, error) {
	key := fmt.Sprintf("compliance:%s", entityHash)
	data, err := r.client.Get(r.ctx, key).Bytes()
	if err == redis.Nil {
		return nil, fmt.Errorf("compliance check not found")
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get compliance check: %w", err)
	}

	var check ComplianceCheck
	if err := json.Unmarshal(data, &check); err != nil {
		return nil, fmt.Errorf("failed to unmarshal compliance check: %w", err)
	}

	return &check, nil
}

// ========================================
// IDEMPOTENCY CACHE
// ========================================

// IdempotentResponse represents a cached API response for idempotency
type IdempotentResponse struct {
	StatusCode  int               `json:"status_code"`
	Body        []byte            `json:"body"`
	Headers     map[string]string `json:"headers"`
	Timestamp   time.Time         `json:"timestamp"`
}

// StoreIdempotentResponse caches an API response
func (r *RedisClient) StoreIdempotentResponse(key string, response *IdempotentResponse, ttl time.Duration) error {
	cacheKey := fmt.Sprintf("idempotent:%s", key)
	data, err := json.Marshal(response)
	if err != nil {
		return fmt.Errorf("failed to marshal response: %w", err)
	}

	return r.client.Set(r.ctx, cacheKey, data, ttl).Err()
}

// GetIdempotentResponse retrieves a cached API response
func (r *RedisClient) GetIdempotentResponse(key string) (*IdempotentResponse, error) {
	cacheKey := fmt.Sprintf("idempotent:%s", key)
	data, err := r.client.Get(r.ctx, cacheKey).Bytes()
	if err == redis.Nil {
		return nil, fmt.Errorf("response not found")
	}
	if err != nil {
		return nil, fmt.Errorf("failed to get response: %w", err)
	}

	var response IdempotentResponse
	if err := json.Unmarshal(data, &response); err != nil {
		return nil, fmt.Errorf("failed to unmarshal response: %w", err)
	}

	return &response, nil
}

// AcquireLock acquires a distributed lock
func (r *RedisClient) AcquireLock(key string, ttl time.Duration) (bool, error) {
	lockKey := fmt.Sprintf("lock:%s", key)
	return r.client.SetNX(r.ctx, lockKey, "locked", ttl).Result()
}

// ReleaseLock releases a distributed lock
func (r *RedisClient) ReleaseLock(key string) error {
	lockKey := fmt.Sprintf("lock:%s", key)
	return r.client.Del(r.ctx, lockKey).Err()
}

// ========================================
// GENERIC CACHE OPERATIONS
// ========================================

// Set stores a value with TTL
func (r *RedisClient) Set(key string, value interface{}, ttl time.Duration) error {
	data, err := json.Marshal(value)
	if err != nil {
		return err
	}
	return r.client.Set(r.ctx, key, data, ttl).Err()
}

// Get retrieves a value
func (r *RedisClient) Get(key string, dest interface{}) error {
	data, err := r.client.Get(r.ctx, key).Bytes()
	if err != nil {
		return err
	}
	return json.Unmarshal(data, dest)
}

// Delete removes a key
func (r *RedisClient) Delete(key string) error {
	return r.client.Del(r.ctx, key).Err()
}

// Exists checks if a key exists
func (r *RedisClient) Exists(key string) (bool, error) {
	result, err := r.client.Exists(r.ctx, key).Result()
	return result > 0, err
}

// Expire sets TTL on a key
func (r *RedisClient) Expire(key string, ttl time.Duration) error {
	return r.client.Expire(r.ctx, key, ttl).Err()
}

// FlushDB clears all keys (use with caution!)
func (r *RedisClient) FlushDB() error {
	return r.client.FlushDB(r.ctx).Err()
}

// Ping checks Redis connection
func (r *RedisClient) Ping() error {
	return r.client.Ping(r.ctx).Err()
}

// GetStats returns Redis statistics
func (r *RedisClient) GetStats() (map[string]string, error) {
	info, err := r.client.Info(r.ctx, "stats").Result()
	if err != nil {
		return nil, err
	}

	stats := make(map[string]string)
	stats["info"] = info

	// Get keyspace info
	dbSize, err := r.client.DBSize(r.ctx).Result()
	if err != nil {
		return nil, err
	}
	stats["keys_count"] = fmt.Sprintf("%d", dbSize)

	// Get pool stats
	poolStats := r.client.PoolStats()
	stats["pool_hits"] = fmt.Sprintf("%d", poolStats.Hits)
	stats["pool_misses"] = fmt.Sprintf("%d", poolStats.Misses)
	stats["pool_timeouts"] = fmt.Sprintf("%d", poolStats.Timeouts)
	stats["pool_total_conns"] = fmt.Sprintf("%d", poolStats.TotalConns)
	stats["pool_idle_conns"] = fmt.Sprintf("%d", poolStats.IdleConns)
	stats["pool_stale_conns"] = fmt.Sprintf("%d", poolStats.StaleConns)

	return stats, nil
}
