package integration

import (
	"context"
	"database/sql"
	"errors"
	"fmt"
	"strings"

	"github.com/redis/go-redis/v9"
)

var (
	ErrDatabaseNotReady = errors.New("database is not ready")
	ErrRedisNotReady    = errors.New("redis is not ready")
	ErrSchemaMissing    = errors.New("database schema is missing")
)

// ComponentStatus represents status of a component
type ComponentStatus struct {
	Name      string `json:"name"`
	Healthy   bool   `json:"healthy"`
	Message   string `json:"message,omitempty"`
	Details   map[string]interface{} `json:"details,omitempty"`
}

// SystemHealth represents overall system health
type SystemHealth struct {
	Healthy    bool              `json:"healthy"`
	Components []ComponentStatus `json:"components"`
}

// HealthChecker validates integration between components
type HealthChecker struct {
	db    *sql.DB
	redis *redis.Client
}

// NewHealthChecker creates a new health checker
func NewHealthChecker(db *sql.DB, redis *redis.Client) *HealthChecker {
	return &HealthChecker{
		db:    db,
		redis: redis,
	}
}

// CheckDatabase validates database connection and schema
func (hc *HealthChecker) CheckDatabase(ctx context.Context) ComponentStatus {
	status := ComponentStatus{
		Name:    "postgres",
		Details: make(map[string]interface{}),
	}

	// Check connection
	if err := hc.db.PingContext(ctx); err != nil {
		status.Healthy = false
		status.Message = fmt.Sprintf("cannot ping database: %v", err)
		return status
	}

	// Check schema exists
	var schemaExists bool
	err := hc.db.QueryRowContext(ctx, `
		SELECT EXISTS (
			SELECT 1 FROM information_schema.schemata WHERE schema_name = 'deltran'
		)
	`).Scan(&schemaExists)

	if err != nil {
		status.Healthy = false
		status.Message = fmt.Sprintf("cannot check schema: %v", err)
		return status
	}

	if !schemaExists {
		status.Healthy = false
		status.Message = "deltran schema does not exist"
		return status
	}

	// Check tables exist
	requiredTables := []string{
		"users", "sessions", "banks", "bank_accounts", "payments",
		"transaction_log", "settlement_batches", "compliance_checks",
		"rate_limits", "audit_log",
	}

	var tableCount int
	placeholders := make([]string, len(requiredTables))
	args := make([]interface{}, len(requiredTables))
	for i, table := range requiredTables {
		placeholders[i] = fmt.Sprintf("$%d", i+1)
		args[i] = table
	}

	err = hc.db.QueryRowContext(ctx, fmt.Sprintf(`
		SELECT COUNT(*)
		FROM information_schema.tables
		WHERE table_schema = 'deltran'
		AND table_name IN (%s)
	`, strings.Join(placeholders, ",")), args...).Scan(&tableCount)

	if err != nil {
		status.Healthy = false
		status.Message = fmt.Sprintf("cannot check tables: %v", err)
		return status
	}

	status.Details["schema"] = "deltran"
	status.Details["tables_found"] = tableCount
	status.Details["tables_required"] = len(requiredTables)

	if tableCount != len(requiredTables) {
		status.Healthy = false
		status.Message = fmt.Sprintf("missing tables: found %d, required %d", tableCount, len(requiredTables))
		return status
	}

	// Check custom types exist
	var typeCount int
	err = hc.db.QueryRowContext(ctx, `
		SELECT COUNT(*)
		FROM pg_type t
		JOIN pg_namespace n ON n.oid = t.typnamespace
		WHERE n.nspname = 'deltran'
		AND t.typname IN ('user_role', 'payment_status', 'settlement_status', 'compliance_status', 'currency_code', 'action_type')
	`).Scan(&typeCount)

	if err != nil {
		status.Healthy = false
		status.Message = fmt.Sprintf("cannot check types: %v", err)
		return status
	}

	status.Details["types_found"] = typeCount
	status.Details["types_required"] = 6

	if typeCount != 6 {
		status.Healthy = false
		status.Message = fmt.Sprintf("missing custom types: found %d, required 6", typeCount)
		return status
	}

	status.Healthy = true
	status.Message = "database is healthy"
	return status
}

// CheckRedis validates Redis connection
func (hc *HealthChecker) CheckRedis(ctx context.Context) ComponentStatus {
	status := ComponentStatus{
		Name:    "redis",
		Details: make(map[string]interface{}),
	}

	// Check connection
	if err := hc.redis.Ping(ctx).Err(); err != nil {
		status.Healthy = false
		status.Message = fmt.Sprintf("cannot ping redis: %v", err)
		return status
	}

	// Get Redis info
	var err error
	_, err = hc.redis.Info(ctx, "server").Result()
	if err != nil {
		status.Healthy = false
		status.Message = fmt.Sprintf("cannot get redis info: %v", err)
		return status
	}

	// Check memory usage
	_, err = hc.redis.Info(ctx, "memory").Result()
	if err == nil {
		status.Details["memory_info"] = "available"
	}

	status.Details["server_info"] = "connected"
	status.Healthy = true
	status.Message = "redis is healthy"
	return status
}

// CheckSystemHealth performs full system health check
func (hc *HealthChecker) CheckSystemHealth(ctx context.Context) SystemHealth {
	components := []ComponentStatus{
		hc.CheckDatabase(ctx),
		hc.CheckRedis(ctx),
	}

	healthy := true
	for _, comp := range components {
		if !comp.Healthy {
			healthy = false
			break
		}
	}

	return SystemHealth{
		Healthy:    healthy,
		Components: components,
	}
}

// ValidateAuthIntegration validates auth components integration
func (hc *HealthChecker) ValidateAuthIntegration(ctx context.Context) error {
	// Check users table has correct columns
	var columnCount int
	err := hc.db.QueryRowContext(ctx, `
		SELECT COUNT(*)
		FROM information_schema.columns
		WHERE table_schema = 'deltran'
		AND table_name = 'users'
		AND column_name IN (
			'id', 'email', 'username', 'password_hash', 'role',
			'is_active', 'is_2fa_enabled', 'totp_secret',
			'failed_login_attempts', 'locked_until',
			'last_login_at', 'last_login_ip'
		)
	`).Scan(&columnCount)

	if err != nil {
		return fmt.Errorf("cannot check users table: %w", err)
	}

	if columnCount != 12 {
		return fmt.Errorf("users table missing required columns: found %d, required 12", columnCount)
	}

	// Check sessions table
	err = hc.db.QueryRowContext(ctx, `
		SELECT COUNT(*)
		FROM information_schema.columns
		WHERE table_schema = 'deltran'
		AND table_name = 'sessions'
		AND column_name IN (
			'id', 'user_id', 'refresh_token_hash',
			'ip_address', 'user_agent', 'device_fingerprint',
			'created_at', 'expires_at', 'last_activity_at',
			'is_revoked', 'revoked_at', 'revoke_reason'
		)
	`).Scan(&columnCount)

	if err != nil {
		return fmt.Errorf("cannot check sessions table: %w", err)
	}

	if columnCount != 12 {
		return fmt.Errorf("sessions table missing required columns: found %d, required 12", columnCount)
	}

	// Verify foreign key exists
	var fkExists bool
	err = hc.db.QueryRowContext(ctx, `
		SELECT EXISTS (
			SELECT 1
			FROM information_schema.table_constraints
			WHERE table_schema = 'deltran'
			AND table_name = 'sessions'
			AND constraint_type = 'FOREIGN KEY'
		)
	`).Scan(&fkExists)

	if err != nil {
		return fmt.Errorf("cannot check foreign keys: %w", err)
	}

	if !fkExists {
		return errors.New("sessions table missing foreign key to users")
	}

	return nil
}

// ValidatePaymentIntegration validates payment components integration
func (hc *HealthChecker) ValidatePaymentIntegration(ctx context.Context) error {
	// Check payments table has SWIFT fields
	var columnCount int
	err := hc.db.QueryRowContext(ctx, `
		SELECT COUNT(*)
		FROM information_schema.columns
		WHERE table_schema = 'deltran'
		AND table_name = 'payments'
		AND column_name IN ('swift_message_type', 'swift_message_id', 'idempotency_key')
	`).Scan(&columnCount)

	if err != nil {
		return fmt.Errorf("cannot check payments table: %w", err)
	}

	if columnCount != 3 {
		return fmt.Errorf("payments table missing SWIFT/idempotency columns: found %d, required 3", columnCount)
	}

	// Check foreign keys to banks
	var fkCount int
	err = hc.db.QueryRowContext(ctx, `
		SELECT COUNT(*)
		FROM information_schema.table_constraints tc
		JOIN information_schema.constraint_column_usage ccu ON tc.constraint_name = ccu.constraint_name
		WHERE tc.table_schema = 'deltran'
		AND tc.table_name = 'payments'
		AND tc.constraint_type = 'FOREIGN KEY'
		AND ccu.table_name = 'banks'
	`).Scan(&fkCount)

	if err != nil {
		return fmt.Errorf("cannot check foreign keys: %w", err)
	}

	if fkCount < 2 {
		return errors.New("payments table missing foreign keys to banks (sender/receiver)")
	}

	return nil
}

// ValidateIndexes checks if critical indexes exist
func (hc *HealthChecker) ValidateIndexes(ctx context.Context) error {
	criticalIndexes := map[string][]string{
		"users": {"idx_users_email", "idx_users_username"},
		"sessions": {"idx_sessions_user_id", "idx_sessions_token_hash"},
		"payments": {"idx_payments_reference", "idx_payments_idempotency", "idx_payments_status"},
		"banks": {"idx_banks_bic"},
	}

	for table, indexes := range criticalIndexes {
		for _, index := range indexes {
			var exists bool
			err := hc.db.QueryRowContext(ctx, `
				SELECT EXISTS (
					SELECT 1
					FROM pg_indexes
					WHERE schemaname = 'deltran'
					AND tablename = $1
					AND indexname = $2
				)
			`, table, index).Scan(&exists)

			if err != nil {
				return fmt.Errorf("cannot check index %s: %w", index, err)
			}

			if !exists {
				return fmt.Errorf("critical index missing: %s on table %s", index, table)
			}
		}
	}

	return nil
}

// FullValidation performs complete system validation
func (hc *HealthChecker) FullValidation(ctx context.Context) []error {
	var errs []error

	// Check basic connectivity
	health := hc.CheckSystemHealth(ctx)
	if !health.Healthy {
		for _, comp := range health.Components {
			if !comp.Healthy {
				errs = append(errs, fmt.Errorf("%s: %s", comp.Name, comp.Message))
			}
		}
		return errs
	}

	// Validate auth integration
	if err := hc.ValidateAuthIntegration(ctx); err != nil {
		errs = append(errs, fmt.Errorf("auth integration: %w", err))
	}

	// Validate payment integration
	if err := hc.ValidatePaymentIntegration(ctx); err != nil {
		errs = append(errs, fmt.Errorf("payment integration: %w", err))
	}

	// Validate indexes
	if err := hc.ValidateIndexes(ctx); err != nil {
		errs = append(errs, fmt.Errorf("indexes: %w", err))
	}

	return errs
}
