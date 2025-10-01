package bus

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

// MessageHandler processes incoming messages
type MessageHandler func(ctx context.Context, msg *Message) error

// Message represents a business message
type Message struct {
	ID             string                 `json:"id"`
	Type           string                 `json:"type"`
	CorridorID     string                 `json:"corridor_id"`
	BankID         string                 `json:"bank_id"`
	Payload        json.RawMessage        `json:"payload"`
	IdempotencyKey string                 `json:"idempotency_key"`
	Timestamp      time.Time              `json:"timestamp"`
	Headers        map[string]string      `json:"headers"`
}

// Consumer consumes messages from NATS JetStream
type Consumer struct {
	js          nats.JetStreamContext
	logger      *zap.Logger
	producer    *Producer
	maxRetries  int
}

// NewConsumer creates a new message consumer
func NewConsumer(nc *nats.Conn, logger *zap.Logger, producer *Producer) (*Consumer, error) {
	js, err := nc.JetStream()
	if err != nil {
		return nil, fmt.Errorf("failed to get JetStream context: %w", err)
	}

	return &Consumer{
		js:         js,
		logger:     logger,
		producer:   producer,
		maxRetries: 5,
	}, nil
}

// Subscribe creates a durable consumer and processes messages
func (c *Consumer) Subscribe(
	ctx context.Context,
	streamName string,
	consumerName string,
	filterSubject string,
	handler MessageHandler,
) error {
	c.logger.Info("Creating consumer",
		zap.String("stream", streamName),
		zap.String("consumer", consumerName),
		zap.String("filter", filterSubject),
	)

	// Create or get durable consumer
	consumerConfig := &nats.ConsumerConfig{
		Durable:       consumerName,
		FilterSubject: filterSubject,
		AckPolicy:     nats.AckExplicitPolicy,
		AckWait:       30 * time.Second,
		MaxDeliver:    c.maxRetries,
		DeliverPolicy: nats.DeliverAllPolicy,
	}

	// Subscribe with pull-based consumption
	sub, err := c.js.PullSubscribe(
		filterSubject,
		consumerName,
		nats.Bind(streamName, consumerName),
	)
	if err != nil {
		// Try to create consumer if it doesn't exist
		_, err = c.js.AddConsumer(streamName, consumerConfig)
		if err != nil {
			return fmt.Errorf("failed to create consumer: %w", err)
		}

		sub, err = c.js.PullSubscribe(
			filterSubject,
			consumerName,
			nats.Bind(streamName, consumerName),
		)
		if err != nil {
			return fmt.Errorf("failed to subscribe: %w", err)
		}
	}

	c.logger.Info("Consumer subscribed successfully",
		zap.String("consumer", consumerName),
	)

	// Start consuming
	go c.consumeLoop(ctx, sub, handler)

	return nil
}

// consumeLoop continuously fetches and processes messages
func (c *Consumer) consumeLoop(ctx context.Context, sub *nats.Subscription, handler MessageHandler) {
	batchSize := 10
	timeout := 5 * time.Second

	for {
		select {
		case <-ctx.Done():
			c.logger.Info("Consumer stopping")
			sub.Drain()
			return
		default:
			// Fetch batch of messages
			msgs, err := sub.Fetch(batchSize, nats.MaxWait(timeout))
			if err != nil {
				if err == nats.ErrTimeout {
					continue
				}
				c.logger.Error("Fetch error", zap.Error(err))
				time.Sleep(1 * time.Second)
				continue
			}

			// Process each message
			for _, natsMsg := range msgs {
				c.processMessage(ctx, natsMsg, handler)
			}
		}
	}
}

// processMessage processes a single message
func (c *Consumer) processMessage(ctx context.Context, natsMsg *nats.Msg, handler MessageHandler) {
	metadata, err := natsMsg.Metadata()
	if err != nil {
		c.logger.Error("Failed to get message metadata", zap.Error(err))
		natsMsg.Nak()
		return
	}

	// Parse message
	msg := &Message{
		ID:             natsMsg.Header.Get("Nats-Msg-Id"),
		CorridorID:     natsMsg.Header.Get("Corridor-Id"),
		BankID:         natsMsg.Header.Get("Bank-Id"),
		IdempotencyKey: natsMsg.Header.Get("Nats-Msg-Id"),
		Payload:        natsMsg.Data,
		Headers:        make(map[string]string),
	}

	// Copy headers
	for key := range natsMsg.Header {
		msg.Headers[key] = natsMsg.Header.Get(key)
	}

	c.logger.Debug("Processing message",
		zap.String("msg_id", msg.ID),
		zap.String("subject", natsMsg.Subject),
		zap.Uint64("sequence", metadata.Sequence.Stream),
		zap.Uint64("delivery_count", metadata.NumDelivered),
	)

	// Process with handler
	handlerCtx, cancel := context.WithTimeout(ctx, 25*time.Second)
	defer cancel()

	err = handler(handlerCtx, msg)
	if err != nil {
		c.logger.Error("Handler error",
			zap.String("msg_id", msg.ID),
			zap.Error(err),
			zap.Uint64("delivery_count", metadata.NumDelivered),
		)

		// Check if max retries reached
		if metadata.NumDelivered >= uint64(c.maxRetries) {
			c.logger.Warn("Max retries reached, sending to DLQ",
				zap.String("msg_id", msg.ID),
			)

			// Send to DLQ
			if dlqErr := c.producer.PublishToDLQ(ctx, msg, err.Error(), int(metadata.NumDelivered)); dlqErr != nil {
				c.logger.Error("Failed to send to DLQ", zap.Error(dlqErr))
			}

			// Ack to remove from stream
			natsMsg.Ack()
		} else {
			// Negative ack for retry
			natsMsg.Nak()
		}
		return
	}

	// Success - acknowledge
	if err := natsMsg.Ack(); err != nil {
		c.logger.Error("Failed to ack message", zap.Error(err))
	}

	c.logger.Debug("Message processed successfully",
		zap.String("msg_id", msg.ID),
	)
}

// Close closes the consumer
func (c *Consumer) Close() error {
	c.logger.Info("Closing consumer")
	return nil
}
