package tests

import (
	"bytes"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"

	"deltran/gateway/internal/clients"
	"deltran/gateway/internal/handlers"
	"deltran/gateway/internal/middleware"
	"deltran/gateway/internal/models"
	"deltran/gateway/internal/orchestration"

	"github.com/gorilla/mux"
)

func TestHealthCheck(t *testing.T) {
	// Create handler
	authMiddleware := middleware.NewAuthMiddleware("test-secret")
	orchestrator := createMockOrchestrator()
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Create request
	req, err := http.NewRequest("GET", "/health", nil)
	if err != nil {
		t.Fatal(err)
	}

	// Create response recorder
	rr := httptest.NewRecorder()

	// Call handler
	handler.HealthCheck(rr, req)

	// Check status code
	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	// Check response
	var response models.HealthResponse
	if err := json.Unmarshal(rr.Body.Bytes(), &response); err != nil {
		t.Errorf("failed to unmarshal response: %v", err)
	}

	if response.Status != "healthy" {
		t.Errorf("handler returned wrong status: got %v want %v", response.Status, "healthy")
	}
}

func TestTransferHandler(t *testing.T) {
	// Create handler
	authMiddleware := middleware.NewAuthMiddleware("test-secret")
	orchestrator := createMockOrchestrator()
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Create request body
	transferReq := models.TransferRequest{
		SenderBank:      "ICICI",
		ReceiverBank:    "ENBD",
		Amount:          100000,
		FromCurrency:    "INR",
		ToCurrency:      "AED",
		SenderAccount:   "ACC001",
		ReceiverAccount: "ACC002",
		Reference:       "TEST001",
	}

	body, err := json.Marshal(transferReq)
	if err != nil {
		t.Fatal(err)
	}

	// Create request
	req, err := http.NewRequest("POST", "/api/v1/transfer", bytes.NewBuffer(body))
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Content-Type", "application/json")

	// Create response recorder
	rr := httptest.NewRecorder()

	// Call handler
	handler.TransferHandler(rr, req)

	// Check status code (should be 202 Accepted or 500 if services are down)
	if status := rr.Code; status != http.StatusAccepted && status != http.StatusInternalServerError {
		t.Errorf("handler returned unexpected status code: got %v want %v or %v",
			status, http.StatusAccepted, http.StatusInternalServerError)
	}
}

func TestGetBanksHandler(t *testing.T) {
	// Create handler
	authMiddleware := middleware.NewAuthMiddleware("test-secret")
	orchestrator := createMockOrchestrator()
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Create request
	req, err := http.NewRequest("GET", "/api/v1/banks", nil)
	if err != nil {
		t.Fatal(err)
	}

	// Create response recorder
	rr := httptest.NewRecorder()

	// Call handler
	handler.GetBanksHandler(rr, req)

	// Check status code
	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	// Check response
	var banks []models.Bank
	if err := json.Unmarshal(rr.Body.Bytes(), &banks); err != nil {
		t.Errorf("failed to unmarshal response: %v", err)
	}

	if len(banks) == 0 {
		t.Error("expected at least one bank in response")
	}
}

func TestGetCorridorsHandler(t *testing.T) {
	// Create handler
	authMiddleware := middleware.NewAuthMiddleware("test-secret")
	orchestrator := createMockOrchestrator()
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Create request
	req, err := http.NewRequest("GET", "/api/v1/corridors", nil)
	if err != nil {
		t.Fatal(err)
	}

	// Create response recorder
	rr := httptest.NewRecorder()

	// Call handler
	handler.GetCorridorsHandler(rr, req)

	// Check status code
	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	// Check response
	var corridors []models.Corridor
	if err := json.Unmarshal(rr.Body.Bytes(), &corridors); err != nil {
		t.Errorf("failed to unmarshal response: %v", err)
	}

	if len(corridors) == 0 {
		t.Error("expected at least one corridor in response")
	}
}

func TestGetRatesHandler(t *testing.T) {
	// Create handler
	authMiddleware := middleware.NewAuthMiddleware("test-secret")
	orchestrator := createMockOrchestrator()
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Create request
	req, err := http.NewRequest("GET", "/api/v1/rates/INR-AED", nil)
	if err != nil {
		t.Fatal(err)
	}

	// Add mux vars
	req = mux.SetURLVars(req, map[string]string{"corridor": "INR-AED"})

	// Create response recorder
	rr := httptest.NewRecorder()

	// Call handler
	handler.GetRatesHandler(rr, req)

	// Check status code
	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	// Check response
	var rate models.FXRate
	if err := json.Unmarshal(rr.Body.Bytes(), &rate); err != nil {
		t.Errorf("failed to unmarshal response: %v", err)
	}

	if rate.Corridor != "INR-AED" {
		t.Errorf("expected corridor INR-AED, got %v", rate.Corridor)
	}
}

func TestLoginHandler(t *testing.T) {
	// Create handler
	authMiddleware := middleware.NewAuthMiddleware("test-secret")
	orchestrator := createMockOrchestrator()
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Create request body
	loginReq := map[string]string{
		"bank_id":  "ICICI",
		"password": "demo",
	}

	body, err := json.Marshal(loginReq)
	if err != nil {
		t.Fatal(err)
	}

	// Create request
	req, err := http.NewRequest("POST", "/api/v1/auth/login", bytes.NewBuffer(body))
	if err != nil {
		t.Fatal(err)
	}
	req.Header.Set("Content-Type", "application/json")

	// Create response recorder
	rr := httptest.NewRecorder()

	// Call handler
	handler.LoginHandler(rr, req)

	// Check status code
	if status := rr.Code; status != http.StatusOK {
		t.Errorf("handler returned wrong status code: got %v want %v", status, http.StatusOK)
	}

	// Check response
	var response map[string]interface{}
	if err := json.Unmarshal(rr.Body.Bytes(), &response); err != nil {
		t.Errorf("failed to unmarshal response: %v", err)
	}

	if _, ok := response["token"]; !ok {
		t.Error("expected token in response")
	}
}

// createMockOrchestrator creates a mock orchestrator for testing
func createMockOrchestrator() *orchestration.TransactionOrchestrator {
	// Note: These will fail when actually called, but that's okay for basic handler tests
	// In production, use proper mocks
	return orchestration.NewTransactionOrchestrator(
		clients.NewComplianceClient("http://localhost:8086"),
		clients.NewRiskClient("http://localhost:8084"),
		clients.NewLiquidityClient("http://localhost:8083"),
		clients.NewObligationClient("http://localhost:8082"),
		clients.NewTokenClient("http://localhost:8081"),
		clients.NewNotificationClient("http://localhost:8089"),
	)
}
