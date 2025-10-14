// Gateway server entry point with full integration
package main

import (
	"context"
	"crypto/rand"
	"database/sql"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/go-chi/chi/v5/middleware"
	_ "github.com/lib/pq"
	"github.com/redis/go-redis/v9"
	"github.com/rs/zerolog"
	"github.com/rs/zerolog/log"
	"github.com/shopspring/decimal"

	"github.com/deltran/gateway/internal/audit"
	"github.com/deltran/gateway/internal/auth"
	"github.com/deltran/gateway/internal/integration"
	"github.com/deltran/gateway/internal/resilience"
	"github.com/deltran/gateway/internal/server"
	"github.com/deltran/gateway/internal/swift"
)

// Config holds application configuration
type Config struct {
	Environment string
	HTTPAddr    string
	Database    DatabaseConfig
	Redis       RedisConfig
	JWTSecret   string
	CORSOrigins []string
}

// DatabaseConfig holds database connection configuration
type DatabaseConfig struct {
	Host     string
	Port     int
	User     string
	Password string
	Database string
	SSLMode  string
}

// RedisConfig holds Redis connection configuration
type RedisConfig struct {
	Addr     string
	Password string
	DB       int
}

func main() {
	// Initialize structured logger
	zerolog.TimeFieldFormat = zerolog.TimeFormatUnix
	if os.Getenv("ENVIRONMENT") == "development" {
		log.Logger = log.Output(zerolog.ConsoleWriter{Out: os.Stderr, TimeFormat: time.RFC3339})
	}

	log.Info().Msg("Starting DelTran Gateway with full integration")

	// Load configuration from environment
	cfg := loadConfig()

	log.Info().
		Str("environment", cfg.Environment).
		Str("http_addr", cfg.HTTPAddr).
		Msg("Configuration loaded")

	// ========== DATABASE CONNECTION ==========
	log.Info().Msg("Connecting to PostgreSQL database...")
	db, err := connectDatabase(cfg.Database)
	if err != nil {
		log.Fatal().Err(err).Msg("Failed to connect to database")
	}
	defer db.Close()
	log.Info().Msg("✓ Database connection established")

	// ========== REDIS CONNECTION ==========
	log.Info().Msg("Connecting to Redis cache...")
	redisClient := connectRedis(cfg.Redis)
	defer redisClient.Close()

	ctx := context.Background()
	if err := redisClient.Ping(ctx).Err(); err != nil {
		log.Fatal().Err(err).Msg("Failed to connect to Redis")
	}
	log.Info().Msg("✓ Redis connection established")

	// ========== SYSTEM VALIDATION ==========
	log.Info().Msg("Validating system integration...")
	healthChecker := integration.NewHealthChecker(db, redisClient)

	if errs := healthChecker.FullValidation(ctx); len(errs) > 0 {
		log.Error().Msg("System validation failed:")
		for _, err := range errs {
			log.Error().Err(err).Msg("")
		}
		log.Fatal().Msg("Cannot start gateway with validation errors")
	}
	log.Info().Msg("✓ All system components validated successfully")

	// ========== RESILIENCE LAYER ==========
	log.Info().Msg("Initializing resilience patterns...")

	// Circuit breaker manager for external services
	cbManager := resilience.NewCircuitBreakerManager()

	// Idempotency manager with 24-hour TTL
	idempotencyMgr := resilience.NewIdempotencyManager(redisClient, 24*time.Hour)

	log.Info().Msg("✓ Circuit breakers and idempotency protection initialized")

	// ========== AUTHENTICATION LAYER ==========
	log.Info().Msg("Initializing authentication services...")

	jwtManager := auth.NewJWTManager(cfg.JWTSecret)
	sessionManager := auth.NewSessionManager(redisClient)
	totpManager := auth.NewTOTPManager()
	_ = auth.NewRateLimiter(redisClient, 100, 10) // 100 req/min, burst 10

	log.Info().Msg("✓ Authentication services initialized")

	// ========== WEBSOCKET HUB ==========
	log.Info().Msg("Initializing WebSocket hub for live updates...")
	wsHub := server.NewWebSocketHub(nil) // NATS connection will be added later
	wsCtx, wsCancel := context.WithCancel(context.Background())
	defer wsCancel()
	go wsHub.Run(wsCtx)
	log.Info().Msg("✓ WebSocket hub initialized")

	// ========== HTTP ROUTER SETUP ==========
	router := chi.NewRouter()

	// Global middleware
	router.Use(middleware.RequestID)
	router.Use(middleware.RealIP)
	router.Use(middleware.Logger)
	router.Use(middleware.Recoverer)
	router.Use(middleware.Timeout(60 * time.Second))

	// CORS middleware
	router.Use(corsMiddleware(cfg.CORSOrigins))

	// Security headers middleware
	router.Use(auth.SecurityHeadersMiddleware())

	// ========== PUBLIC ROUTES ==========
	router.Group(func(r chi.Router) {
		r.Get("/health", healthCheckHandler(healthChecker))
		r.Get("/health/live", livenessHandler())
		r.Get("/health/ready", readinessHandler(db, redisClient))
	})

	// ========== AUTH ROUTES ==========
	authHandler := auth.NewAuthHandler(db, jwtManager, totpManager, sessionManager)

	router.Group(func(r chi.Router) {
		r.Post("/api/v1/auth/login", authHandler.Login)
		r.Post("/api/v1/auth/logout", authHandler.Logout)
		r.Post("/api/v1/auth/refresh", authHandler.RefreshToken)
		r.Post("/api/v1/auth/2fa/setup", authHandler.Setup2FA)
		r.Post("/api/v1/auth/2fa/verify", authHandler.Verify2FA)
		r.Post("/api/v1/auth/2fa/disable", authHandler.Disable2FA)
		r.Get("/api/v1/auth/sessions", authHandler.GetSessions)
		r.Post("/api/v1/auth/sessions/revoke", authHandler.RevokeSession)
	})

	// ========== AGGREGATION API ==========
	aggregationAPI := server.NewAggregationAPI(db, redisClient, wsHub)

	// ========== AUDIT EXPORTER ==========
	auditExporter := audit.NewAuditExporter(db)

	// ========== STRESS TEST ROUTE (NO AUTH) ==========
	router.Post("/api/v1/payments/initiate", createPaymentHandlerNoAuth(db, idempotencyMgr, cbManager))

	// ========== PROTECTED ROUTES ==========
	router.Group(func(r chi.Router) {
		r.Use(auth.JWTMiddleware(jwtManager))

		// Aggregation & Metrics API (dashboard, reporting)
		aggregationAPI.RegisterRoutes(r)

		// Big Four Audit Report Export API
		r.Post("/api/v1/audit/export/trail", exportAuditTrailHandler(auditExporter))
		r.Post("/api/v1/audit/export/ledger", exportTransactionLedgerHandler(auditExporter))
		r.Post("/api/v1/audit/export/reconciliation", exportReconciliationHandler(auditExporter))

		// Settlement routes (requires batch finalize permission)
		r.With(auth.RequirePermission(auth.PermBatchFinalize)).
			Post("/api/v1/settlement/execute", executeSettlementHandler(db, cbManager))

		// Admin routes (requires admin role)
		r.With(auth.RequireRole(auth.RoleAdmin)).
			Get("/api/v1/admin/users", listUsersHandler(db))
	})

	log.Info().Msg("✓ All routes registered")

	// ========== START HTTP SERVER ==========
	server := &http.Server{
		Addr:         cfg.HTTPAddr,
		Handler:      router,
		ReadTimeout:  30 * time.Second,
		WriteTimeout: 30 * time.Second,
		IdleTimeout:  120 * time.Second,
	}

	// Start server in goroutine
	go func() {
		log.Info().
			Str("addr", cfg.HTTPAddr).
			Msg("HTTP server listening")

		if err := server.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatal().Err(err).Msg("HTTP server failed")
		}
	}()

	// ========== GRACEFUL SHUTDOWN ==========
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
	<-sigChan

	log.Info().Msg("Shutdown signal received, shutting down gracefully...")

	shutdownCtx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	if err := server.Shutdown(shutdownCtx); err != nil {
		log.Error().Err(err).Msg("HTTP server shutdown error")
	}

	log.Info().Msg("Shutdown complete")
}

// loadConfig loads configuration from environment variables
func loadConfig() Config {
	cfg := Config{
		Environment: getEnv("ENVIRONMENT", "development"),
		HTTPAddr:    getEnv("HTTP_ADDR", ":8080"),
		JWTSecret:   getEnv("JWT_SECRET", "change-me-in-production"),
		CORSOrigins: []string{
			getEnv("CORS_ORIGIN", "http://localhost:3000"),
		},
		Database: DatabaseConfig{
			Host:     getEnv("DB_HOST", "localhost"),
			Port:     getEnvInt("DB_PORT", 5432),
			User:     getEnv("DB_USER", "deltran"),
			Password: getEnv("DB_PASSWORD", "deltran123"),
			Database: getEnv("DB_NAME", "deltran"),
			SSLMode:  getEnv("DB_SSLMODE", "disable"),
		},
		Redis: RedisConfig{
			Addr:     getEnv("REDIS_ADDR", "localhost:6379"),
			Password: getEnv("REDIS_PASSWORD", ""),
			DB:       getEnvInt("REDIS_DB", 0),
		},
	}

	return cfg
}

// getEnv gets environment variable with fallback
func getEnv(key, fallback string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return fallback
}

// getEnvInt gets integer environment variable with fallback
func getEnvInt(key string, fallback int) int {
	if value := os.Getenv(key); value != "" {
		var intVal int
		if _, err := fmt.Sscanf(value, "%d", &intVal); err == nil {
			return intVal
		}
	}
	return fallback
}

// connectDatabase establishes database connection with connection pooling
func connectDatabase(cfg DatabaseConfig) (*sql.DB, error) {
	dsn := fmt.Sprintf(
		"host=%s port=%d user=%s password=%s dbname=%s sslmode=%s",
		cfg.Host, cfg.Port, cfg.User, cfg.Password, cfg.Database, cfg.SSLMode,
	)

	db, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	// Configure connection pool
	db.SetMaxOpenConns(25)
	db.SetMaxIdleConns(5)
	db.SetConnMaxLifetime(5 * time.Minute)
	db.SetConnMaxIdleTime(10 * time.Minute)

	// Test connection
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	return db, nil
}

// connectRedis establishes Redis connection
func connectRedis(cfg RedisConfig) *redis.Client {
	return redis.NewClient(&redis.Options{
		Addr:         cfg.Addr,
		Password:     cfg.Password,
		DB:           cfg.DB,
		MaxRetries:   3,
		DialTimeout:  5 * time.Second,
		ReadTimeout:  3 * time.Second,
		WriteTimeout: 3 * time.Second,
		PoolSize:     10,
		MinIdleConns: 2,
	})
}

// corsMiddleware adds CORS headers
func corsMiddleware(origins []string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			origin := r.Header.Get("Origin")

			// Check if origin is allowed
			allowed := false
			for _, allowedOrigin := range origins {
				if origin == allowedOrigin || allowedOrigin == "*" {
					allowed = true
					break
				}
			}

			if allowed {
				w.Header().Set("Access-Control-Allow-Origin", origin)
			}

			w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS, PATCH")
			w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Request-ID, Idempotency-Key")
			w.Header().Set("Access-Control-Max-Age", "3600")
			w.Header().Set("Access-Control-Allow-Credentials", "true")

			// Handle preflight
			if r.Method == "OPTIONS" {
				w.WriteHeader(http.StatusOK)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// healthCheckHandler returns system health status
func healthCheckHandler(hc *integration.HealthChecker) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()
		health := hc.CheckSystemHealth(ctx)

		w.Header().Set("Content-Type", "application/json")
		if health.Healthy {
			w.WriteHeader(http.StatusOK)
		} else {
			w.WriteHeader(http.StatusServiceUnavailable)
		}

		// Return JSON response
		fmt.Fprintf(w, `{"healthy":%t,"components":[`, health.Healthy)
		for i, comp := range health.Components {
			if i > 0 {
				fmt.Fprint(w, ",")
			}
			fmt.Fprintf(w, `{"name":"%s","healthy":%t,"message":"%s"}`,
				comp.Name, comp.Healthy, comp.Message)
		}
		fmt.Fprint(w, "]}")
	}
}

// livenessHandler for Kubernetes liveness probe
func livenessHandler() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "alive")
	}
}

// readinessHandler for Kubernetes readiness probe
func readinessHandler(db *sql.DB, redisClient *redis.Client) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		// Check database
		if err := db.PingContext(ctx); err != nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			fmt.Fprintf(w, "database not ready: %v", err)
			return
		}

		// Check Redis
		if err := redisClient.Ping(ctx).Err(); err != nil {
			w.WriteHeader(http.StatusServiceUnavailable)
			fmt.Fprintf(w, "redis not ready: %v", err)
			return
		}

		w.WriteHeader(http.StatusOK)
		fmt.Fprint(w, "ready")
	}
}

// ========== PAYMENT HANDLERS ==========

// createPaymentHandlerNoAuth handles payment creation without auth (for stress testing)
func createPaymentHandlerNoAuth(db *sql.DB, idempotencyMgr *resilience.IdempotencyManager, cbManager *resilience.CircuitBreakerManager) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		ctx := r.Context()

		// Get idempotency key
		idempotencyKey := r.Header.Get("Idempotency-Key")
		if idempotencyKey == "" {
			http.Error(w, "Idempotency-Key header required", http.StatusBadRequest)
			return
		}

		// Execute with idempotency protection
		result, statusCode, err := idempotencyMgr.Execute(ctx, idempotencyKey, func() (interface{}, int, error) {
			// Parse request (simplified for example)
			type PaymentRequest struct {
				SenderBIC     string `json:"sender_bic"`
				ReceiverBIC   string `json:"receiver_bic"`
				Amount        string `json:"amount"`
				Currency      string `json:"currency"`
				OrderingCustomer string `json:"ordering_customer"`
				Beneficiary   string `json:"beneficiary"`
			}

			var req PaymentRequest
			if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
				return nil, http.StatusBadRequest, fmt.Errorf("invalid request: %w", err)
			}

			// Look up bank IDs from BIC codes
			var senderBankID, receiverBankID string
			err := db.QueryRowContext(ctx, "SELECT id FROM deltran.banks WHERE bic_code = $1", req.SenderBIC).Scan(&senderBankID)
			if err != nil {
				return nil, http.StatusBadRequest, fmt.Errorf("sender bank not found: %s", req.SenderBIC)
			}

			err = db.QueryRowContext(ctx, "SELECT id FROM deltran.banks WHERE bic_code = $1", req.ReceiverBIC).Scan(&receiverBankID)
			if err != nil {
				return nil, http.StatusBadRequest, fmt.Errorf("receiver bank not found: %s", req.ReceiverBIC)
			}

			// Generate SWIFT MT103 message
			amount, _ := decimal.NewFromString(req.Amount)
			mt103Builder := swift.NewMT103Builder()
			mt103, err := mt103Builder.
				SetSender(req.SenderBIC).
				SetReceiver(req.ReceiverBIC).
				SetReference(swift.GenerateReference("PAY")).
				SetValueDateCurrencyAmount(time.Now(), req.Currency, amount).
				SetOrderingCustomer(req.OrderingCustomer).
				SetBeneficiary("", req.Beneficiary).
				Build()

			if err != nil {
				return nil, http.StatusBadRequest, fmt.Errorf("invalid payment details: %w", err)
			}

			generator := swift.NewGenerator(req.SenderBIC)
			swiftMessage, err := generator.GenerateMT103(mt103)
			if err != nil {
				return nil, http.StatusInternalServerError, fmt.Errorf("failed to generate SWIFT message: %w", err)
			}

			// Store payment in database (with circuit breaker protection)
			cb := cbManager.Get("database", nil)
			err = cb.Execute(func() error {
				_, err := db.ExecContext(ctx, `
					INSERT INTO deltran.payments (
						id, payment_reference, sender_bank_id, receiver_bank_id, amount, currency,
						status, swift_message_type, swift_message_id, idempotency_key, created_at
					) VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, NOW())
				`,
					generateUUID(),
					mt103.SenderReference,
					senderBankID,
					receiverBankID,
					req.Amount,
					req.Currency,
					"pending",
					"MT103",
					mt103.SenderReference,
					idempotencyKey,
				)
				return err
			})

			if err != nil {
				return nil, http.StatusInternalServerError, fmt.Errorf("failed to store payment: %w", err)
			}

			return map[string]interface{}{
				"payment_id":      mt103.SenderReference,
				"swift_message":   swiftMessage[:100] + "...", // truncated for response
				"status":          "pending",
				"created_by":      "stress-test",
			}, http.StatusCreated, nil
		})

		if err != nil {
			log.Error().Err(err).Msg("Payment creation failed")
			http.Error(w, err.Error(), statusCode)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(statusCode)
		json.NewEncoder(w).Encode(result)
	}
}


// executeSettlementHandler executes settlement batch
func executeSettlementHandler(db *sql.DB, cbManager *resilience.CircuitBreakerManager) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// Implementation would include settlement logic
		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]string{
			"status": "settlement_initiated",
		})
	}
}

// listUsersHandler lists all users (admin only)
func listUsersHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		rows, err := db.QueryContext(r.Context(), `
			SELECT id, email, username, role, is_active, created_at
			FROM deltran.users
			ORDER BY created_at DESC
		`)

		if err != nil {
			log.Error().Err(err).Msg("Failed to query users")
			http.Error(w, "internal server error", http.StatusInternalServerError)
			return
		}
		defer rows.Close()

		var users []map[string]interface{}
		for rows.Next() {
			var u struct {
				ID        string
				Email     string
				Username  string
				Role      string
				IsActive  bool
				CreatedAt time.Time
			}

			if err := rows.Scan(&u.ID, &u.Email, &u.Username, &u.Role, &u.IsActive, &u.CreatedAt); err != nil {
				continue
			}

			users = append(users, map[string]interface{}{
				"id":         u.ID,
				"email":      u.Email,
				"username":   u.Username,
				"role":       u.Role,
				"is_active":  u.IsActive,
				"created_at": u.CreatedAt,
			})
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(map[string]interface{}{
			"users": users,
			"total": len(users),
		})
	}
}

// generateUUID generates a proper UUID v4
func generateUUID() string {
	b := make([]byte, 16)
	_, err := rand.Read(b)
	if err != nil {
		panic(err)
	}
	// Set version (4) and variant (2) bits
	b[6] = (b[6] & 0x0F) | 0x40
	b[8] = (b[8] & 0x3F) | 0x80

	return fmt.Sprintf("%s-%s-%s-%s-%s",
		hex.EncodeToString(b[0:4]),
		hex.EncodeToString(b[4:6]),
		hex.EncodeToString(b[6:8]),
		hex.EncodeToString(b[8:10]),
		hex.EncodeToString(b[10:16]))
}

// ========== AUDIT EXPORT HANDLERS ==========

// exportAuditTrailHandler exports complete audit trail for Big Four compliance
func exportAuditTrailHandler(exporter *audit.AuditExporter) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req audit.ExportRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request", http.StatusBadRequest)
			return
		}

		req.ReportType = "audit_trail"
		resp, err := exporter.ExportAuditTrail(r.Context(), req)
		if err != nil {
			log.Error().Err(err).Msg("Failed to export audit trail")
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}
}

// exportTransactionLedgerHandler exports immutable transaction ledger
func exportTransactionLedgerHandler(exporter *audit.AuditExporter) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req audit.ExportRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request", http.StatusBadRequest)
			return
		}

		req.ReportType = "transaction_ledger"
		resp, err := exporter.ExportTransactionLedger(r.Context(), req)
		if err != nil {
			log.Error().Err(err).Msg("Failed to export transaction ledger")
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}
}

// exportReconciliationHandler exports reconciliation reports
func exportReconciliationHandler(exporter *audit.AuditExporter) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		var req audit.ExportRequest
		if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
			http.Error(w, "invalid request", http.StatusBadRequest)
			return
		}

		req.ReportType = "reconciliation"
		resp, err := exporter.ExportReconciliation(r.Context(), req)
		if err != nil {
			log.Error().Err(err).Msg("Failed to export reconciliation")
			http.Error(w, err.Error(), http.StatusInternalServerError)
			return
		}

		w.Header().Set("Content-Type", "application/json")
		json.NewEncoder(w).Encode(resp)
	}
}