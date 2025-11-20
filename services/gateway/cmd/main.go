package main

import (
	"context"
	"log"
	"net/http"
	"os"
	"os/signal"
	"syscall"

	"deltran/gateway/internal/clients"
	"deltran/gateway/internal/config"
	"deltran/gateway/internal/handlers"
	"deltran/gateway/internal/middleware"
	"deltran/gateway/internal/orchestration"
	"deltran/gateway/internal/repository"

	"github.com/gorilla/mux"
)

func main() {
	log.Println("Starting DelTran Gateway Service...")

	// Load configuration
	cfg := config.Load()
	log.Printf("Configuration loaded. Port: %s", cfg.Server.Port)

	// Initialize bank repository
	log.Println("Initializing bank repository...")
	bankRepo, err := repository.NewBankRepository(cfg.Database.URL)
	if err != nil {
		log.Fatalf("Failed to initialize bank repository: %v", err)
	}
	log.Println("Bank repository initialized successfully")

	// Initialize service clients
	log.Println("Initializing service clients...")
	serviceClients := initializeServiceClients(cfg)

	// Initialize orchestrator
	log.Println("Initializing transaction orchestrator...")
	orchestrator := orchestration.NewTransactionOrchestrator(
		serviceClients.ComplianceClient,
		serviceClients.RiskClient,
		serviceClients.LiquidityClient,
		serviceClients.ObligationClient,
		serviceClients.TokenClient,
		serviceClients.NotificationClient,
		bankRepo,
	)

	// Initialize middleware
	log.Println("Initializing middleware...")
	authMiddleware := middleware.NewAuthMiddleware(cfg.Auth.JWTSecret)
	rateLimiter := middleware.NewRateLimiter(cfg.RateLimit.RequestsPerMinute, cfg.RateLimit.BurstSize)

	// Start cleanup goroutine for rate limiter
	go rateLimiter.CleanupOldLimiters()

	// Initialize handlers
	handler := handlers.NewHandler(orchestrator, authMiddleware)

	// Setup router
	router := setupRouter(handler, authMiddleware, rateLimiter)

	// Create HTTP server
	srv := &http.Server{
		Addr:         ":" + cfg.Server.Port,
		Handler:      router,
		ReadTimeout:  cfg.Server.ReadTimeout,
		WriteTimeout: cfg.Server.WriteTimeout,
		IdleTimeout:  cfg.Server.IdleTimeout,
	}

	// Start server in goroutine
	go func() {
		log.Printf("Gateway Service listening on port %s", cfg.Server.Port)
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			log.Fatalf("Failed to start server: %v", err)
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	log.Println("Shutting down server...")

	// Graceful shutdown
	ctx, cancel := context.WithTimeout(context.Background(), cfg.Server.ShutdownTimeout)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		log.Fatalf("Server forced to shutdown: %v", err)
	}

	log.Println("Server exited")
}

// ServiceClients holds all service clients
type ServiceClients struct {
	ComplianceClient   *clients.ComplianceClient
	RiskClient         *clients.RiskClient
	LiquidityClient    *clients.LiquidityClient
	ObligationClient   *clients.ObligationClient
	TokenClient        *clients.TokenClient
	ClearingClient     *clients.ClearingClient
	SettlementClient   *clients.SettlementClient
	NotificationClient *clients.NotificationClient
	ReportingClient    *clients.ReportingClient
}

// initializeServiceClients initializes all backend service clients
func initializeServiceClients(cfg *config.Config) *ServiceClients {
	return &ServiceClients{
		ComplianceClient:   clients.NewComplianceClient(cfg.Services.ComplianceEngine),
		RiskClient:         clients.NewRiskClient(cfg.Services.RiskEngine),
		LiquidityClient:    clients.NewLiquidityClient(cfg.Services.LiquidityRouter),
		ObligationClient:   clients.NewObligationClient(cfg.Services.ObligationEngine),
		TokenClient:        clients.NewTokenClient(cfg.Services.TokenEngine),
		ClearingClient:     clients.NewClearingClient(cfg.Services.ClearingEngine),
		SettlementClient:   clients.NewSettlementClient(cfg.Services.SettlementEngine),
		NotificationClient: clients.NewNotificationClient(cfg.Services.NotificationEngine),
		ReportingClient:    clients.NewReportingClient(cfg.Services.ReportingEngine),
	}
}

// setupRouter configures the HTTP router with all routes and middleware
func setupRouter(h *handlers.Handler, authMw *middleware.AuthMiddleware, rateLimiter *middleware.RateLimiter) *mux.Router {
	router := mux.NewRouter()

	// Global middleware (applied to all routes)
	router.Use(middleware.CORS)
	router.Use(middleware.Logging)
	router.Use(rateLimiter.Middleware)

	// Public routes (no authentication required)
	router.HandleFunc("/health", h.HealthCheck).Methods("GET")
	router.HandleFunc("/api/v1/auth/login", h.LoginHandler).Methods("POST")

	// Protected routes (authentication required)
	api := router.PathPrefix("/api/v1").Subrouter()
	api.Use(authMw.Middleware)

	// Transaction endpoints
	api.HandleFunc("/transfer", h.TransferHandler).Methods("POST")
	api.HandleFunc("/transaction/{id}", h.GetTransactionHandler).Methods("GET")
	api.HandleFunc("/transactions", h.GetTransactionsHandler).Methods("GET")

	// Bank endpoints
	api.HandleFunc("/banks", h.GetBanksHandler).Methods("GET")

	// Corridor and rates endpoints
	api.HandleFunc("/corridors", h.GetCorridorsHandler).Methods("GET")
	api.HandleFunc("/rates/{corridor}", h.GetRatesHandler).Methods("GET")

	return router
}
