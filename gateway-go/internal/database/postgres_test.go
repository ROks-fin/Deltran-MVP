package database

import (
	"context"
	"testing"
	"time"

	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"
)

// setupTestDB creates a test database connection
func setupTestDB(t *testing.T) *PostgresDB {
	config := PostgresConfig{
		Host:            "localhost",
		Port:            5432,
		Database:        "deltran_test",
		User:            "deltran_app",
		Password:        "changeme123",
		SSLMode:         "disable",
		MaxOpenConns:    25,
		MaxIdleConns:    5,
		ConnMaxLifetime: 5 * time.Minute,
		ConnMaxIdleTime: 1 * time.Minute,
	}

	db, err := NewPostgresDB(config)
	require.NoError(t, err, "Failed to connect to test database")

	return db
}

func TestPostgresConnection(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	db := setupTestDB(t)
	defer db.Close()

	ctx := context.Background()

	t.Run("Ping Database", func(t *testing.T) {
		err := db.Ping(ctx)
		assert.NoError(t, err)
	})

	t.Run("Check Connection Pool", func(t *testing.T) {
		stats := db.GetStats()
		assert.GreaterOrEqual(t, stats.MaxOpenConnections, 1)
	})

	t.Run("Health Check", func(t *testing.T) {
		err := db.HealthCheck(ctx)
		assert.NoError(t, err)
	})
}

func TestUserQueries(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	db := setupTestDB(t)
	defer db.Close()

	ctx := context.Background()

	t.Run("Get User By Email", func(t *testing.T) {
		user, err := db.GetUserByEmail(ctx, "admin@deltran.local")
		require.NoError(t, err)
		assert.NotNil(t, user)
		assert.Equal(t, "admin@deltran.local", user.Email)
		assert.Equal(t, "admin", user.Role)
		assert.True(t, user.IsActive)
	})

	t.Run("Get User By ID", func(t *testing.T) {
		// First get user by email to get ID
		user, err := db.GetUserByEmail(ctx, "admin@deltran.local")
		require.NoError(t, err)

		// Then get by ID
		userByID, err := db.GetUserByID(ctx, user.ID)
		require.NoError(t, err)
		assert.Equal(t, user.Email, userByID.Email)
	})

	t.Run("Get Non-existent User", func(t *testing.T) {
		_, err := db.GetUserByEmail(ctx, "nonexistent@example.com")
		assert.Error(t, err)
	})

	t.Run("Update Last Login", func(t *testing.T) {
		user, err := db.GetUserByEmail(ctx, "admin@deltran.local")
		require.NoError(t, err)

		err = db.UpdateLastLogin(ctx, user.ID, "192.168.1.1")
		assert.NoError(t, err)

		// Verify update
		updated, err := db.GetUserByID(ctx, user.ID)
		require.NoError(t, err)
		assert.NotNil(t, updated.LastLoginAt)
		assert.NotNil(t, updated.LastLoginIP)
		assert.Equal(t, "192.168.1.1", *updated.LastLoginIP)
	})

	t.Run("Increment Failed Logins", func(t *testing.T) {
		user, err := db.GetUserByEmail(ctx, "admin@deltran.local")
		require.NoError(t, err)

		initialAttempts := user.FailedLoginAttempts

		err = db.IncrementFailedLogins(ctx, user.ID)
		assert.NoError(t, err)

		// Verify increment
		updated, err := db.GetUserByID(ctx, user.ID)
		require.NoError(t, err)
		assert.Equal(t, initialAttempts+1, updated.FailedLoginAttempts)
	})
}

func TestBankQueries(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	db := setupTestDB(t)
	defer db.Close()

	ctx := context.Background()

	t.Run("Get Bank By BIC", func(t *testing.T) {
		bank, err := db.GetBankByBIC(ctx, "CHASUS33XXX")
		require.NoError(t, err)
		assert.NotNil(t, bank)
		assert.Equal(t, "CHASUS33XXX", bank.BICCode)
		assert.Contains(t, bank.Name, "JPMorgan")
		assert.True(t, bank.IsActive)
	})

	t.Run("List Active Banks", func(t *testing.T) {
		banks, err := db.ListActiveBanks(ctx)
		require.NoError(t, err)
		assert.NotEmpty(t, banks)

		// Verify all are active
		for _, bank := range banks {
			assert.True(t, bank.IsActive)
		}
	})

	t.Run("Get Non-existent Bank", func(t *testing.T) {
		_, err := db.GetBankByBIC(ctx, "INVALID000")
		assert.Error(t, err)
	})
}

func TestPaymentQueries(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	db := setupTestDB(t)
	defer db.Close()

	ctx := context.Background()

	// Get test banks
	senderBank, err := db.GetBankByBIC(ctx, "CHASUS33XXX")
	require.NoError(t, err)

	receiverBank, err := db.GetBankByBIC(ctx, "DEUTDEFFXXX")
	require.NoError(t, err)

	t.Run("Create Payment", func(t *testing.T) {
		payment := &Payment{
			PaymentReference: "TEST-" + time.Now().Format("20060102150405"),
			SenderBankID:     senderBank.ID,
			ReceiverBankID:   receiverBank.ID,
			Amount:           1000.00,
			Currency:         "USD",
			Status:           "pending",
			RemittanceInfo:   stringPtr("Test payment"),
		}

		err := db.CreatePayment(ctx, payment)
		require.NoError(t, err)
		assert.NotEmpty(t, payment.ID)
		assert.NotZero(t, payment.CreatedAt)
	})

	t.Run("Get Payment By ID", func(t *testing.T) {
		// Create payment first
		payment := &Payment{
			PaymentReference: "TEST-" + time.Now().Format("20060102150405"),
			SenderBankID:     senderBank.ID,
			ReceiverBankID:   receiverBank.ID,
			Amount:           2000.00,
			Currency:         "EUR",
			Status:           "pending",
		}

		err := db.CreatePayment(ctx, payment)
		require.NoError(t, err)

		// Get by ID
		retrieved, err := db.GetPaymentByID(ctx, payment.ID)
		require.NoError(t, err)
		assert.Equal(t, payment.ID, retrieved.ID)
		assert.Equal(t, payment.Amount, retrieved.Amount)
		assert.Equal(t, payment.Currency, retrieved.Currency)
	})

	t.Run("Get Payment By Reference", func(t *testing.T) {
		reference := "TEST-" + time.Now().Format("20060102150405")

		// Create payment
		payment := &Payment{
			PaymentReference: reference,
			SenderBankID:     senderBank.ID,
			ReceiverBankID:   receiverBank.ID,
			Amount:           3000.00,
			Currency:         "GBP",
			Status:           "pending",
		}

		err := db.CreatePayment(ctx, payment)
		require.NoError(t, err)

		// Get by reference
		retrieved, err := db.GetPaymentByReference(ctx, reference)
		require.NoError(t, err)
		assert.Equal(t, reference, retrieved.PaymentReference)
	})

	t.Run("Update Payment Status", func(t *testing.T) {
		// Create payment
		payment := &Payment{
			PaymentReference: "TEST-" + time.Now().Format("20060102150405"),
			SenderBankID:     senderBank.ID,
			ReceiverBankID:   receiverBank.ID,
			Amount:           4000.00,
			Currency:         "USD",
			Status:           "pending",
		}

		err := db.CreatePayment(ctx, payment)
		require.NoError(t, err)

		// Update status
		err = db.UpdatePaymentStatus(ctx, payment.ID, "processing")
		require.NoError(t, err)

		// Verify update
		updated, err := db.GetPaymentByID(ctx, payment.ID)
		require.NoError(t, err)
		assert.Equal(t, "processing", updated.Status)
		assert.NotNil(t, updated.ProcessedAt)

		// Update to settled
		err = db.UpdatePaymentStatus(ctx, payment.ID, "settled")
		require.NoError(t, err)

		// Verify settled
		settled, err := db.GetPaymentByID(ctx, payment.ID)
		require.NoError(t, err)
		assert.Equal(t, "settled", settled.Status)
		assert.NotNil(t, settled.SettledAt)
	})

	t.Run("List Pending Payments", func(t *testing.T) {
		// Create multiple pending payments
		for i := 0; i < 5; i++ {
			payment := &Payment{
				PaymentReference: "PENDING-" + time.Now().Format("20060102150405"),
				SenderBankID:     senderBank.ID,
				ReceiverBankID:   receiverBank.ID,
				Amount:           float64(i+1) * 100.00,
				Currency:         "USD",
				Status:           "pending",
			}

			err := db.CreatePayment(ctx, payment)
			require.NoError(t, err)

			time.Sleep(10 * time.Millisecond) // Ensure different timestamps
		}

		// List pending
		payments, err := db.ListPendingPayments(ctx, 10)
		require.NoError(t, err)
		assert.NotEmpty(t, payments)

		// Verify all are pending
		for _, p := range payments {
			assert.Equal(t, "pending", p.Status)
		}
	})
}

func TestTransactions(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	db := setupTestDB(t)
	defer db.Close()

	ctx := context.Background()

	// Get test banks
	senderBank, err := db.GetBankByBIC(ctx, "CHASUS33XXX")
	require.NoError(t, err)

	receiverBank, err := db.GetBankByBIC(ctx, "DEUTDEFFXXX")
	require.NoError(t, err)

	t.Run("Successful Transaction", func(t *testing.T) {
		err := db.ExecTx(ctx, func(tx *sql.Tx) error {
			// Insert payment within transaction
			query := `
				INSERT INTO deltran.payments (
					payment_reference, sender_bank_id, receiver_bank_id,
					amount, currency, status
				) VALUES ($1, $2, $3, $4, $5, $6)
			`

			_, err := tx.ExecContext(ctx, query,
				"TX-"+time.Now().Format("20060102150405"),
				senderBank.ID,
				receiverBank.ID,
				5000.00,
				"USD",
				"pending",
			)

			return err
		})

		assert.NoError(t, err)
	})

	t.Run("Rollback On Error", func(t *testing.T) {
		err := db.ExecTx(ctx, func(tx *sql.Tx) error {
			// Insert payment
			query := `
				INSERT INTO deltran.payments (
					payment_reference, sender_bank_id, receiver_bank_id,
					amount, currency, status
				) VALUES ($1, $2, $3, $4, $5, $6)
			`

			_, err := tx.ExecContext(ctx, query,
				"TX-ROLLBACK-"+time.Now().Format("20060102150405"),
				senderBank.ID,
				receiverBank.ID,
				6000.00,
				"USD",
				"pending",
			)

			if err != nil {
				return err
			}

			// Force error to trigger rollback
			return assert.AnError
		})

		assert.Error(t, err)

		// Verify payment was not created
		_, err = db.GetPaymentByReference(ctx, "TX-ROLLBACK-"+time.Now().Format("20060102150405"))
		assert.Error(t, err)
	})
}

func TestAuditLog(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping integration test in short mode")
	}

	db := setupTestDB(t)
	defer db.Close()

	ctx := context.Background()

	t.Run("Create Audit Log", func(t *testing.T) {
		log := &AuditLog{
			EventType:    "payment.created",
			Severity:     "info",
			ActorType:    stringPtr("user"),
			ActorName:    stringPtr("test-user"),
			Action:       "create_payment",
			ResourceType: stringPtr("payment"),
			Result:       "success",
			IPAddress:    stringPtr("192.168.1.100"),
		}

		err := db.CreateAuditLog(ctx, log)
		require.NoError(t, err)
		assert.NotZero(t, log.ID)
		assert.NotEmpty(t, log.EventID)
		assert.NotZero(t, log.Timestamp)
	})
}

func BenchmarkGetUserByEmail(b *testing.B) {
	db := setupTestDB(&testing.T{})
	defer db.Close()

	ctx := context.Background()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := db.GetUserByEmail(ctx, "admin@deltran.local")
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkGetBankByBIC(b *testing.B) {
	db := setupTestDB(&testing.T{})
	defer db.Close()

	ctx := context.Background()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, err := db.GetBankByBIC(ctx, "CHASUS33XXX")
		if err != nil {
			b.Fatal(err)
		}
	}
}

func BenchmarkCreatePayment(b *testing.B) {
	db := setupTestDB(&testing.T{})
	defer db.Close()

	ctx := context.Background()

	// Get test banks
	senderBank, _ := db.GetBankByBIC(ctx, "CHASUS33XXX")
	receiverBank, _ := db.GetBankByBIC(ctx, "DEUTDEFFXXX")

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		payment := &Payment{
			PaymentReference: "BENCH-" + time.Now().Format("20060102150405"),
			SenderBankID:     senderBank.ID,
			ReceiverBankID:   receiverBank.ID,
			Amount:           1000.00,
			Currency:         "USD",
			Status:           "pending",
		}

		err := db.CreatePayment(ctx, payment)
		if err != nil {
			b.Fatal(err)
		}
	}
}

// Helper functions
func stringPtr(s string) *string {
	return &s
}
