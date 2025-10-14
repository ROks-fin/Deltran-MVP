package resilience

import (
	"context"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"errors"
	"fmt"
	"time"

	"github.com/redis/go-redis/v9"
)

var (
	ErrDuplicateRequest = errors.New("duplicate request detected")
	ErrKeyExpired       = errors.New("idempotency key expired")
)

// IdempotencyResult represents the result of an idempotent operation
type IdempotencyResult struct {
	Key        string      `json:"key"`
	Response   interface{} `json:"response"`
	StatusCode int         `json:"status_code"`
	CreatedAt  time.Time   `json:"created_at"`
	ExpiresAt  time.Time   `json:"expires_at"`
}

// IdempotencyManager manages idempotency keys
type IdempotencyManager struct {
	redis  *redis.Client
	ttl    time.Duration
	prefix string
}

// NewIdempotencyManager creates a new idempotency manager
func NewIdempotencyManager(redisClient *redis.Client, ttl time.Duration) *IdempotencyManager {
	if ttl == 0 {
		ttl = 24 * time.Hour // Default 24 hours
	}

	return &IdempotencyManager{
		redis:  redisClient,
		ttl:    ttl,
		prefix: "idempotency:",
	}
}

// Execute executes function with idempotency protection
func (im *IdempotencyManager) Execute(ctx context.Context, key string, fn func() (interface{}, int, error)) (interface{}, int, error) {
	// Check if key already exists
	existing, err := im.Get(ctx, key)
	if err == nil && existing != nil {
		// Return cached result
		return existing.Response, existing.StatusCode, nil
	}

	// Key doesn't exist or expired, execute function
	result, statusCode, err := fn()
	if err != nil {
		return nil, 0, err
	}

	// Store result
	if err := im.Store(ctx, key, result, statusCode); err != nil {
		// Log error but return result anyway
		fmt.Printf("Failed to store idempotency key: %v\n", err)
	}

	return result, statusCode, nil
}

// Store stores an idempotency result
func (im *IdempotencyManager) Store(ctx context.Context, key string, response interface{}, statusCode int) error {
	redisKey := im.prefix + key

	result := &IdempotencyResult{
		Key:        key,
		Response:   response,
		StatusCode: statusCode,
		CreatedAt:  time.Now().UTC(),
		ExpiresAt:  time.Now().UTC().Add(im.ttl),
	}

	data, err := json.Marshal(result)
	if err != nil {
		return fmt.Errorf("failed to marshal result: %w", err)
	}

	if err := im.redis.Set(ctx, redisKey, data, im.ttl).Err(); err != nil {
		return fmt.Errorf("failed to store in redis: %w", err)
	}

	return nil
}

// Get retrieves an idempotency result
func (im *IdempotencyManager) Get(ctx context.Context, key string) (*IdempotencyResult, error) {
	redisKey := im.prefix + key

	data, err := im.redis.Get(ctx, redisKey).Bytes()
	if err != nil {
		if err == redis.Nil {
			return nil, nil // Key not found
		}
		return nil, fmt.Errorf("failed to get from redis: %w", err)
	}

	var result IdempotencyResult
	if err := json.Unmarshal(data, &result); err != nil {
		return nil, fmt.Errorf("failed to unmarshal result: %w", err)
	}

	// Check if expired
	if time.Now().UTC().After(result.ExpiresAt) {
		return nil, ErrKeyExpired
	}

	return &result, nil
}

// Delete deletes an idempotency key
func (im *IdempotencyManager) Delete(ctx context.Context, key string) error {
	redisKey := im.prefix + key
	return im.redis.Del(ctx, redisKey).Err()
}

// Exists checks if an idempotency key exists
func (im *IdempotencyManager) Exists(ctx context.Context, key string) (bool, error) {
	redisKey := im.prefix + key
	count, err := im.redis.Exists(ctx, redisKey).Result()
	if err != nil {
		return false, err
	}
	return count > 0, nil
}

// GenerateKey generates an idempotency key from request data
func GenerateKey(prefix string, data ...string) string {
	h := sha256.New()
	for _, d := range data {
		h.Write([]byte(d))
	}
	hash := hex.EncodeToString(h.Sum(nil))
	if prefix != "" {
		return fmt.Sprintf("%s-%s", prefix, hash[:16])
	}
	return hash[:16]
}

// GeneratePaymentKey generates idempotency key for payment
func GeneratePaymentKey(senderBIC, receiverBIC, amount, currency, reference string) string {
	return GenerateKey("payment", senderBIC, receiverBIC, amount, currency, reference)
}

// ProcessingLock provides distributed lock for processing
type ProcessingLock struct {
	redis  *redis.Client
	key    string
	ttl    time.Duration
	token  string
}

// AcquireLock acquires a processing lock
func (im *IdempotencyManager) AcquireLock(ctx context.Context, key string, ttl time.Duration) (*ProcessingLock, error) {
	lockKey := fmt.Sprintf("%slock:%s", im.prefix, key)
	token := GenerateKey("", key, time.Now().String())

	// Try to acquire lock (SET NX with TTL)
	success, err := im.redis.SetNX(ctx, lockKey, token, ttl).Result()
	if err != nil {
		return nil, fmt.Errorf("failed to acquire lock: %w", err)
	}

	if !success {
		return nil, fmt.Errorf("lock already held by another process")
	}

	return &ProcessingLock{
		redis: im.redis,
		key:   lockKey,
		ttl:   ttl,
		token: token,
	}, nil
}

// Release releases the processing lock
func (pl *ProcessingLock) Release(ctx context.Context) error {
	// Use Lua script to ensure we only delete our own lock
	script := `
		if redis.call("get", KEYS[1]) == ARGV[1] then
			return redis.call("del", KEYS[1])
		else
			return 0
		end
	`

	_, err := pl.redis.Eval(ctx, script, []string{pl.key}, pl.token).Result()
	if err != nil {
		return fmt.Errorf("failed to release lock: %w", err)
	}

	return nil
}

// Extend extends the lock TTL
func (pl *ProcessingLock) Extend(ctx context.Context, duration time.Duration) error {
	script := `
		if redis.call("get", KEYS[1]) == ARGV[1] then
			return redis.call("pexpire", KEYS[1], ARGV[2])
		else
			return 0
		end
	`

	_, err := pl.redis.Eval(ctx, script, []string{pl.key}, pl.token, int64(duration/time.Millisecond)).Result()
	if err != nil {
		return fmt.Errorf("failed to extend lock: %w", err)
	}

	return nil
}

// ExecuteWithLock executes function with distributed lock
func (im *IdempotencyManager) ExecuteWithLock(ctx context.Context, key string, ttl time.Duration, fn func() error) error {
	lock, err := im.AcquireLock(ctx, key, ttl)
	if err != nil {
		return err
	}

	defer func() {
		if err := lock.Release(ctx); err != nil {
			fmt.Printf("Failed to release lock: %v\n", err)
		}
	}()

	return fn()
}

// IdempotencyStats holds idempotency statistics
type IdempotencyStats struct {
	TotalKeys     int64 `json:"total_keys"`
	CacheHits     int64 `json:"cache_hits"`
	CacheMisses   int64 `json:"cache_misses"`
	HitRate       float64 `json:"hit_rate"`
}

// GetStats returns idempotency statistics (simplified)
func (im *IdempotencyManager) GetStats(ctx context.Context) (*IdempotencyStats, error) {
	// Count keys with prefix
	pattern := im.prefix + "*"
	var cursor uint64
	var totalKeys int64

	for {
		keys, nextCursor, err := im.redis.Scan(ctx, cursor, pattern, 100).Result()
		if err != nil {
			return nil, err
		}

		totalKeys += int64(len(keys))
		cursor = nextCursor

		if cursor == 0 {
			break
		}
	}

	return &IdempotencyStats{
		TotalKeys: totalKeys,
	}, nil
}
