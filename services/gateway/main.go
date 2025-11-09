package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"github.com/gorilla/mux"
	"github.com/joho/godotenv"
)

// TransferRequest represents incoming transfer request
type TransferRequest struct {
	SenderBank     string  `json:"sender_bank"`
	ReceiverBank   string  `json:"receiver_bank"`
	Amount         float64 `json:"amount"`
	FromCurrency   string  `json:"from_currency"`
	ToCurrency     string  `json:"to_currency"`
	Reference      string  `json:"reference"`
	SenderAccount  string  `json:"sender_account"`
	ReceiverAccount string `json:"receiver_account"`
}

// TransferResponse represents the response for a transfer request
type TransferResponse struct {
	TransactionID    string    `json:"transaction_id"`
	Status           string    `json:"status"`
	Message          string    `json:"message"`
	InstantSettlement bool     `json:"instant_settlement"`
	EstimatedTime    string    `json:"estimated_time"`
	CreatedAt        time.Time `json:"created_at"`
}

// Health check response
type HealthResponse struct {
	Status  string `json:"status"`
	Service string `json:"service"`
	Version string `json:"version"`
	Uptime  string `json:"uptime"`
}

var startTime = time.Now()

func main() {
	// Load .env file
	godotenv.Load()

	// Get port from environment
	port := os.Getenv("GATEWAY_PORT")
	if port == "" {
		port = "8080"
	}

	// Create router
	router := mux.NewRouter()

	// Register routes
	router.HandleFunc("/health", healthHandler).Methods("GET")
	router.HandleFunc("/api/v1/transfer", transferHandler).Methods("POST")
	router.HandleFunc("/api/v1/transaction/{id}", getTransactionHandler).Methods("GET")
	router.HandleFunc("/api/v1/banks", getBanksHandler).Methods("GET")
	router.HandleFunc("/api/v1/corridors", getCorridorsHandler).Methods("GET")
	router.HandleFunc("/api/v1/rates/{corridor}", getRatesHandler).Methods("GET")

	// CORS middleware
	router.Use(corsMiddleware)
	router.Use(loggingMiddleware)

	log.Printf("Gateway Service starting on port %s", port)
	log.Fatal(http.ListenAndServe(":"+port, router))
}

// Health check handler
func healthHandler(w http.ResponseWriter, r *http.Request) {
	response := HealthResponse{
		Status:  "healthy",
		Service: "gateway",
		Version: "1.0.0",
		Uptime:  time.Since(startTime).String(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// Transfer handler - main endpoint for initiating transfers
func transferHandler(w http.ResponseWriter, r *http.Request) {
	var req TransferRequest

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request body", http.StatusBadRequest)
		return
	}

	// Validate request
	if req.Amount <= 0 {
		http.Error(w, "Amount must be positive", http.StatusBadRequest)
		return
	}

	// Generate transaction ID
	txID := fmt.Sprintf("TXN-%d", time.Now().UnixNano())

	// TODO: Call Token Engine to mint tokens
	// TODO: Call Obligation Engine to create obligation
	// TODO: Call Risk Engine for risk assessment
	// TODO: Call Liquidity Router for instant settlement decision

	// For now, return mock response
	response := TransferResponse{
		TransactionID:     txID,
		Status:           "PROCESSING",
		Message:          "Transaction initiated successfully",
		InstantSettlement: true,
		EstimatedTime:    "5-30 seconds",
		CreatedAt:        time.Now(),
	}

	log.Printf("Transfer initiated: %s from %s to %s, amount: %.2f %s",
		txID, req.SenderBank, req.ReceiverBank, req.Amount, req.FromCurrency)

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusAccepted)
	json.NewEncoder(w).Encode(response)
}

// Get transaction status
func getTransactionHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	txID := vars["id"]

	// TODO: Query database for actual transaction
	// For now, return mock data
	response := map[string]interface{}{
		"transaction_id": txID,
		"status":        "COMPLETED",
		"amount":        100000,
		"from_currency": "INR",
		"to_currency":   "AED",
		"fx_rate":       0.044,
		"created_at":    time.Now().Add(-5 * time.Minute),
		"completed_at":  time.Now().Add(-4 * time.Minute),
		"instant_settlement": true,
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// Get supported banks
func getBanksHandler(w http.ResponseWriter, r *http.Request) {
	banks := []map[string]string{
		{"code": "ICICI", "name": "ICICI Bank Limited", "country": "IN"},
		{"code": "ENBD", "name": "Emirates NBD Bank", "country": "AE"},
		{"code": "ADCB", "name": "Abu Dhabi Commercial Bank", "country": "AE"},
		{"code": "AXIS", "name": "Axis Bank Limited", "country": "IN"},
		{"code": "HDFC", "name": "HDFC Bank Limited", "country": "IN"},
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(banks)
}

// Get supported corridors
func getCorridorsHandler(w http.ResponseWriter, r *http.Request) {
	corridors := []map[string]interface{}{
		{
			"corridor": "INR-AED",
			"active": true,
			"instant_settlement": true,
			"max_amount": 1000000,
		},
		{
			"corridor": "AED-INR",
			"active": true,
			"instant_settlement": true,
			"max_amount": 100000,
		},
		{
			"corridor": "USD-AED",
			"active": true,
			"instant_settlement": true,
			"max_amount": 500000,
		},
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(corridors)
}

// Get FX rates for a corridor
func getRatesHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	corridor := vars["corridor"]

	rates := map[string]interface{}{
		"corridor": corridor,
		"rate": getFXRate(corridor),
		"spread": 0.001,
		"updated_at": time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(rates)
}

// Helper function to get FX rates
func getFXRate(corridor string) float64 {
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

// CORS middleware
func corsMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Access-Control-Allow-Origin", "*")
		w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS")
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization")

		if r.Method == "OPTIONS" {
			w.WriteHeader(http.StatusOK)
			return
		}

		next.ServeHTTP(w, r)
	})
}

// Logging middleware
func loggingMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		start := time.Now()

		next.ServeHTTP(w, r)

		log.Printf("%s %s %s %v", r.Method, r.URL.Path, r.RemoteAddr, time.Since(start))
	})
}