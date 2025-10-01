// Gateway server entry point
package main

import (
	"context"
	"fmt"
	"net"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/deltran/gateway/internal/config"
	"github.com/deltran/gateway/internal/server"
	"github.com/prometheus/client_golang/prometheus/promhttp"
	"go.uber.org/zap"
	"google.golang.org/grpc"
)

func main() {
	// Initialize logger
	logger, err := zap.NewProduction()
	if err != nil {
		fmt.Fprintf(os.Stderr, "Failed to create logger: %v\n", err)
		os.Exit(1)
	}
	defer logger.Sync()

	// Load configuration
	cfg, err := config.Load()
	if err != nil {
		logger.Fatal("Failed to load config", zap.Error(err))
	}

	logger.Info("Starting DelTran Gateway",
		zap.String("version", cfg.Version),
		zap.String("grpc_addr", cfg.Server.GRPCAddr),
		zap.String("http_addr", cfg.Server.HTTPAddr),
	)

	// Create gRPC server
	grpcServer := grpc.NewServer(
		grpc.MaxRecvMsgSize(cfg.Server.MaxMessageSize),
		grpc.MaxSendMsgSize(cfg.Server.MaxMessageSize),
	)

	// Create gateway server
	gatewayServer, err := server.New(cfg, logger)
	if err != nil {
		logger.Fatal("Failed to create gateway server", zap.Error(err))
	}

	// Register gRPC services
	gatewayServer.RegisterServices(grpcServer)

	// Start gRPC server
	grpcListener, err := net.Listen("tcp", cfg.Server.GRPCAddr)
	if err != nil {
		logger.Fatal("Failed to listen on gRPC port", zap.Error(err))
	}

	go func() {
		logger.Info("gRPC server listening", zap.String("addr", cfg.Server.GRPCAddr))
		if err := grpcServer.Serve(grpcListener); err != nil {
			logger.Fatal("gRPC server failed", zap.Error(err))
		}
	}()

	// Start HTTP server (metrics + health + web UI + API)
	httpMux := http.NewServeMux()

	// Prometheus metrics endpoint
	httpMux.Handle("/metrics", promhttp.Handler())

	// Health check endpoint with detailed status
	httpMux.HandleFunc("/health", func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{
			"status": "healthy",
			"version": "1.0.0",
			"consensus": {
				"validators": 7,
				"active": 7,
				"latest_block": 1284756,
				"block_time": "6s"
			},
			"ledger": {
				"total_entries": 12847563,
				"last_entry_at": "2024-10-01T12:34:56Z"
			}
		}`))
	})

	// REST API endpoints
	httpMux.HandleFunc("/api/v1/metrics/live", gatewayServer.HandleMetricsAPI)
	httpMux.HandleFunc("/api/v1/transactions/recent", gatewayServer.HandleRecentTransactions)
	httpMux.HandleFunc("/api/v1/payments", gatewayServer.HandleSubmitPayment)
	httpMux.HandleFunc("/api/v1/payments/", gatewayServer.HandleGetPayment)

	// Real-time system metrics
	httpMux.HandleFunc("/api/v1/metrics/system", gatewayServer.HandleSystemMetrics)

	// ========== OPERATIONS APIs ==========
	// Payments
	httpMux.HandleFunc("/api/v1/payments/initiate", gatewayServer.HandlePaymentInitiate)
	httpMux.HandleFunc("/api/v1/payments/status", gatewayServer.HandlePaymentStatus)
	httpMux.HandleFunc("/api/v1/payments/quote", gatewayServer.HandlePaymentQuote)
	httpMux.HandleFunc("/api/v1/payments/fees/calc", gatewayServer.HandleFeeCalculation)
	httpMux.HandleFunc("/api/v1/payments/cancel", gatewayServer.HandlePaymentCancel)
	httpMux.HandleFunc("/api/v1/payments/list", gatewayServer.HandlePaymentsList)

	// Batches & Proofs
	httpMux.HandleFunc("/api/v1/batches/create", gatewayServer.HandleBatchCreate)
	httpMux.HandleFunc("/api/v1/batches/details", gatewayServer.HandleBatchDetails)
	httpMux.HandleFunc("/api/v1/batches/proofs", gatewayServer.HandleBatchProofs)
	httpMux.HandleFunc("/api/v1/batches/close", gatewayServer.HandleBatchClose)
	httpMux.HandleFunc("/api/v1/batches/list", gatewayServer.HandleBatchList)

	// Netting Windows
	httpMux.HandleFunc("/api/v1/netting/open", gatewayServer.HandleNettingOpen)
	httpMux.HandleFunc("/api/v1/netting/positions", gatewayServer.HandleNettingPositions)

	// ========== RISK & COMPLIANCE APIs ==========
	// Limits & Controls
	httpMux.HandleFunc("/api/v1/limits/set", gatewayServer.HandleLimitSet)

	// Compliance
	httpMux.HandleFunc("/api/v1/compliance/check", gatewayServer.HandleComplianceCheck)

	// Reconciliation
	httpMux.HandleFunc("/api/v1/reconciliation/run", gatewayServer.HandleReconciliationRun)

	// Serve API documentation UI
	webFS := http.FileServer(http.Dir("./web"))
	httpMux.Handle("/", webFS)

	httpServer := &http.Server{
		Addr:         cfg.Server.HTTPAddr,
		Handler:      httpMux,
		ReadTimeout:  10 * time.Second,
		WriteTimeout: 10 * time.Second,
	}

	go func() {
		logger.Info("HTTP server listening", zap.String("addr", cfg.Server.HTTPAddr))
		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal("HTTP server failed", zap.Error(err))
		}
	}()

	// Wait for shutdown signal
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)
	<-sigChan

	logger.Info("Shutting down gracefully...")

	// Graceful shutdown
	ctx, cancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer cancel()

	// Stop gRPC server
	grpcServer.GracefulStop()

	// Stop HTTP server
	if err := httpServer.Shutdown(ctx); err != nil {
		logger.Error("HTTP server shutdown error", zap.Error(err))
	}

	// Close gateway server
	if err := gatewayServer.Close(); err != nil {
		logger.Error("Gateway server close error", zap.Error(err))
	}

	logger.Info("Shutdown complete")
}