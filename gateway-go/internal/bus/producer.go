package bus

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

// Producer publishes messages to NATS JetStream
type Producer struct {
	js     nats.JetStreamContext
	logger *zap.Logger
}

// NewProducer creates a new message producer
func NewProducer(nc *nats.Conn, logger *zap.Logger) (*Producer, error) {
	js, err := nc.JetStream()
	if err != nil {
		return nil, fmt.Errorf("failed to get JetStream context: %w", err)
	}

	return &Producer{
		js:     js,
		logger: logger,
	}, nil
}

// PublishPayment publishes a payment message
func (p *Producer) PublishPayment(ctx context.Context, corridorID, bankID string, payload interface{}, idempotencyKey string) error {
	subject := fmt.Sprintf("payments.%s.%s", corridorID, bankID)
	return p.publish(ctx, subject, payload, idempotencyKey, corridorID, bankID)
}

// PublishSettlement publishes a settlement message
func (p *Producer) PublishSettlement(ctx context.Context, corridorID string, payload interface{}, idempotencyKey string) error {
	subject := fmt.Sprintf("settlements.%s.all", corridorID)
	return p.publish(ctx, subject, payload, idempotencyKey, corridorID, "")
}

// PublishNetting publishes a netting proposal
func (p *Producer) PublishNetting(ctx context.Context, corridorID string, payload interface{}, idempotencyKey string) error {
	subject := fmt.Sprintf("netting.%s.all", corridorID)
	return p.publish(ctx, subject, payload, idempotencyKey, corridorID, "")
}

// publish is the internal publish method
func (p *Producer) publish(ctx context.Context, subject string, payload interface{}, idempotencyKey, corridorID, bankID string) error {
	// Serialize payload
	data, err := json.Marshal(payload)
	if err != nil {
		return fmt.Errorf("failed to marshal payload: %w", err)
	}

	// Create message with headers
	msg := &nats.Msg{
		Subject: subject,
		Data:    data,
		Header:  nats.Header{},
	}

	// Add deduplication header
	msg.Header.Set("Nats-Msg-Id", idempotencyKey)
	msg.Header.Set("Corridor-Id", corridorID)
	if bankID != "" {
		msg.Header.Set("Bank-Id", bankID)
	}
	msg.Header.Set("Timestamp", time.Now().Format(time.RFC3339))

	// Publish with context timeout
	pubCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()

	ack, err := p.js.PublishMsg(msg, nats.Context(pubCtx))
	if err != nil {
		p.logger.Error("Failed to publish message",
			zap.String("subject", subject),
			zap.String("idempotency_key", idempotencyKey),
			zap.Error(err),
		)
		return fmt.Errorf("publish failed: %w", err)
	}

	p.logger.Debug("Message published",
		zap.String("subject", subject),
		zap.String("idempotency_key", idempotencyKey),
		zap.Uint64("sequence", ack.Sequence),
		zap.String("stream", ack.Stream),
	)

	return nil
}

// PublishToDLQ publishes a failed message to Dead Letter Queue
func (p *Producer) PublishToDLQ(ctx context.Context, originalMsg interface{}, reason string, retryCount int) error {
	subject := "dlq.failed"

	dlqEntry := map[string]interface{}{
		"original_message": originalMsg,
		"failure_reason":   reason,
		"retry_count":      retryCount,
		"failed_at":        time.Now().Format(time.RFC3339),
		"reprocessable":    isReprocessable(reason),
	}

	data, err := json.Marshal(dlqEntry)
	if err != nil {
		return fmt.Errorf("failed to marshal DLQ entry: %w", err)
	}

	msg := &nats.Msg{
		Subject: subject,
		Data:    data,
	}

	pubCtx, cancel := context.WithTimeout(ctx, 5*time.Second)
	defer cancel()

	_, err = p.js.PublishMsg(msg, nats.Context(pubCtx))
	if err != nil {
		p.logger.Error("Failed to publish to DLQ",
			zap.String("reason", reason),
			zap.Int("retry_count", retryCount),
			zap.Error(err),
		)
		return fmt.Errorf("DLQ publish failed: %w", err)
	}

	p.logger.Warn("Message sent to DLQ",
		zap.String("reason", reason),
		zap.Int("retry_count", retryCount),
	)

	return nil
}

// isReprocessable checks if a failure is transient
func isReprocessable(reason string) bool {
	transientErrors := []string{
		"timeout",
		"connection",
		"unavailable",
		"rate_limit",
		"temporary",
	}

	for _, err := range transientErrors {
		if contains(reason, err) {
			return true
		}
	}
	return false
}

func contains(s, substr string) bool {
	return len(s) >= len(substr) && (s == substr || len(s) > len(substr) && (s[:len(substr)] == substr || s[len(s)-len(substr):] == substr))
}
