package main

import (
	"context"
	"database/sql"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/deltran/reporting-engine/internal/api"
	"github.com/deltran/reporting-engine/internal/config"
	"github.com/deltran/reporting-engine/internal/generators"
	"github.com/deltran/reporting-engine/internal/reports"
	"github.com/deltran/reporting-engine/internal/scheduler"
	"github.com/deltran/reporting-engine/internal/storage"
	"github.com/gorilla/mux"
	_ "github.com/lib/pq"
	"go.uber.org/zap"
)

func main() {
	// Initialize logger
	logger, err := zap.NewProduction()
	if err != nil {
		panic(fmt.Sprintf("Failed to initialize logger: %v", err))
	}
	defer logger.Sync()

	logger.Info("Starting DelTran Reporting Engine")

	// Load configuration
	cfg, err := loadConfig()
	if err != nil {
		logger.Fatal("Failed to load configuration", zap.Error(err))
	}

	logger.Info("Configuration loaded",
		zap.String("port", cfg.Server.Port),
		zap.String("db_host", cfg.Database.Host),
		zap.String("s3_bucket", cfg.S3.Bucket))

	// Initialize storage
	store, err := storage.NewStorage(cfg, logger)
	if err != nil {
		logger.Fatal("Failed to initialize storage", zap.Error(err))
	}
	defer store.Close()

	logger.Info("Storage initialized successfully")

	// Initialize generators
	excelGen := generators.NewExcelGenerator(logger)
	csvGen := generators.NewCSVGenerator(nil, logger) // DB will be injected per report

	// Initialize database connection for generators
	db, err := initDatabase(cfg.DatabaseDSN())
	if err != nil {
		logger.Fatal("Failed to initialize database for generators", zap.Error(err))
	}
	csvGen = generators.NewCSVGenerator(db, logger)

	// Initialize report generators
	reportGenerators := make(map[string]scheduler.ReportGenerator)
	reportGenerators["aml"] = reports.NewAMLReportGenerator(db, excelGen, csvGen, logger)
	reportGenerators["settlement"] = reports.NewSettlementReportGenerator(db, excelGen, csvGen, logger)
	// Add more report types as needed

	logger.Info("Report generators initialized", zap.Int("count", len(reportGenerators)))

	// Initialize scheduler
	reportScheduler := scheduler.NewReportScheduler(
		reportGenerators,
		store,
		cfg.Scheduler.MaxConcurrent,
		logger,
	)

	if cfg.Scheduler.Enabled {
		ctx := context.Background()
		if err := reportScheduler.Start(ctx); err != nil {
			logger.Fatal("Failed to start scheduler", zap.Error(err))
		}
		logger.Info("Report scheduler started")
	}

	// Initialize HTTP server
	router := mux.NewRouter()

	// Initialize handlers
	reportHandler := api.NewReportHandler(store, reportGenerators, logger)
	reportHandler.RegisterRoutes(router)

	// Add middleware
	router.Use(loggingMiddleware(logger))
	router.Use(corsMiddleware)

	// Create HTTP server
	srv := &http.Server{
		Addr:         fmt.Sprintf(":%s", cfg.Server.Port),
		Handler:      router,
		ReadTimeout:  cfg.Server.ReadTimeout,
		WriteTimeout: cfg.Server.WriteTimeout,
	}

	// Start server in goroutine
	go func() {
		logger.Info("HTTP server starting", zap.String("port", cfg.Server.Port))
		if err := srv.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal("HTTP server error", zap.Error(err))
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	logger.Info("Shutting down gracefully...")

	// Stop scheduler
	if cfg.Scheduler.Enabled {
		reportScheduler.Stop()
	}

	// Shutdown HTTP server
	ctx, cancel := context.WithTimeout(context.Background(), 10*time.Second)
	defer cancel()

	if err := srv.Shutdown(ctx); err != nil {
		logger.Error("Server forced to shutdown", zap.Error(err))
	}

	logger.Info("Server exited successfully")
}

func loadConfig() (*config.Config, error) {
	configPath := os.Getenv("CONFIG_PATH")
	if configPath == "" {
		// Try to load from default locations
		if _, err := os.Stat("config.yaml"); err == nil {
			configPath = "config.yaml"
		}
		// If no config file found, Load will use defaults from Viper
	}

	return config.Load(configPath)
}

func initDatabase(dsn string) (*sql.DB, error) {
	db, err := sql.Open("postgres", dsn)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	if err := db.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	db.SetMaxOpenConns(20)
	db.SetMaxIdleConns(5)
	db.SetConnMaxLifetime(5 * time.Minute)

	return db, nil
}

func loggingMiddleware(logger *zap.Logger) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			start := time.Now()

			// Wrap response writer to capture status code
			wrapped := &responseWriter{ResponseWriter: w, statusCode: http.StatusOK}

			next.ServeHTTP(wrapped, r)

			duration := time.Since(start)

			logger.Info("HTTP request",
				zap.String("method", r.Method),
				zap.String("path", r.URL.Path),
				zap.Int("status", wrapped.statusCode),
				zap.Duration("duration", duration),
				zap.String("remote_addr", r.RemoteAddr))
		})
	}
}

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

type responseWriter struct {
	http.ResponseWriter
	statusCode int
}

func (rw *responseWriter) WriteHeader(code int) {
	rw.statusCode = code
	rw.ResponseWriter.WriteHeader(code)
}
