package consumer

import (
	"context"
	"encoding/json"
	"fmt"
	"time"

	"github.com/deltran/notification-engine/pkg/types"
	"github.com/nats-io/nats.go"
	"go.uber.org/zap"
)

type EventHandler func(ctx context.Context, event *types.Event) error

type EventConsumer struct {
	conn      *nats.Conn
	jetStream nats.JetStreamContext
	logger    *zap.Logger
}

func NewEventConsumer(natsURL string, logger *zap.Logger) (*EventConsumer, error) {
	opts := []nats.Option{
		nats.Name("notification-engine"),
		nats.MaxReconnects(10),
		nats.ReconnectWait(2 * time.Second),
	}

	conn, err := nats.Connect(natsURL, opts...)
	if err != nil {
		return nil, fmt.Errorf("failed to connect to NATS: %w", err)
	}

	js, err := conn.JetStream()
	if err != nil {
		conn.Close()
		return nil, fmt.Errorf("failed to create JetStream context: %w", err)
	}

	logger.Info("Connected to NATS JetStream", zap.String("url", natsURL))

	return &EventConsumer{
		conn:      conn,
		jetStream: js,
		logger:    logger,
	}, nil
}

func (ec *EventConsumer) Start(ctx context.Context, handler EventHandler) error {
	sub, err := ec.jetStream.PullSubscribe(
		"events.*",
		"notification-engine",
		nats.ManualAck(),
	)
	if err != nil {
		return fmt.Errorf("failed to subscribe: %w", err)
	}

	go ec.consumeMessages(ctx, sub, handler)
	return nil
}

func (ec *EventConsumer) consumeMessages(ctx context.Context, sub *nats.Subscription, handler EventHandler) {
	for {
		select {
		case <-ctx.Done():
			return
		default:
			msgs, err := sub.Fetch(10, nats.MaxWait(1*time.Second))
			if err != nil && err != nats.ErrTimeout {
				ec.logger.Error("Failed to fetch messages", zap.Error(err))
				continue
			}

			for _, msg := range msgs {
				go ec.processMessage(ctx, msg, handler)
			}
		}
	}
}

func (ec *EventConsumer) processMessage(ctx context.Context, msg *nats.Msg, handler EventHandler) {
	var event types.Event
	if err := json.Unmarshal(msg.Data, &event); err != nil {
		ec.logger.Error("Failed to unmarshal event", zap.Error(err))
		msg.Term()
		return
	}

	event.Type = msg.Subject

	if err := handler(ctx, &event); err != nil {
		ec.logger.Error("Failed to process event", zap.Error(err), zap.String("event_id", event.ID))
		msg.NakWithDelay(5 * time.Second)
		return
	}

	msg.Ack()
}

func (ec *EventConsumer) Close() {
	if ec.conn != nil {
		ec.conn.Close()
	}
}
