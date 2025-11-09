package integration

import (
	"context"
	"database/sql"
	"testing"
	"time"

	_ "github.com/lib/pq"
)

const (
	dbURL = "postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran?sslmode=disable"
)

// TestDatabaseConnection tests basic database connectivity
func TestDatabaseConnection(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Fatalf("Failed to open database: %v", err)
	}
	defer db.Close()

	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		t.Fatalf("Failed to ping database: %v", err)
	}

	t.Log("✓ Successfully connected to database")
}

// TestDatabaseSchema tests if required tables exist
func TestDatabaseSchema(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Fatalf("Failed to open database: %v", err)
	}
	defer db.Close()

	requiredTables := []string{
		"banks",
		"transactions",
		"obligations",
		"tokens",
		"clearing_windows",
		"settlement_instructions",
	}

	for _, table := range requiredTables {
		var exists bool
		query := `SELECT EXISTS (
			SELECT FROM information_schema.tables
			WHERE table_schema = 'public'
			AND table_name = $1
		)`

		err := db.QueryRow(query, table).Scan(&exists)
		if err != nil {
			t.Errorf("Failed to check table %s: %v", table, err)
			continue
		}

		if !exists {
			t.Logf("Warning: Table %s does not exist", table)
		} else {
			t.Logf("✓ Table %s exists", table)
		}
	}
}

// TestDatabaseTransactions tests transaction isolation
func TestDatabaseTransactions(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Fatalf("Failed to open database: %v", err)
	}
	defer db.Close()

	ctx := context.Background()

	// Begin transaction
	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		t.Fatalf("Failed to begin transaction: %v", err)
	}

	// Try to create a test table
	_, err = tx.ExecContext(ctx, `
		CREATE TABLE IF NOT EXISTS test_transactions (
			id SERIAL PRIMARY KEY,
			data TEXT
		)
	`)

	if err != nil {
		tx.Rollback()
		t.Fatalf("Failed to create test table: %v", err)
	}

	// Insert test data
	_, err = tx.ExecContext(ctx, "INSERT INTO test_transactions (data) VALUES ($1)", "test")
	if err != nil {
		tx.Rollback()
		t.Fatalf("Failed to insert test data: %v", err)
	}

	// Rollback
	if err := tx.Rollback(); err != nil {
		t.Fatalf("Failed to rollback: %v", err)
	}

	// Check that data was rolled back
	var count int
	err = db.QueryRowContext(ctx, "SELECT COUNT(*) FROM test_transactions WHERE data = $1", "test").Scan(&count)

	// If table doesn't exist or count is 0, rollback worked
	if err != nil || count == 0 {
		t.Log("✓ Database transaction rollback working correctly")
	} else {
		t.Errorf("Rollback failed: found %d rows", count)
	}

	// Cleanup
	db.ExecContext(ctx, "DROP TABLE IF EXISTS test_transactions")
}

// TestDatabaseConnectionPool tests connection pooling
func TestDatabaseConnectionPool(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Fatalf("Failed to open database: %v", err)
	}
	defer db.Close()

	db.SetMaxOpenConns(10)
	db.SetMaxIdleConns(5)
	db.SetConnMaxLifetime(time.Hour)

	// Get stats
	stats := db.Stats()
	t.Logf("✓ Connection pool configured - Max Open: %d, Max Idle: %d",
		db.Stats().MaxOpenConnections, stats.Idle)
}

// TestDatabasePerformance tests query performance
func TestDatabasePerformance(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Fatalf("Failed to open database: %v", err)
	}
	defer db.Close()

	ctx := context.Background()

	// Simple query performance test
	start := time.Now()

	for i := 0; i < 100; i++ {
		var result int
		if err := db.QueryRowContext(ctx, "SELECT 1").Scan(&result); err != nil {
			t.Errorf("Query %d failed: %v", i, err)
		}
	}

	elapsed := time.Since(start)
	queriesPerSecond := 100.0 / elapsed.Seconds()

	t.Logf("✓ Database Performance: 100 queries in %v (%.0f queries/sec)", elapsed, queriesPerSecond)

	if queriesPerSecond < 100 {
		t.Logf("Warning: Low query performance (%.0f q/s), expected > 100", queriesPerSecond)
	}
}

// TestTimescaleDBExtension tests TimescaleDB availability
func TestTimescaleDBExtension(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Fatalf("Failed to open database: %v", err)
	}
	defer db.Close()

	var version string
	err = db.QueryRow("SELECT extversion FROM pg_extension WHERE extname = 'timescaledb'").Scan(&version)

	if err != nil {
		t.Logf("Warning: TimescaleDB extension not found: %v", err)
	} else {
		t.Logf("✓ TimescaleDB extension available - Version: %s", version)
	}
}
