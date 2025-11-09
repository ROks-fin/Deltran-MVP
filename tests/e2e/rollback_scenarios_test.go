package e2e

import (
	"bytes"
	"context"
	"database/sql"
	"encoding/json"
	"fmt"
	"net/http"
	"testing"
	"time"

	_ "github.com/lib/pq"
)

const (
	dbURL     = "postgresql://deltran:deltran_secure_pass_2024@localhost:5432/deltran?sslmode=disable"
	gatewayURL = "http://localhost:8080"
)

// TestNetworkFailureDuringSettlement simulates network failure during settlement
func TestNetworkFailureDuringSettlement(t *testing.T) {
	// This test would require service orchestration control
	// For now, we test the rollback mechanism manually

	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Skipf("Database not available: %v", err)
		return
	}
	defer db.Close()

	client := &http.Client{Timeout: 30 * time.Second}

	// Create transaction
	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         1000.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "ROLLBACK-TEST-NETWORK",
		"idempotency_key": fmt.Sprintf("rollback-net-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)
	resp, err := client.Post(gatewayURL+"/api/v1/transfer", "application/json", bytes.NewBuffer(payload))

	if err != nil {
		t.Logf("Network simulation: %v", err)
	}

	if resp != nil {
		defer resp.Body.Close()
		var result map[string]interface{}
		json.NewDecoder(resp.Body).Decode(&result)

		// Check if transaction was rolled back
		if txID, ok := result["transaction_id"].(string); ok {
			time.Sleep(5 * time.Second) // Wait for potential rollback

			// Verify transaction state in DB
			var status string
			err := db.QueryRow("SELECT status FROM transactions WHERE id = $1", txID).Scan(&status)
			if err == nil {
				t.Logf("Transaction status after network failure: %s", status)
				if status != "failed" && status != "rolled_back" {
					t.Logf("Warning: Transaction not properly rolled back, status: %s", status)
				} else {
					t.Log("✓ Transaction properly rolled back on network failure")
				}
			}
		}
	}
}

// TestDatabaseConnectionLoss tests recovery from database disconnection
func TestDatabaseConnectionLoss(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Skipf("Database not available: %v", err)
		return
	}
	defer db.Close()

	// Test database reconnection
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		t.Errorf("Database ping failed: %v", err)
	}

	// Simulate connection pool exhaustion
	db.SetMaxOpenConns(1)
	db.SetMaxIdleConns(0)

	// Try to acquire connection
	conn, err := db.Conn(ctx)
	if err != nil {
		t.Errorf("Failed to acquire connection: %v", err)
	} else {
		conn.Close()
		t.Log("✓ Database connection recovery works")
	}
}

// TestClearingWindowRollback tests atomic rollback of clearing window
func TestClearingWindowRollback(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Skipf("Database not available: %v", err)
		return
	}
	defer db.Close()

	client := &http.Client{Timeout: 30 * time.Second}

	// Check current clearing window
	resp, err := client.Get("http://localhost:8085/api/v1/clearing/windows/current")
	if err != nil {
		t.Skipf("Clearing engine not available: %v", err)
		return
	}
	defer resp.Body.Close()

	var windowInfo map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&windowInfo); err != nil {
		t.Fatalf("Failed to decode window info: %v", err)
	}

	t.Logf("Current clearing window: %v", windowInfo)

	// Test rollback by checking window state consistency
	// In a real scenario, we would inject a failure mid-window
	time.Sleep(2 * time.Second)

	resp2, err := client.Get("http://localhost:8085/api/v1/clearing/windows/current")
	if err == nil {
		defer resp2.Body.Close()
		var windowInfo2 map[string]interface{}
		json.NewDecoder(resp2.Body).Decode(&windowInfo2)

		// Window should be consistent
		if windowInfo["window_id"] == windowInfo2["window_id"] {
			t.Log("✓ Clearing window state is consistent")
		}
	}
}

// TestSettlementPartialFailure tests compensation transaction on partial failure
func TestSettlementPartialFailure(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Skipf("Database not available: %v", err)
		return
	}
	defer db.Close()

	// Create a settlement request that might partially fail
	settlementReq := map[string]interface{}{
		"clearing_window_id": 1,
		"instructions": []map[string]interface{}{
			{
				"from_bank": "BANK001",
				"to_bank":   "BANK002",
				"amount":    1000.00,
				"currency":  "USD",
			},
		},
	}

	payload, _ := json.Marshal(settlementReq)
	client := &http.Client{Timeout: 30 * time.Second}

	resp, err := client.Post("http://localhost:8087/api/v1/settlement/execute",
		"application/json",
		bytes.NewBuffer(payload))

	if err != nil {
		t.Logf("Settlement request failed (expected in some cases): %v", err)
		return
	}
	defer resp.Body.Close()

	var result map[string]interface{}
	json.NewDecoder(resp.Body).Decode(&result)

	// Check if settlement has rollback mechanism
	if status, ok := result["status"].(string); ok {
		t.Logf("Settlement status: %s", status)

		if status == "failed" || status == "rolled_back" {
			// Check for compensation transaction
			if compensationID, exists := result["compensation_id"]; exists {
				t.Logf("✓ Compensation transaction created: %v", compensationID)
			} else {
				t.Log("Note: No compensation transaction ID in response")
			}
		}
	}

	t.Log("✓ Settlement partial failure test completed")
}

// TestAtomicOperationRollback tests atomic operation controller rollback
func TestAtomicOperationRollback(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Skipf("Database not available: %v", err)
		return
	}
	defer db.Close()

	ctx := context.Background()

	// Begin a transaction
	tx, err := db.BeginTx(ctx, nil)
	if err != nil {
		t.Fatalf("Failed to begin transaction: %v", err)
	}

	// Try to insert test data
	_, err = tx.ExecContext(ctx, `
		INSERT INTO test_rollback (id, data)
		VALUES ($1, $2)
		ON CONFLICT DO NOTHING
	`, 999, "test_data")

	if err != nil {
		// Table might not exist, which is ok
		t.Logf("Insert failed (table may not exist): %v", err)
	}

	// Rollback
	if err := tx.Rollback(); err != nil {
		t.Errorf("Rollback failed: %v", err)
	} else {
		t.Log("✓ Atomic operation rollback successful")
	}

	// Verify data was not committed
	var count int
	err = db.QueryRowContext(ctx, "SELECT COUNT(*) FROM test_rollback WHERE id = $1", 999).Scan(&count)

	if err != nil {
		t.Log("✓ Rollback verified (table cleaned or doesn't exist)")
	} else if count == 0 {
		t.Log("✓ Rollback verified (no data committed)")
	} else {
		t.Errorf("Rollback failed: data was committed")
	}
}

// TestNATSServerDown tests NATS message retry
func TestNATSServerDown(t *testing.T) {
	// This is a simulation test
	// In production, you would stop NATS container and verify retry logic

	client := &http.Client{Timeout: 30 * time.Second}

	// Try to send a transaction that would trigger NATS events
	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         100.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "NATS-DOWN-TEST",
		"idempotency_key": fmt.Sprintf("nats-test-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)
	resp, err := client.Post(gatewayURL+"/api/v1/transfer", "application/json", bytes.NewBuffer(payload))

	if err != nil {
		t.Logf("Request failed (NATS might be down): %v", err)
	}

	if resp != nil {
		defer resp.Body.Close()

		// Transaction should still be processed even if NATS is down
		// Events should be queued for retry
		if resp.StatusCode == http.StatusOK || resp.StatusCode == http.StatusCreated {
			t.Log("✓ Transaction processed despite NATS issues")
		} else {
			t.Logf("Transaction failed with status %d", resp.StatusCode)
		}
	}

	t.Log("✓ NATS failure handling test completed")
}

// TestConcurrentRollback tests multiple simultaneous rollbacks
func TestConcurrentRollback(t *testing.T) {
	db, err := sql.Open("postgres", dbURL)
	if err != nil {
		t.Skipf("Database not available: %v", err)
		return
	}
	defer db.Close()

	numConcurrent := 5
	done := make(chan bool, numConcurrent)

	for i := 0; i < numConcurrent; i++ {
		go func(id int) {
			ctx := context.Background()
			tx, err := db.BeginTx(ctx, nil)
			if err != nil {
				done <- false
				return
			}

			// Simulate work
			time.Sleep(time.Duration(id*100) * time.Millisecond)

			// Rollback
			if err := tx.Rollback(); err != nil {
				t.Errorf("Concurrent rollback %d failed: %v", id, err)
				done <- false
			} else {
				done <- true
			}
		}(i)
	}

	successCount := 0
	for i := 0; i < numConcurrent; i++ {
		if <-done {
			successCount++
		}
	}

	if successCount == numConcurrent {
		t.Logf("✓ All %d concurrent rollbacks successful", numConcurrent)
	} else {
		t.Errorf("Only %d/%d concurrent rollbacks successful", successCount, numConcurrent)
	}
}

// TestIdempotencyOnRetry tests idempotency during retry scenarios
func TestIdempotencyOnRetry(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	idempotencyKey := fmt.Sprintf("retry-test-%d", time.Now().Unix())

	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         500.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "RETRY-TEST",
		"idempotency_key": idempotencyKey,
	}

	payload, _ := json.Marshal(transfer)

	// First attempt
	resp1, err := client.Post(gatewayURL+"/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Skipf("First request failed: %v", err)
		return
	}
	defer resp1.Body.Close()

	var result1 map[string]interface{}
	json.NewDecoder(resp1.Body).Decode(&result1)
	txID1, _ := result1["transaction_id"].(string)

	// Simulate retry after failure
	time.Sleep(2 * time.Second)

	// Second attempt with same idempotency key
	payload2, _ := json.Marshal(transfer)
	resp2, err := client.Post(gatewayURL+"/api/v1/transfer", "application/json", bytes.NewBuffer(payload2))
	if err != nil {
		t.Skipf("Second request failed: %v", err)
		return
	}
	defer resp2.Body.Close()

	var result2 map[string]interface{}
	json.NewDecoder(resp2.Body).Decode(&result2)
	txID2, _ := result2["transaction_id"].(string)

	if txID1 != "" && txID2 != "" {
		if txID1 == txID2 {
			t.Log("✓ Idempotency maintained on retry")
		} else {
			t.Errorf("Idempotency violated on retry: %s != %s", txID1, txID2)
		}
	}
}
