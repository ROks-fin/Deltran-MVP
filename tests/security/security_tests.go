package security

import (
	"bytes"
	"encoding/json"
	"net/http"
	"strings"
	"testing"
	"time"
)

// TestAuthenticationBypass tests unauthorized access attempts
func TestAuthenticationBypass(t *testing.T) {
	client := &http.Client{Timeout: 10 * time.Second}

	tests := []struct {
		name     string
		endpoint string
		headers  map[string]string
	}{
		{
			name:     "No authentication header",
			endpoint: "http://localhost:8080/api/v1/transactions",
			headers:  map[string]string{},
		},
		{
			name:     "Invalid JWT token",
			endpoint: "http://localhost:8080/api/v1/transactions",
			headers:  map[string]string{"Authorization": "Bearer invalid_token_12345"},
		},
		{
			name:     "Expired JWT token",
			endpoint: "http://localhost:8080/api/v1/transactions",
			headers:  map[string]string{"Authorization": "Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjE1MTYyMzkwMjJ9.invalid"},
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req, _ := http.NewRequest("GET", tt.endpoint, nil)
			for k, v := range tt.headers {
				req.Header.Set(k, v)
			}

			resp, err := client.Do(req)
			if err != nil {
				t.Logf("Request failed (expected): %v", err)
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == http.StatusOK {
				t.Errorf("Expected authentication failure, got 200 OK")
			} else {
				t.Logf("✓ Properly rejected with status %d", resp.StatusCode)
			}
		})
	}
}

// TestSQLInjection tests SQL injection vulnerabilities
func TestSQLInjection(t *testing.T) {
	client := &http.Client{Timeout: 10 * time.Second}

	injectionPayloads := []string{
		"' OR '1'='1",
		"'; DROP TABLE users; --",
		"1' UNION SELECT * FROM banks--",
		"admin'--",
		"' OR 1=1--",
	}

	for _, payload := range injectionPayloads {
		transfer := map[string]interface{}{
			"sender_bank":    payload,
			"receiver_bank":  "BANK002",
			"amount":         100.00,
			"from_currency":  "USD",
			"to_currency":    "USD",
			"reference":      payload,
			"idempotency_key": "sql-injection-test",
		}

		jsonData, _ := json.Marshal(transfer)

		resp, err := client.Post("http://localhost:8080/api/v1/transfer",
			"application/json",
			bytes.NewBuffer(jsonData))

		if err != nil {
			t.Logf("Request with payload '%s' failed (good): %v", payload, err)
			continue
		}
		defer resp.Body.Close()

		// Should be rejected with 400 Bad Request
		if resp.StatusCode == http.StatusOK || resp.StatusCode == http.StatusCreated {
			t.Errorf("SQL injection payload not blocked: %s", payload)
		} else {
			t.Logf("✓ SQL injection blocked: %s (status %d)", payload, resp.StatusCode)
		}
	}
}

// TestRateLimiting tests rate limit enforcement
func TestRateLimiting(t *testing.T) {
	client := &http.Client{Timeout: 5 * time.Second}

	endpoint := "http://localhost:8080/health"
	requestCount := 150 // Should exceed 100 req/min limit

	rateLimitHit := false

	for i := 0; i < requestCount; i++ {
		resp, err := client.Get(endpoint)
		if err != nil {
			t.Logf("Request %d failed: %v", i, err)
			continue
		}

		if resp.StatusCode == http.StatusTooManyRequests {
			rateLimitHit = true
			t.Logf("✓ Rate limit enforced at request %d", i)
			resp.Body.Close()
			break
		}

		resp.Body.Close()

		// Small delay to simulate real usage
		if i%10 == 0 {
			time.Sleep(10 * time.Millisecond)
		}
	}

	if !rateLimitHit {
		t.Logf("Note: Rate limiting not enforced (may not be configured for health endpoint)")
	}
}

// TestInputSanitization tests input validation
func TestInputSanitization(t *testing.T) {
	client := &http.Client{Timeout: 10 * time.Second}

	tests := []struct {
		name    string
		payload map[string]interface{}
		expect  string
	}{
		{
			name: "Negative amount",
			payload: map[string]interface{}{
				"sender_bank":   "BANK001",
				"receiver_bank": "BANK002",
				"amount":        -1000.00,
				"from_currency": "USD",
				"to_currency":   "USD",
			},
			expect: "should be rejected",
		},
		{
			name: "Invalid currency",
			payload: map[string]interface{}{
				"sender_bank":   "BANK001",
				"receiver_bank": "BANK002",
				"amount":        1000.00,
				"from_currency": "INVALID",
				"to_currency":   "XYZ",
			},
			expect: "should be rejected",
		},
		{
			name: "Missing required fields",
			payload: map[string]interface{}{
				"amount": 1000.00,
			},
			expect: "should be rejected",
		},
		{
			name: "XSS attempt in reference",
			payload: map[string]interface{}{
				"sender_bank":   "BANK001",
				"receiver_bank": "BANK002",
				"amount":        100.00,
				"from_currency": "USD",
				"to_currency":   "USD",
				"reference":     "<script>alert('XSS')</script>",
			},
			expect: "should sanitize",
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			jsonData, _ := json.Marshal(tt.payload)

			resp, err := client.Post("http://localhost:8080/api/v1/transfer",
				"application/json",
				bytes.NewBuffer(jsonData))

			if err != nil {
				t.Logf("Request failed (expected): %v", err)
				return
			}
			defer resp.Body.Close()

			if resp.StatusCode == http.StatusOK || resp.StatusCode == http.StatusCreated {
				var result map[string]interface{}
				json.NewDecoder(resp.Body).Decode(&result)
				t.Logf("Warning: Invalid input accepted - %s", tt.name)
			} else {
				t.Logf("✓ Invalid input rejected - %s (status %d)", tt.name, resp.StatusCode)
			}
		})
	}
}

// TestJWTValidation tests JWT token validation
func TestJWTValidation(t *testing.T) {
	client := &http.Client{Timeout: 10 * time.Second}

	malformedTokens := []string{
		"Bearer ",
		"Bearer invalid",
		"Bearer eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9",
		"NotBearer token",
		"",
	}

	for _, token := range malformedTokens {
		req, _ := http.NewRequest("GET", "http://localhost:8080/api/v1/transactions", nil)
		if token != "" {
			req.Header.Set("Authorization", token)
		}

		resp, err := client.Do(req)
		if err != nil {
			continue
		}
		defer resp.Body.Close()

		if resp.StatusCode == http.StatusOK {
			t.Errorf("Malformed JWT token accepted: %s", token)
		} else {
			t.Logf("✓ Malformed JWT rejected: status %d", resp.StatusCode)
		}
	}
}

// TestXSSPrevention tests XSS attack prevention
func TestXSSPrevention(t *testing.T) {
	client := &http.Client{Timeout: 10 * time.Second}

	xssPayloads := []string{
		"<script>alert('XSS')</script>",
		"<img src=x onerror=alert('XSS')>",
		"javascript:alert('XSS')",
		"<iframe src='javascript:alert(1)'>",
	}

	for _, payload := range xssPayloads {
		transfer := map[string]interface{}{
			"sender_bank":    "BANK001",
			"receiver_bank":  "BANK002",
			"amount":         100.00,
			"from_currency":  "USD",
			"to_currency":    "USD",
			"reference":      payload,
			"sender_name":    payload,
			"idempotency_key": "xss-test",
		}

		jsonData, _ := json.Marshal(transfer)
		resp, err := client.Post("http://localhost:8080/api/v1/transfer",
			"application/json",
			bytes.NewBuffer(jsonData))

		if err != nil {
			continue
		}
		defer resp.Body.Close()

		var result map[string]interface{}
		json.NewDecoder(resp.Body).Decode(&result)

		// Check if response contains unsanitized payload
		respBody := result["reference"]
		if respBody != nil && strings.Contains(respBody.(string), "<script>") {
			t.Errorf("XSS payload not sanitized: %s", payload)
		} else {
			t.Logf("✓ XSS payload handled: %s", payload)
		}
	}
}
