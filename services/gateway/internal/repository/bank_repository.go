package repository

import (
	"context"
	"database/sql"
	"fmt"

	_ "github.com/lib/pq"
)

// Bank represents a bank in the system
type Bank struct {
	ID        string
	BankCode  string
	BankName  string
	Country   string
	BICCode   string
	Status    string
}

// BankRepository handles bank data access
type BankRepository struct {
	db *sql.DB
}

// NewBankRepository creates a new bank repository
func NewBankRepository(dbURL string) (*BankRepository, error) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to database: %w", err)
	}

	// Test connection
	if err := db.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	return &BankRepository{db: db}, nil
}

// GetBankByCode retrieves a bank by its code
func (r *BankRepository) GetBankByCode(ctx context.Context, bankCode string) (*Bank, error) {
	query := `
		SELECT id, bank_code, bank_name, country, bic_code, status
		FROM banks
		WHERE bank_code = $1 AND status = 'Active'
	`

	var bank Bank
	err := r.db.QueryRowContext(ctx, query, bankCode).Scan(
		&bank.ID,
		&bank.BankCode,
		&bank.BankName,
		&bank.Country,
		&bank.BICCode,
		&bank.Status,
	)

	if err == sql.ErrNoRows {
		return nil, fmt.Errorf("bank not found: %s", bankCode)
	}

	if err != nil {
		return nil, fmt.Errorf("database error: %w", err)
	}

	return &bank, nil
}

// GetAllActiveBanks retrieves all active banks
func (r *BankRepository) GetAllActiveBanks(ctx context.Context) ([]Bank, error) {
	query := `
		SELECT id, bank_code, bank_name, country, bic_code, status
		FROM banks
		WHERE status = 'Active'
		ORDER BY bank_name
	`

	rows, err := r.db.QueryContext(ctx, query)
	if err != nil {
		return nil, fmt.Errorf("database error: %w", err)
	}
	defer rows.Close()

	var banks []Bank
	for rows.Next() {
		var bank Bank
		err := rows.Scan(
			&bank.ID,
			&bank.BankCode,
			&bank.BankName,
			&bank.Country,
			&bank.BICCode,
			&bank.Status,
		)
		if err != nil {
			return nil, fmt.Errorf("scan error: %w", err)
		}
		banks = append(banks, bank)
	}

	if err = rows.Err(); err != nil {
		return nil, fmt.Errorf("rows error: %w", err)
	}

	return banks, nil
}

// Close closes the database connection
func (r *BankRepository) Close() error {
	return r.db.Close()
}
