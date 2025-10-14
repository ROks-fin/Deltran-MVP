package database

import (
	"context"
	"database/sql"
	"fmt"
	"time"

	_ "github.com/lib/pq"
)

// PostgresConfig holds database configuration
type PostgresConfig struct {
	Host            string
	Port            int
	Database        string
	User            string
	Password        string
	SSLMode         string
	MaxOpenConns    int
	MaxIdleConns    int
	ConnMaxLifetime time.Duration
	ConnMaxIdleTime time.Duration
}

// PostgresDB wraps sql.DB with DelTran-specific methods
type PostgresDB struct {
	db     *sql.DB
	config PostgresConfig
}

// NewPostgresDB creates a new PostgreSQL connection pool
func NewPostgresDB(config PostgresConfig) (*PostgresDB, error) {
	// Build connection string
	connStr := fmt.Sprintf(
		"host=%s port=%d user=%s password=%s dbname=%s sslmode=%s",
		config.Host,
		config.Port,
		config.User,
		config.Password,
		config.Database,
		config.SSLMode,
	)

	// Open connection
	db, err := sql.Open("postgres", connStr)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	// Configure connection pool
	db.SetMaxOpenConns(config.MaxOpenConns)
	db.SetMaxIdleConns(config.MaxIdleConns)
	db.SetConnMaxLifetime(config.ConnMaxLifetime)
	db.SetConnMaxIdleTime(config.ConnMaxIdleTime)

	// Verify connection
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	return &PostgresDB{
		db:     db,
		config: config,
	}, nil
}

// Close closes the database connection
func (p *PostgresDB) Close() error {
	return p.db.Close()
}

// DB returns the underlying sql.DB for custom queries
func (p *PostgresDB) DB() *sql.DB {
	return p.db
}

// Ping checks if database is alive
func (p *PostgresDB) Ping(ctx context.Context) error {
	return p.db.PingContext(ctx)
}

// GetStats returns connection pool statistics
func (p *PostgresDB) GetStats() sql.DBStats {
	return p.db.Stats()
}

// BeginTx starts a transaction
func (p *PostgresDB) BeginTx(ctx context.Context, opts *sql.TxOptions) (*sql.Tx, error) {
	return p.db.BeginTx(ctx, opts)
}

// ExecTx executes a function within a transaction
func (p *PostgresDB) ExecTx(ctx context.Context, fn func(*sql.Tx) error) error {
	tx, err := p.db.BeginTx(ctx, nil)
	if err != nil {
		return err
	}

	err = fn(tx)
	if err != nil {
		if rbErr := tx.Rollback(); rbErr != nil {
			return fmt.Errorf("tx error: %v, rollback error: %v", err, rbErr)
		}
		return err
	}

	return tx.Commit()
}

// ========================================
// USER QUERIES
// ========================================

type User struct {
	ID                   string    `json:"id"`
	Email                string    `json:"email"`
	Username             string    `json:"username"`
	PasswordHash         string    `json:"-"`
	Role                 string    `json:"role"`
	BankID               *string   `json:"bank_id,omitempty"`
	IsActive             bool      `json:"is_active"`
	Is2FAEnabled         bool      `json:"is_2fa_enabled"`
	TOTPSecret           *string   `json:"-"`
	CreatedAt            time.Time `json:"created_at"`
	UpdatedAt            time.Time `json:"updated_at"`
	LastLoginAt          *time.Time `json:"last_login_at,omitempty"`
	LastLoginIP          *string   `json:"last_login_ip,omitempty"`
	FailedLoginAttempts  int       `json:"failed_login_attempts"`
	LockedUntil          *time.Time `json:"locked_until,omitempty"`
}

// GetUserByEmail retrieves a user by email
func (p *PostgresDB) GetUserByEmail(ctx context.Context, email string) (*User, error) {
	query := `
		SELECT id, email, username, password_hash, role, bank_id, is_active,
		       is_2fa_enabled, totp_secret, created_at, updated_at, last_login_at,
		       last_login_ip, failed_login_attempts, locked_until
		FROM deltran.users
		WHERE email = $1 AND is_active = true
	`

	var user User
	err := p.db.QueryRowContext(ctx, query, email).Scan(
		&user.ID, &user.Email, &user.Username, &user.PasswordHash, &user.Role,
		&user.BankID, &user.IsActive, &user.Is2FAEnabled, &user.TOTPSecret,
		&user.CreatedAt, &user.UpdatedAt, &user.LastLoginAt, &user.LastLoginIP,
		&user.FailedLoginAttempts, &user.LockedUntil,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("user not found")
	}
	if err != nil {
		return nil, err
	}

	return &user, nil
}

// GetUserByID retrieves a user by ID
func (p *PostgresDB) GetUserByID(ctx context.Context, id string) (*User, error) {
	query := `
		SELECT id, email, username, password_hash, role, bank_id, is_active,
		       is_2fa_enabled, totp_secret, created_at, updated_at, last_login_at,
		       last_login_ip, failed_login_attempts, locked_until
		FROM deltran.users
		WHERE id = $1
	`

	var user User
	err := p.db.QueryRowContext(ctx, query, id).Scan(
		&user.ID, &user.Email, &user.Username, &user.PasswordHash, &user.Role,
		&user.BankID, &user.IsActive, &user.Is2FAEnabled, &user.TOTPSecret,
		&user.CreatedAt, &user.UpdatedAt, &user.LastLoginAt, &user.LastLoginIP,
		&user.FailedLoginAttempts, &user.LockedUntil,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("user not found")
	}
	if err != nil {
		return nil, err
	}

	return &user, nil
}

// UpdateLastLogin updates user's last login timestamp and IP
func (p *PostgresDB) UpdateLastLogin(ctx context.Context, userID, ipAddress string) error {
	query := `
		UPDATE deltran.users
		SET last_login_at = NOW(),
		    last_login_ip = $2,
		    failed_login_attempts = 0
		WHERE id = $1
	`

	_, err := p.db.ExecContext(ctx, query, userID, ipAddress)
	return err
}

// IncrementFailedLogins increments failed login attempts
func (p *PostgresDB) IncrementFailedLogins(ctx context.Context, userID string) error {
	query := `
		UPDATE deltran.users
		SET failed_login_attempts = failed_login_attempts + 1,
		    locked_until = CASE
		        WHEN failed_login_attempts + 1 >= 5
		        THEN NOW() + INTERVAL '15 minutes'
		        ELSE locked_until
		    END
		WHERE id = $1
	`

	_, err := p.db.ExecContext(ctx, query, userID)
	return err
}

// ========================================
// BANK QUERIES
// ========================================

type Bank struct {
	ID             string     `json:"id"`
	BICCode        string     `json:"bic_code"`
	Name           string     `json:"name"`
	CountryCode    string     `json:"country_code"`
	IsActive       bool       `json:"is_active"`
	OnboardedAt    time.Time  `json:"onboarded_at"`
	RiskRating     *string    `json:"risk_rating,omitempty"`
	KYCStatus      string     `json:"kyc_status"`
	KYCVerifiedAt  *time.Time `json:"kyc_verified_at,omitempty"`
	ContactEmail   *string    `json:"contact_email,omitempty"`
	ContactPhone   *string    `json:"contact_phone,omitempty"`
	CreatedAt      time.Time  `json:"created_at"`
	UpdatedAt      time.Time  `json:"updated_at"`
}

// GetBankByBIC retrieves a bank by BIC code
func (p *PostgresDB) GetBankByBIC(ctx context.Context, bicCode string) (*Bank, error) {
	query := `
		SELECT id, bic_code, name, country_code, is_active, onboarded_at,
		       risk_rating, kyc_status, kyc_verified_at, contact_email,
		       contact_phone, created_at, updated_at
		FROM deltran.banks
		WHERE bic_code = $1
	`

	var bank Bank
	err := p.db.QueryRowContext(ctx, query, bicCode).Scan(
		&bank.ID, &bank.BICCode, &bank.Name, &bank.CountryCode, &bank.IsActive,
		&bank.OnboardedAt, &bank.RiskRating, &bank.KYCStatus, &bank.KYCVerifiedAt,
		&bank.ContactEmail, &bank.ContactPhone, &bank.CreatedAt, &bank.UpdatedAt,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("bank not found")
	}
	if err != nil {
		return nil, err
	}

	return &bank, nil
}

// ListActiveBanks retrieves all active banks
func (p *PostgresDB) ListActiveBanks(ctx context.Context) ([]*Bank, error) {
	query := `
		SELECT id, bic_code, name, country_code, is_active, onboarded_at,
		       risk_rating, kyc_status, kyc_verified_at, contact_email,
		       contact_phone, created_at, updated_at
		FROM deltran.banks
		WHERE is_active = true
		ORDER BY name ASC
	`

	rows, err := p.db.QueryContext(ctx, query)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var banks []*Bank
	for rows.Next() {
		var bank Bank
		err := rows.Scan(
			&bank.ID, &bank.BICCode, &bank.Name, &bank.CountryCode, &bank.IsActive,
			&bank.OnboardedAt, &bank.RiskRating, &bank.KYCStatus, &bank.KYCVerifiedAt,
			&bank.ContactEmail, &bank.ContactPhone, &bank.CreatedAt, &bank.UpdatedAt,
		)
		if err != nil {
			return nil, err
		}
		banks = append(banks, &bank)
	}

	return banks, rows.Err()
}

// ========================================
// PAYMENT QUERIES
// ========================================

type Payment struct {
	ID                 string     `json:"id"`
	PaymentReference   string     `json:"payment_reference"`
	SenderBankID       string     `json:"sender_bank_id"`
	ReceiverBankID     string     `json:"receiver_bank_id"`
	SenderAccountID    *string    `json:"sender_account_id,omitempty"`
	ReceiverAccountID  *string    `json:"receiver_account_id,omitempty"`
	Amount             float64    `json:"amount"`
	Currency           string     `json:"currency"`
	Status             string     `json:"status"`
	ComplianceCheckID  *string    `json:"compliance_check_id,omitempty"`
	RiskScore          *float64   `json:"risk_score,omitempty"`
	BatchID            *string    `json:"batch_id,omitempty"`
	SWIFTMessageType   *string    `json:"swift_message_type,omitempty"`
	SWIFTMessageID     *string    `json:"swift_message_id,omitempty"`
	IdempotencyKey     *string    `json:"idempotency_key,omitempty"`
	CreatedAt          time.Time  `json:"created_at"`
	UpdatedAt          time.Time  `json:"updated_at"`
	ProcessedAt        *time.Time `json:"processed_at,omitempty"`
	SettledAt          *time.Time `json:"settled_at,omitempty"`
	RemittanceInfo     *string    `json:"remittance_info,omitempty"`
}

// CreatePayment inserts a new payment
func (p *PostgresDB) CreatePayment(ctx context.Context, payment *Payment) error {
	query := `
		INSERT INTO deltran.payments (
			payment_reference, sender_bank_id, receiver_bank_id, sender_account_id,
			receiver_account_id, amount, currency, status, idempotency_key,
			remittance_info, swift_message_type
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
		RETURNING id, created_at, updated_at
	`

	err := p.db.QueryRowContext(ctx, query,
		payment.PaymentReference, payment.SenderBankID, payment.ReceiverBankID,
		payment.SenderAccountID, payment.ReceiverAccountID, payment.Amount,
		payment.Currency, payment.Status, payment.IdempotencyKey,
		payment.RemittanceInfo, payment.SWIFTMessageType,
	).Scan(&payment.ID, &payment.CreatedAt, &payment.UpdatedAt)

	return err
}

// GetPaymentByID retrieves a payment by ID
func (p *PostgresDB) GetPaymentByID(ctx context.Context, id string) (*Payment, error) {
	query := `
		SELECT id, payment_reference, sender_bank_id, receiver_bank_id,
		       sender_account_id, receiver_account_id, amount, currency,
		       status, compliance_check_id, risk_score, batch_id,
		       swift_message_type, swift_message_id, idempotency_key,
		       created_at, updated_at, processed_at, settled_at, remittance_info
		FROM deltran.payments
		WHERE id = $1
	`

	var payment Payment
	err := p.db.QueryRowContext(ctx, query, id).Scan(
		&payment.ID, &payment.PaymentReference, &payment.SenderBankID,
		&payment.ReceiverBankID, &payment.SenderAccountID, &payment.ReceiverAccountID,
		&payment.Amount, &payment.Currency, &payment.Status, &payment.ComplianceCheckID,
		&payment.RiskScore, &payment.BatchID, &payment.SWIFTMessageType,
		&payment.SWIFTMessageID, &payment.IdempotencyKey, &payment.CreatedAt,
		&payment.UpdatedAt, &payment.ProcessedAt, &payment.SettledAt,
		&payment.RemittanceInfo,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("payment not found")
	}
	if err != nil {
		return nil, err
	}

	return &payment, nil
}

// GetPaymentByReference retrieves a payment by reference
func (p *PostgresDB) GetPaymentByReference(ctx context.Context, reference string) (*Payment, error) {
	query := `
		SELECT id, payment_reference, sender_bank_id, receiver_bank_id,
		       sender_account_id, receiver_account_id, amount, currency,
		       status, compliance_check_id, risk_score, batch_id,
		       swift_message_type, swift_message_id, idempotency_key,
		       created_at, updated_at, processed_at, settled_at, remittance_info
		FROM deltran.payments
		WHERE payment_reference = $1
	`

	var payment Payment
	err := p.db.QueryRowContext(ctx, query, reference).Scan(
		&payment.ID, &payment.PaymentReference, &payment.SenderBankID,
		&payment.ReceiverBankID, &payment.SenderAccountID, &payment.ReceiverAccountID,
		&payment.Amount, &payment.Currency, &payment.Status, &payment.ComplianceCheckID,
		&payment.RiskScore, &payment.BatchID, &payment.SWIFTMessageType,
		&payment.SWIFTMessageID, &payment.IdempotencyKey, &payment.CreatedAt,
		&payment.UpdatedAt, &payment.ProcessedAt, &payment.SettledAt,
		&payment.RemittanceInfo,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("payment not found")
	}
	if err != nil {
		return nil, err
	}

	return &payment, nil
}

// UpdatePaymentStatus updates the status of a payment
func (p *PostgresDB) UpdatePaymentStatus(ctx context.Context, id, status string) error {
	query := `
		UPDATE deltran.payments
		SET status = $2,
		    processed_at = CASE WHEN $2 = 'processing' THEN NOW() ELSE processed_at END,
		    settled_at = CASE WHEN $2 = 'settled' THEN NOW() ELSE settled_at END
		WHERE id = $1
	`

	result, err := p.db.ExecContext(ctx, query, id, status)
	if err != nil {
		return err
	}

	rows, err := result.RowsAffected()
	if err != nil {
		return err
	}

	if rows == 0 {
		return fmt.Errorf("payment not found")
	}

	return nil
}

// ListPendingPayments retrieves all pending payments
func (p *PostgresDB) ListPendingPayments(ctx context.Context, limit int) ([]*Payment, error) {
	query := `
		SELECT id, payment_reference, sender_bank_id, receiver_bank_id,
		       sender_account_id, receiver_account_id, amount, currency,
		       status, compliance_check_id, risk_score, batch_id,
		       swift_message_type, swift_message_id, idempotency_key,
		       created_at, updated_at, processed_at, settled_at, remittance_info
		FROM deltran.payments
		WHERE status = 'pending'
		ORDER BY created_at ASC
		LIMIT $1
	`

	rows, err := p.db.QueryContext(ctx, query, limit)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var payments []*Payment
	for rows.Next() {
		var payment Payment
		err := rows.Scan(
			&payment.ID, &payment.PaymentReference, &payment.SenderBankID,
			&payment.ReceiverBankID, &payment.SenderAccountID, &payment.ReceiverAccountID,
			&payment.Amount, &payment.Currency, &payment.Status, &payment.ComplianceCheckID,
			&payment.RiskScore, &payment.BatchID, &payment.SWIFTMessageType,
			&payment.SWIFTMessageID, &payment.IdempotencyKey, &payment.CreatedAt,
			&payment.UpdatedAt, &payment.ProcessedAt, &payment.SettledAt,
			&payment.RemittanceInfo,
		)
		if err != nil {
			return nil, err
		}
		payments = append(payments, &payment)
	}

	return payments, rows.Err()
}

// ========================================
// AUDIT LOG
// ========================================

type AuditLog struct {
	ID            int64
	EventID       string
	EventType     string
	Severity      string
	ActorID       *string
	ActorType     *string
	ActorName     *string
	Action        string
	ResourceType  *string
	ResourceID    *string
	Result        string
	ErrorMessage  *string
	IPAddress     *string
	UserAgent     *string
	RequestID     *string
	Timestamp     time.Time
}

// CreateAuditLog inserts a new audit log entry
func (p *PostgresDB) CreateAuditLog(ctx context.Context, log *AuditLog) error {
	query := `
		INSERT INTO deltran.audit_log (
			event_type, severity, actor_id, actor_type, actor_name,
			action, resource_type, resource_id, result, error_message,
			ip_address, user_agent, request_id
		) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11, $12, $13)
		RETURNING id, event_id, timestamp
	`

	err := p.db.QueryRowContext(ctx, query,
		log.EventType, log.Severity, log.ActorID, log.ActorType, log.ActorName,
		log.Action, log.ResourceType, log.ResourceID, log.Result, log.ErrorMessage,
		log.IPAddress, log.UserAgent, log.RequestID,
	).Scan(&log.ID, &log.EventID, &log.Timestamp)

	return err
}

// ========================================
// HEALTH CHECK
// ========================================

// HealthCheck performs a comprehensive database health check
func (p *PostgresDB) HealthCheck(ctx context.Context) error {
	// Check connection
	if err := p.Ping(ctx); err != nil {
		return fmt.Errorf("ping failed: %w", err)
	}

	// Check if can query
	var count int
	err := p.db.QueryRowContext(ctx, "SELECT COUNT(*) FROM deltran.users").Scan(&count)
	if err != nil {
		return fmt.Errorf("query failed: %w", err)
	}

	// Check pool stats
	stats := p.GetStats()
	if stats.OpenConnections == 0 {
		return fmt.Errorf("no open connections")
	}

	return nil
}
