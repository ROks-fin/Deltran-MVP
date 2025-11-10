package main

import (
	"encoding/json"
	"fmt"
	"log"
	"net/http"
	"os"
	"time"

	"deltran/gateway/middleware"

	"github.com/gorilla/mux"
	"github.com/joho/godotenv"
)

// Configuration holds the application configuration
type Config struct {
	Port              string
	JWTSecret         string
	AnalyticsURL      string
	EnableAuth        bool
	EnableRateLimit   bool
	EnableAnalytics   bool
}

// TransferRequest represents incoming transfer request
type TransferRequest struct {
	SenderBank      string  `json:"sender_bank"`
	ReceiverBank    string  `json:"receiver_bank"`
	Amount          float64 `json:"amount"`
	FromCurrency    string  `json:"from_currency"`
	ToCurrency      string  `json:"to_currency"`
	Reference       string  `json:"reference"`
	SenderAccount   string  `json:"sender_account"`
	ReceiverAccount string  `json:"receiver_account"`
}

// TransferResponse represents the response for a transfer request
type TransferResponse struct {
	TransactionID     string    `json:"transaction_id"`
	Status            string    `json:"status"`
	Message           string    `json:"message"`
	InstantSettlement bool      `json:"instant_settlement"`
	EstimatedTime     string    `json:"estimated_time"`
	CreatedAt         time.Time `json:"created_at"`
}

// HealthResponse for health check
type HealthResponse struct {
	Status  string `json:"status"`
	Service string `json:"service"`
	Version string `json:"version"`
	Uptime  string `json:"uptime"`
}

var (
	startTime time.Time
	config    *Config
	analytics *middleware.AnalyticsCollector
)

func init() {
	startTime = time.Now()
}

func main() {
	// Load environment variables
	godotenv.Load()

	// Load configuration
	config = loadConfig()

	// Initialize analytics collector
	if config.EnableAnalytics {
		analytics = middleware.NewAnalyticsCollector(config.AnalyticsURL)
		log.Printf("✅ Analytics collector enabled: %s", config.AnalyticsURL)
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

	// Apply global middleware
	router.Use(corsMiddleware)
	router.Use(loggingMiddleware)
	router.Use(securityHeadersMiddleware)

	// Apply analytics middleware if enabled
	if config.EnableAnalytics {
		router.Use(analytics.Middleware)
		log.Println("✅ Analytics middleware enabled")
	}

	// Apply rate limiting if enabled
	if config.EnableRateLimit {
		tieredLimiter := middleware.NewTieredRateLimiter()
		router.Use(tieredLimiter.Middleware)
		log.Println("✅ Tiered rate limiting enabled")
	}

	// Apply JWT authentication if enabled
	if config.EnableAuth {
		jwtConfig := middleware.NewJWTConfig(config.JWTSecret)
		router.Use(jwtConfig.AuthMiddleware)
		log.Println("✅ JWT authentication enabled")
	}

	// Protected routes with specific permissions
	apiRouter := router.PathPrefix("/api/v1").Subrouter()
	if config.EnableAuth {
		// Transactions require write permission
		apiRouter.HandleFunc("/transfer", transferHandler).
			Methods("POST").
			Handler(middleware.RequirePermission("transactions:write")(http.HandlerFunc(transferHandler)))
	}

	log.Printf("🚀 Gateway Service starting on port %s", config.Port)
	log.Printf("📊 Configuration:")
	log.Printf("   - Auth: %v", config.EnableAuth)
	log.Printf("   - Rate Limiting: %v", config.EnableRateLimit)
	log.Printf("   - Analytics: %v", config.EnableAnalytics)
	log.Printf("   - JWT Secret: %s", maskSecret(config.JWTSecret))

	log.Fatal(http.ListenAndServe(":"+config.Port, router))
}

func loadConfig() *Config {
	return &Config{
		Port:              getEnv("GATEWAY_PORT", "8080"),
		JWTSecret:         getEnv("JWT_SECRET", "deltran-secret-key-change-in-production"),
		AnalyticsURL:      getEnv("ANALYTICS_URL", "http://localhost:8093"),
		EnableAuth:        getEnv("ENABLE_AUTH", "false") == "true",
		EnableRateLimit:   getEnv("ENABLE_RATE_LIMIT", "true") == "true",
		EnableAnalytics:   getEnv("ENABLE_ANALYTICS", "true") == "true",
	}
}

func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func maskSecret(secret string) string {
	if len(secret) <= 8 {
		return "****"
	}
	return secret[:4] + "..." + secret[len(secret)-4:]
}

// Health check handler
func healthHandler(w http.ResponseWriter, r *http.Request) {
	response := HealthResponse{
		Status:  "healthy",
		Service: "gateway-enhanced",
		Version: "2.0.0",
		Uptime:  time.Since(startTime).String(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(response)
}

// Transfer handler
func transferHandler(w http.ResponseWriter, r *http.Request) {
	var req TransferRequest

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		respondWithError(w, http.StatusBadRequest, "Invalid request body")
		return
	}

	// Validate request
	if req.Amount <= 0 {
		respondWithError(w, http.StatusBadRequest, "Amount must be positive")
		return
	}

	if req.SenderBank == "" || req.ReceiverBank == "" {
		respondWithError(w, http.StatusBadRequest, "Sender and receiver banks are required")
		return
	}

	// Generate transaction ID
	txID := fmt.Sprintf("TXN-%d", time.Now().UnixNano())

	// Record transaction in analytics if enabled
	if config.EnableAnalytics && analytics != nil {
		go analytics.CreateTransaction(
			txID,
			req.SenderAccount,
			req.ReceiverAccount,
			req.FromCurrency,
			req.Amount,
		)
	}

	// Get user info from headers (if authenticated)
	userID := r.Header.Get("X-User-ID")
	userRole := r.Header.Get("X-User-Role")

	log.Printf("📤 Transfer initiated: %s from %s to %s, amount: %.2f %s (user: %s, role: %s)",
		txID, req.SenderBank, req.ReceiverBank, req.Amount, req.FromCurrency, userID, userRole)

	// TODO: Call downstream services
	// - Token Engine to mint tokens
	// - Obligation Engine to create obligation
	// - Risk Engine for risk assessment
	// - Liquidity Router for instant settlement decision

	response := TransferResponse{
		TransactionID:     txID,
		Status:            "PROCESSING",
		Message:           "Transaction initiated successfully",
		InstantSettlement: true,
		EstimatedTime:     "5-30 seconds",
		CreatedAt:         time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("X-Transaction-ID", txID)
	w.WriteHeader(http.StatusAccepted)
	json.NewEncoder(w).Encode(response)
}

// Get transaction status
func getTransactionHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	txID := vars["id"]

	// TODO: Query actual transaction from database
	response := map[string]interface{}{
		"transaction_id":      txID,
		"status":              "COMPLETED",
		"amount":              100000,
		"from_currency":       "INR",
		"to_currency":         "AED",
		"fx_rate":             0.044,
		"created_at":          time.Now().Add(-5 * time.Minute),
		"completed_at":        time.Now().Add(-4 * time.Minute),
		"instant_settlement":  true,
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
			"corridor":            "INR-AED",
			"active":              true,
			"instant_settlement":  true,
			"max_amount":          1000000,
		},
		{
			"corridor":            "AED-INR",
			"active":              true,
			"instant_settlement":  true,
			"max_amount":          100000,
		},
		{
			"corridor":            "USD-AED",
			"active":              true,
			"instant_settlement":  true,
			"max_amount":          500000,
		},
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(corridors)
}

// Get FX rates
func getRatesHandler(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	corridor := vars["corridor"]

	rates := map[string]interface{}{
		"corridor":   corridor,
		"rate":       getFXRate(corridor),
		"spread":     0.001,
		"updated_at": time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(rates)
}

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
		w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Transaction-ID")
		w.Header().Set("Access-Control-Expose-Headers", "X-Transaction-ID, X-RateLimit-Limit, X-RateLimit-Remaining")

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
		log.Printf("📊 %s %s %s %v", r.Method, r.URL.Path, r.RemoteAddr, time.Since(start))
	})
}

// Security headers middleware
func securityHeadersMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("X-Frame-Options", "DENY")
		w.Header().Set("X-Content-Type-Options", "nosniff")
		w.Header().Set("X-XSS-Protection", "1; mode=block")
		w.Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")
		next.ServeHTTP(w, r)
	})
}

func respondWithError(w http.ResponseWriter, code int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{
		"error": message,
		"code":  fmt.Sprintf("%d", code),
	})
}
