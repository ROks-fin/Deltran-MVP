package handlers

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"time"

	"deltran/gateway/internal/middleware"
	"deltran/gateway/internal/models"
	"deltran/gateway/internal/orchestration"

	"github.com/google/uuid"
	"github.com/gorilla/mux"
)

// Handler holds all dependencies for request handlers
type Handler struct {
	orchestrator *orchestration.TransactionOrchestrator
	authMiddleware *middleware.AuthMiddleware
	startTime    time.Time
}

// NewHandler creates a new handler
func NewHandler(orchestrator *orchestration.TransactionOrchestrator, authMiddleware *middleware.AuthMiddleware) *Handler {
	return &Handler{
		orchestrator: orchestrator,
		authMiddleware: authMiddleware,
		startTime:    time.Now(),
	}
}

// HealthCheck handles health check requests
func (h *Handler) HealthCheck(w http.ResponseWriter, r *http.Request) {
	response := models.HealthResponse{
		Status:  "healthy",
		Service: "gateway",
		Version: "1.0.0",
		Uptime:  time.Since(h.startTime).String(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// TransferHandler handles transfer requests
func (h *Handler) TransferHandler(w http.ResponseWriter, r *http.Request) {
	var req models.TransferRequest

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.errorResponse(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Validate request
	if err := h.validateTransferRequest(&req); err != nil {
		h.errorResponse(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Generate transaction ID
	transactionID := fmt.Sprintf("TXN-%s", uuid.New().String()[:8])

	// Check for idempotency key
	if req.IdempotencyKey != "" {
		// In production, check Redis for existing transaction with this key
		// For MVP, just use it in logging
		log.Printf("Idempotency key: %s", req.IdempotencyKey)
	}

	// Process transfer through orchestration
	result, err := h.orchestrator.ProcessTransfer(r.Context(), req, transactionID)
	if err != nil {
		log.Printf("Transfer failed: %v", err)
		h.errorResponse(w, fmt.Sprintf("Transfer processing failed: %v", err), http.StatusInternalServerError)
		return
	}

	// Return response
	w.Header().Set("Content-Type", "application/json")

	// Set status code based on result
	statusCode := http.StatusAccepted
	if result.Status == string(models.StatusBlocked) {
		statusCode = http.StatusForbidden
	}

	w.WriteHeader(statusCode)
	json.NewEncoder(w).Encode(result)
}

// GetTransactionHandler retrieves transaction details
func (h *Handler) GetTransactionHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	txID := vars["id"]

	if txID == "" {
		h.errorResponse(w, "Transaction ID is required", http.StatusBadRequest)
		return
	}

	// TODO: Query database for actual transaction
	// For MVP, return mock data
	response := models.Transaction{
		ID:                txID,
		SenderBank:        "ICICI",
		ReceiverBank:      "ENBD",
		Amount:            100000,
		FromCurrency:      "INR",
		ToCurrency:        "AED",
		FXRate:            0.044,
		Status:            models.StatusCompleted,
		InstantSettlement: true,
		CreatedAt:         time.Now().Add(-5 * time.Minute),
	}

	completedAt := time.Now().Add(-4 * time.Minute)
	response.CompletedAt = &completedAt

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// GetTransactionsHandler retrieves list of transactions
func (h *Handler) GetTransactionsHandler(w http.ResponseWriter, r *http.Request) {
	// Get bank ID from context (set by auth middleware)
	bankID := middleware.GetBankIDFromContext(r.Context())

	// TODO: Query database for transactions
	// For MVP, return mock data
	transactions := []models.Transaction{
		{
			ID:                "TXN-12345678",
			SenderBank:        bankID,
			ReceiverBank:      "ENBD",
			Amount:            100000,
			FromCurrency:      "INR",
			ToCurrency:        "AED",
			FXRate:            0.044,
			Status:            models.StatusCompleted,
			InstantSettlement: true,
			CreatedAt:         time.Now().Add(-1 * time.Hour),
		},
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(transactions)
}

// GetBanksHandler retrieves list of supported banks
func (h *Handler) GetBanksHandler(w http.ResponseWriter, r *http.Request) {
	banks := []models.Bank{
		{ID: "1", Code: "ICICI", Name: "ICICI Bank Limited", Country: "IN", Active: true},
		{ID: "2", Code: "ENBD", Name: "Emirates NBD Bank", Country: "AE", Active: true},
		{ID: "3", Code: "ADCB", Name: "Abu Dhabi Commercial Bank", Country: "AE", Active: true},
		{ID: "4", Code: "AXIS", Name: "Axis Bank Limited", Country: "IN", Active: true},
		{ID: "5", Code: "HDFC", Name: "HDFC Bank Limited", Country: "IN", Active: true},
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(banks)
}

// GetCorridorsHandler retrieves list of supported corridors
func (h *Handler) GetCorridorsHandler(w http.ResponseWriter, r *http.Request) {
	corridors := []models.Corridor{
		{
			ID:                "1",
			Name:              "INR-AED",
			FromCurrency:      "INR",
			ToCurrency:        "AED",
			Active:            true,
			InstantSettlement: true,
			MaxAmount:         1000000,
		},
		{
			ID:                "2",
			Name:              "AED-INR",
			FromCurrency:      "AED",
			ToCurrency:        "INR",
			Active:            true,
			InstantSettlement: true,
			MaxAmount:         100000,
		},
		{
			ID:                "3",
			Name:              "USD-AED",
			FromCurrency:      "USD",
			ToCurrency:        "AED",
			Active:            true,
			InstantSettlement: true,
			MaxAmount:         500000,
		},
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(corridors)
}

// GetRatesHandler retrieves FX rates for a corridor
func (h *Handler) GetRatesHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	corridor := vars["corridor"]

	if corridor == "" {
		h.errorResponse(w, "Corridor is required", http.StatusBadRequest)
		return
	}

	rate := h.getFXRate(corridor)

	response := models.FXRate{
		Corridor:  corridor,
		Rate:      rate,
		Spread:    0.001,
		UpdatedAt: time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// LoginHandler handles authentication
func (h *Handler) LoginHandler(w http.ResponseWriter, r *http.Request) {
	var loginReq struct {
		BankID   string `json:"bank_id"`
		Password string `json:"password"`
	}

	if err := json.NewDecoder(r.Body).Decode(&loginReq); err != nil {
		h.errorResponse(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// TODO: Validate credentials against database
	// For MVP, accept any bank ID with password "demo"
	if loginReq.Password != "demo" {
		h.errorResponse(w, "Invalid credentials", http.StatusUnauthorized)
		return
	}

	// Generate JWT token
	token, err := h.authMiddleware.GenerateToken(loginReq.BankID, "bank")
	if err != nil {
		h.errorResponse(w, "Failed to generate token", http.StatusInternalServerError)
		return
	}

	response := map[string]interface{}{
		"token":      token,
		"bank_id":    loginReq.BankID,
		"expires_in": 86400, // 24 hours
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// Helper methods

func (h *Handler) validateTransferRequest(req *models.TransferRequest) error {
	if req.SenderBank == "" {
		return fmt.Errorf("sender_bank is required")
	}
	if req.ReceiverBank == "" {
		return fmt.Errorf("receiver_bank is required")
	}
	if req.Amount <= 0 {
		return fmt.Errorf("amount must be positive")
	}
	if req.FromCurrency == "" {
		return fmt.Errorf("from_currency is required")
	}
	if req.ToCurrency == "" {
		return fmt.Errorf("to_currency is required")
	}
	if req.SenderAccount == "" {
		return fmt.Errorf("sender_account is required")
	}
	if req.ReceiverAccount == "" {
		return fmt.Errorf("receiver_account is required")
	}
	return nil
}

func (h *Handler) getFXRate(corridor string) float64 {
	rates := map[string]float64{
		"INR-AED": 0.044,
		"AED-INR": 22.73,
		"USD-AED": 3.67,
		"AED-USD": 0.27,
		"INR-USD": 0.012,
		"USD-INR": 83.33,
	}

	if rate, ok := rates[corridor]; ok {
		return rate
	}
	return 1.0
}

func (h *Handler) errorResponse(w http.ResponseWriter, message string, code int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(models.ErrorResponse{
		Error:   "error",
		Message: message,
		Code:    code,
	})
}
