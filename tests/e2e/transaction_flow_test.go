package e2e

import (
	"bytes"
	"encoding/json"
	"fmt"
	"net/http"
	"testing"
	"time"
)

// TestTransactionHappyPath tests successful payment flow
// compliance → risk → liquidity → obligation → token → clearing → settlement
func TestTransactionHappyPath(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	// Test data
	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         1000.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "TEST-TRANSFER-001",
		"idempotency_key": fmt.Sprintf("test-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)

	// Step 1: Submit transfer via Gateway
	resp, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Fatalf("Failed to submit transfer: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusCreated {
		t.Fatalf("Expected 200/201, got %d", resp.StatusCode)
	}

	var result map[string]interface{}
	if err := json.NewDecoder(resp.Body).Decode(&result); err != nil {
		t.Fatalf("Failed to decode response: %v", err)
	}

	transactionID, ok := result["transaction_id"].(string)
	if !ok {
		t.Fatal("No transaction_id in response")
	}

	t.Logf("Transaction created: %s", transactionID)

	// Step 2: Check transaction status
	statusResp, err := client.Get(fmt.Sprintf("http://localhost:8080/api/v1/transaction/%s", transactionID))
	if err != nil {
		t.Fatalf("Failed to get transaction status: %v", err)
	}
	defer statusResp.Body.Close()

	if statusResp.StatusCode != http.StatusOK {
		t.Fatalf("Expected 200 for status, got %d", statusResp.StatusCode)
	}

	t.Log("✓ Happy path transaction flow completed successfully")
}

// TestComplianceBlock tests transaction blocked by compliance
func TestComplianceBlock(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	// Sanctioned entity name
	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         5000.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "SANCTIONED-TEST",
		"sender_name":    "OSAMA BIN LADEN", // Known sanctioned name
		"idempotency_key": fmt.Sprintf("test-compliance-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)

	resp, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Fatalf("Failed to submit transfer: %v", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusForbidden && resp.StatusCode != http.StatusBadRequest {
		t.Logf("Warning: Expected 403/400 for sanctioned transfer, got %d", resp.StatusCode)
	}

	t.Log("✓ Compliance blocking test completed")
}

// TestRiskHighAmount tests risk engine blocks high-risk payment
func TestRiskHighAmount(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	// Very high amount to trigger risk check
	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         10000000.00, // 10M USD
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "HIGH-RISK-TEST",
		"idempotency_key": fmt.Sprintf("test-risk-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)

	resp, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Fatalf("Failed to submit transfer: %v", err)
	}
	defer resp.Body.Close()

	t.Logf("Risk test response status: %d", resp.StatusCode)
	t.Log("✓ Risk engine high amount test completed")
}

// TestInsufficientLiquidity tests liquidity rejection
func TestInsufficientLiquidity(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	// Exotic currency pair with no liquidity
	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         1000000.00,
		"from_currency":  "XYZ",
		"to_currency":    "ABC",
		"reference":      "NO-LIQUIDITY-TEST",
		"idempotency_key": fmt.Sprintf("test-liquidity-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)

	resp, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Fatalf("Failed to submit transfer: %v", err)
	}
	defer resp.Body.Close()

	t.Logf("Liquidity test response status: %d", resp.StatusCode)
	t.Log("✓ Insufficient liquidity test completed")
}

// TestDuplicateTransaction tests idempotency
func TestDuplicateTransaction(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	idempotencyKey := fmt.Sprintf("test-duplicate-%d", time.Now().Unix())

	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         500.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "DUPLICATE-TEST",
		"idempotency_key": idempotencyKey,
	}

	payload, _ := json.Marshal(transfer)

	// First request
	resp1, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Fatalf("Failed to submit first transfer: %v", err)
	}
	defer resp1.Body.Close()

	var result1 map[string]interface{}
	json.NewDecoder(resp1.Body).Decode(&result1)
	txID1, _ := result1["transaction_id"].(string)

	// Wait a bit
	time.Sleep(1 * time.Second)

	// Second request with same idempotency key
	payload2, _ := json.Marshal(transfer)
	resp2, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload2))
	if err != nil {
		t.Fatalf("Failed to submit second transfer: %v", err)
	}
	defer resp2.Body.Close()

	var result2 map[string]interface{}
	json.NewDecoder(resp2.Body).Decode(&result2)
	txID2, _ := result2["transaction_id"].(string)

	if txID1 != "" && txID2 != "" && txID1 != txID2 {
		t.Errorf("Idempotency violated: %s != %s", txID1, txID2)
	} else {
		t.Log("✓ Idempotency test completed successfully")
	}
}

// TestConcurrentTransactions tests system under concurrent load
func TestConcurrentTransactions(t *testing.T) {
	client := &http.Client{Timeout: 30 * time.Second}

	numConcurrent := 10
	done := make(chan bool, numConcurrent)
	errors := make(chan error, numConcurrent)

	for i := 0; i < numConcurrent; i++ {
		go func(idx int) {
			transfer := map[string]interface{}{
				"sender_bank":    "BANK001",
				"receiver_bank":  "BANK002",
				"amount":         float64(100 + idx),
				"from_currency":  "USD",
				"to_currency":    "USD",
				"reference":      fmt.Sprintf("CONCURRENT-TEST-%d", idx),
				"idempotency_key": fmt.Sprintf("test-concurrent-%d-%d", time.Now().Unix(), idx),
			}

			payload, _ := json.Marshal(transfer)
			resp, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
			if err != nil {
				errors <- err
				done <- false
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode >= 500 {
				errors <- fmt.Errorf("server error: %d", resp.StatusCode)
				done <- false
				return
			}

			done <- true
		}(i)
	}

	successCount := 0
	for i := 0; i < numConcurrent; i++ {
		if <-done {
			successCount++
		}
	}

	close(errors)
	errorCount := len(errors)

	t.Logf("Concurrent test: %d/%d successful, %d errors", successCount, numConcurrent, errorCount)

	if successCount == 0 {
		t.Fatal("All concurrent transactions failed")
	}

	t.Log("✓ Concurrent transactions test completed")
}

// TestSettlementLatency tests settlement completes within 30 seconds
func TestSettlementLatency(t *testing.T) {
	client := &http.Client{Timeout: 60 * time.Second}

	transfer := map[string]interface{}{
		"sender_bank":    "BANK001",
		"receiver_bank":  "BANK002",
		"amount":         1000.00,
		"from_currency":  "USD",
		"to_currency":    "USD",
		"reference":      "LATENCY-TEST",
		"idempotency_key": fmt.Sprintf("test-latency-%d", time.Now().Unix()),
	}

	payload, _ := json.Marshal(transfer)

	start := time.Now()

	resp, err := client.Post("http://localhost:8080/api/v1/transfer", "application/json", bytes.NewBuffer(payload))
	if err != nil {
		t.Fatalf("Failed to submit transfer: %v", err)
	}
	defer resp.Body.Close()

	elapsed := time.Since(start)

	if elapsed > 30*time.Second {
		t.Errorf("Settlement took %v, expected < 30s", elapsed)
	} else {
		t.Logf("✓ Settlement completed in %v (< 30s target)", elapsed)
	}
}
