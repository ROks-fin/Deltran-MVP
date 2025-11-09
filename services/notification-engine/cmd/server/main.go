package main

import (
	"context"
	"fmt"
	"net/http"
	"os"
	"os/signal"
	"syscall"
	"time"

	"github.com/deltran/notification-engine/internal/api"
	"github.com/deltran/notification-engine/internal/config"
	"github.com/deltran/notification-engine/internal/consumer"
	"github.com/deltran/notification-engine/internal/dispatcher"
	"github.com/deltran/notification-engine/internal/storage"
	"github.com/deltran/notification-engine/internal/templates"
	"github.com/deltran/notification-engine/internal/websocket"
	"github.com/deltran/notification-engine/pkg/types"
	"github.com/google/uuid"
	"github.com/gorilla/mux"
	"go.uber.org/zap"
)

func main() {
	// Initialize logger
	logger, _ := zap.NewProduction()
	defer logger.Sync()

	logger.Info("Starting Notification Engine")

	// Load configuration
	cfg, err := config.Load("config.yaml")
	if err != nil {
		logger.Fatal("Failed to load config", zap.Error(err))
	}

	// Initialize Redis
	redisCache, err := storage.NewRedisCache(cfg.Redis.Address, cfg.Redis.Password, cfg.Redis.DB, logger)
	if err != nil {
		logger.Fatal("Failed to connect to Redis", zap.Error(err))
	}
	defer redisCache.Close()

	// Initialize PostgreSQL
	connStr := fmt.Sprintf("host=%s port=%d user=%s password=%s dbname=%s sslmode=%s",
		cfg.Database.Host, cfg.Database.Port, cfg.Database.User,
		cfg.Database.Password, cfg.Database.Name, cfg.Database.SSLMode)

	stor, err := storage.NewStorage(connStr, logger)
	if err != nil {
		logger.Fatal("Failed to connect to PostgreSQL", zap.Error(err))
	}
	defer stor.Close()

	// Initialize WebSocket Hub
	serverID := uuid.New().String()
	wsHub := websocket.NewHub(redisCache.GetClient(), serverID, logger)

	// Initialize template manager
	templateMgr := templates.NewManager(logger)

	// Initialize dispatchers
	emailSender := dispatcher.NewEmailSender(logger, cfg.Email.SMTPHost, cfg.Email.SMTPPort, cfg.Email.FromAddress, cfg.Email.FromName)
	smsSender := dispatcher.NewSMSSender(logger, cfg.SMS.MockMode, cfg.SMS.FromNumber)

	disp := dispatcher.NewDispatcher(logger, emailSender, smsSender, wsHub, stor, templateMgr)

	// Initialize NATS consumer
	natsConsumer, err := consumer.NewEventConsumer(cfg.NATS.URL, logger)
	if err != nil {
		logger.Fatal("Failed to create NATS consumer", zap.Error(err))
	}
	defer natsConsumer.Close()

	// Start event processing
	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Event handler
	eventHandler := func(ctx context.Context, event *types.Event) error {
		logger.Info("Processing event",
			zap.String("event_id", event.ID),
			zap.String("event_type", event.Type),
		)

		// Create notification from event
		notification := &types.Notification{
			ID:        uuid.New().String(),
			UserID:    event.UserID,
			BankID:    event.BankID,
			Type:      types.NotificationTypeWebSocket,
			Content:   fmt.Sprintf("Event: %s", event.Type),
			Metadata:  event.Data,
			Status:    types.NotificationStatusPending,
			CreatedAt: time.Now(),
			UpdatedAt: time.Now(),
		}

		return disp.Dispatch(ctx, notification)
	}

	if err := natsConsumer.Start(ctx, eventHandler); err != nil {
		logger.Fatal("Failed to start NATS consumer", zap.Error(err))
	}

	// Start WebSocket Hub
	go wsHub.Run(ctx)

	// Initialize API handlers
	handler := api.NewHandler(logger, disp, stor, wsHub)
	router := mux.NewRouter()
	handler.RegisterRoutes(router)

	// HTTP server
	httpServer := &http.Server{
		Addr:         fmt.Sprintf(":%d", cfg.Server.HTTPPort),
		Handler:      router,
		ReadTimeout:  cfg.Server.ReadTimeout,
		WriteTimeout: cfg.Server.WriteTimeout,
		IdleTimeout:  cfg.Server.IdleTimeout,
	}

	// Start HTTP server
	go func() {
		logger.Info("HTTP server listening", zap.Int("port", cfg.Server.HTTPPort))
		if err := httpServer.ListenAndServe(); err != nil && err != http.ErrServerClosed {
			logger.Fatal("HTTP server failed", zap.Error(err))
		}
	}()

	// Wait for interrupt signal
	quit := make(chan os.Signal, 1)
	signal.Notify(quit, syscall.SIGINT, syscall.SIGTERM)
	<-quit

	logger.Info("Shutting down server...")

	// Graceful shutdown
	shutdownCtx, shutdownCancel := context.WithTimeout(context.Background(), 30*time.Second)
	defer shutdownCancel()

	if err := httpServer.Shutdown(shutdownCtx); err != nil {
		logger.Fatal("Server forced to shutdown", zap.Error(err))
	}

	logger.Info("Server exited")
}
