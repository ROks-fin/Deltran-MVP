package bus

import (
	"context"
	"fmt"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

// Config holds NATS configuration
type Config struct {
	URL               string
	StreamPrefix      string
	Corridors         []string
	MaxReconnects     int
	ReconnectWait     time.Duration
	PingInterval      time.Duration
	MaxPingsOut       int
}

// DefaultConfig returns default configuration
func DefaultConfig() *Config {
	return &Config{
		URL:           "nats://localhost:4222",
		StreamPrefix:  "deltran",
		Corridors:     []string{"UAE_IN", "IL_UAE", "UAE_US"},
		MaxReconnects: -1, // Unlimited
		ReconnectWait: 2 * time.Second,
		PingInterval:  20 * time.Second,
		MaxPingsOut:   3,
	}
}

// Integration manages NATS connection and streams
type Integration struct {
	config   *Config
	conn     *nats.Conn
	logger   *zap.Logger
	producer *Producer
	consumer *Consumer
}

// NewIntegration creates a new NATS integration
func NewIntegration(config *Config, logger *zap.Logger) (*Integration, error) {
	logger.Info("Connecting to NATS", zap.String("url", config.URL))

	// Connection options
	opts := []nats.Option{
		nats.Name("deltran-gateway"),
		nats.MaxReconnects(config.MaxReconnects),
		nats.ReconnectWait(config.ReconnectWait),
		nats.PingInterval(config.PingInterval),
		nats.MaxPingsOutstanding(config.MaxPingsOut),
		nats.ReconnectHandler(func(nc *nats.Conn) {
			logger.Info("Reconnected to NATS", zap.String("url", nc.ConnectedUrl()))
		}),
		nats.DisconnectErrHandler(func(nc *nats.Conn, err error) {
			logger.Warn("Disconnected from NATS", zap.Error(err))
		}),
		nats.ErrorHandler(func(nc *nats.Conn, sub *nats.Subscription, err error) {
			logger.Error("NATS error", zap.Error(err))
		}),
	}

	// Connect
	nc, err := nats.Connect(config.URL, opts...)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to NATS: %w", err)
	}

	logger.Info("Connected to NATS successfully",
		zap.String("server", nc.ConnectedUrl()),
		zap.String("cluster_id", nc.ConnectedClusterName()),
	)

	// Create producer
	producer, err := NewProducer(nc, logger)
	if err != nil {
		nc.Close()
		return nil, fmt.Errorf("failed to create producer: %w", err)
	}

	// Create consumer
	consumer, err := NewConsumer(nc, logger, producer)
	if err != nil {
		nc.Close()
		return nil, fmt.Errorf("failed to create consumer: %w", err)
	}

	integration := &Integration{
		config:   config,
		conn:     nc,
		logger:   logger,
		producer: producer,
		consumer: consumer,
	}

	// Initialize streams
	if err := integration.initializeStreams(); err != nil {
		nc.Close()
		return nil, fmt.Errorf("failed to initialize streams: %w", err)
	}

	return integration, nil
}

// initializeStreams creates JetStream streams for each corridor
func (i *Integration) initializeStreams() error {
	js, err := i.conn.JetStream()
	if err != nil {
		return fmt.Errorf("failed to get JetStream context: %w", err)
	}

	for _, corridor := range i.config.Corridors {
		streamName := fmt.Sprintf("%s_corridor_%s", i.config.StreamPrefix, corridor)

		i.logger.Info("Creating stream", zap.String("stream", streamName))

		// Stream configuration
		streamConfig := &nats.StreamConfig{
			Name:        streamName,
			Description: fmt.Sprintf("DelTran corridor: %s", corridor),
			Subjects: []string{
				fmt.Sprintf("payments.%s.>", corridor),
				fmt.Sprintf("settlements.%s.>", corridor),
				fmt.Sprintf("netting.%s.>", corridor),
			},
			Retention:    nats.WorkQueuePolicy,
			MaxMsgs:      10_000_000,
			MaxBytes:     10 * 1024 * 1024 * 1024, // 10GB
			MaxAge:       7 * 24 * time.Hour,      // 7 days
			Storage:      nats.FileStorage,
			Replicas:     3,
			Duplicates:   5 * time.Minute, // 5 min deduplication window
		}

		_, err := js.AddStream(streamConfig)
		if err != nil {
			// Stream might already exist
			info, streamErr := js.StreamInfo(streamName)
			if streamErr != nil {
				return fmt.Errorf("failed to create/get stream %s: %w", streamName, err)
			}
			i.logger.Info("Stream already exists",
				zap.String("stream", streamName),
				zap.Uint64("messages", info.State.Msgs),
			)
		} else {
			i.logger.Info("Stream created successfully", zap.String("stream", streamName))
		}
	}

	// Create DLQ stream
	dlqStreamConfig := &nats.StreamConfig{
		Name:        fmt.Sprintf("%s_dlq", i.config.StreamPrefix),
		Description: "Dead Letter Queue for failed messages",
		Subjects:    []string{"dlq.>"},
		Retention:   nats.LimitsPolicy,
		MaxMsgs:     1_000_000,
		MaxBytes:    1 * 1024 * 1024 * 1024, // 1GB
		MaxAge:      30 * 24 * time.Hour,    // 30 days
		Storage:     nats.FileStorage,
		Replicas:    3,
	}

	_, err = js.AddStream(dlqStreamConfig)
	if err != nil {
		// Check if exists
		_, streamErr := js.StreamInfo(dlqStreamConfig.Name)
		if streamErr != nil {
			return fmt.Errorf("failed to create/get DLQ stream: %w", err)
		}
		i.logger.Info("DLQ stream already exists")
	} else {
		i.logger.Info("DLQ stream created successfully")
	}

	return nil
}

// Producer returns the producer
func (i *Integration) Producer() *Producer {
	return i.producer
}

// Consumer returns the consumer
func (i *Integration) Consumer() *Consumer {
	return i.consumer
}

// Close closes the integration
func (i *Integration) Close() error {
	i.logger.Info("Closing NATS integration")

	if i.conn != nil {
		i.conn.Close()
	}

	return nil
}

// Health checks NATS connection health
func (i *Integration) Health(ctx context.Context) error {
	if i.conn == nil {
		return fmt.Errorf("NATS connection is nil")
	}

	if !i.conn.IsConnected() {
		return fmt.Errorf("NATS is not connected")
	}

	// Try a simple request to verify health
	js, err := i.conn.JetStream()
	if err != nil {
		return fmt.Errorf("JetStream unavailable: %w", err)
	}

	// Check if we can access account info
	_, err = js.AccountInfo()
	if err != nil {
		return fmt.Errorf("JetStream account info failed: %w", err)
	}

	return nil
}
