// Ledger client - gRPC client to Rust ledger core
package ledger

import (
	"context"
	"fmt"
	"time"

	"github.com/deltran/gateway/internal/types"
	"github.com/google/uuid"
	"github.com/shopspring/decimal"
	"go.uber.org/zap"
	"google.golang.org/grpc"
	"google.golang.org/grpc/credentials/insecure"
)

// Client represents a ledger client
type Client struct {
	conn    *grpc.ClientConn
	logger  *zap.Logger
	timeout time.Duration
}

// NewClient creates a new ledger client
func NewClient(addr string, timeout time.Duration, logger *zap.Logger) (*Client, error) {
	// Create gRPC connection (non-blocking, will connect on first use)
	conn, err := grpc.Dial(addr,
		grpc.WithTransportCredentials(insecure.NewCredentials()),
	)
	if err != nil {
		return nil, fmt.Errorf("failed to create ledger client: %w", err)
	}

	logger.Info("Ledger client created (connection lazy)", zap.String("addr", addr))

	return &Client{
		conn:    conn,
		logger:  logger,
		timeout: timeout,
	}, nil
}

// AppendEvent appends an event to the ledger
func (c *Client) AppendEvent(ctx context.Context, payment *types.Payment, eventType types.EventType) (uuid.UUID, error) {
	// TODO: Implement gRPC call to ledger AppendEvent
	// For now, return a dummy UUID

	c.logger.Debug("Appending event to ledger",
		zap.String("payment_id", payment.PaymentID.String()),
		zap.String("event_type", string(eventType)),
	)

	// Simulate ledger call
	eventID := uuid.New()

	return eventID, nil
}

// GetPaymentState retrieves payment state from ledger
func (c *Client) GetPaymentState(ctx context.Context, paymentID uuid.UUID) (*types.Payment, error) {
	// TODO: Implement gRPC call to ledger GetPaymentState

	c.logger.Debug("Getting payment state from ledger",
		zap.String("payment_id", paymentID.String()),
	)

	// Simulate ledger call - return dummy payment
	payment := &types.Payment{
		PaymentID:       paymentID,
		Amount:          decimal.NewFromFloat(100.0),
		Currency:        "USD",
		Status:          types.PaymentStatusInitiated,
		CreatedAt:       time.Now(),
		UpdatedAt:       time.Now(),
	}

	return payment, nil
}

// GetPaymentEvents retrieves all events for a payment
func (c *Client) GetPaymentEvents(ctx context.Context, paymentID uuid.UUID) ([]types.EventType, error) {
	// TODO: Implement gRPC call to ledger GetPaymentEvents

	c.logger.Debug("Getting payment events from ledger",
		zap.String("payment_id", paymentID.String()),
	)

	// Simulate ledger call
	events := []types.EventType{
		types.EventTypePaymentInitiated,
		types.EventTypeValidationPassed,
	}

	return events, nil
}

// FinalizeBlock finalizes a block of events
func (c *Client) FinalizeBlock(ctx context.Context, eventIDs []uuid.UUID) (uuid.UUID, error) {
	// TODO: Implement gRPC call to ledger FinalizeBlock

	c.logger.Debug("Finalizing block in ledger",
		zap.Int("event_count", len(eventIDs)),
	)

	// Simulate ledger call
	blockID := uuid.New()

	return blockID, nil
}

// Close closes the ledger client
func (c *Client) Close() error {
	if c.conn != nil {
		return c.conn.Close()
	}
	return nil
}

// Batch represents a batch of events for efficient submission
type Batch struct {
	events    []*Event
	maxSize   int
	timeout   time.Duration
	submitFn  func([]*Event) error
	timer     *time.Timer
	submitCh  chan struct{}
}

// Event represents a ledger event
type Event struct {
	PaymentID uuid.UUID
	EventType types.EventType
	Timestamp time.Time
}

// NewBatch creates a new batch
func NewBatch(maxSize int, timeout time.Duration, submitFn func([]*Event) error) *Batch {
	b := &Batch{
		events:   make([]*Event, 0, maxSize),
		maxSize:  maxSize,
		timeout:  timeout,
		submitFn: submitFn,
		submitCh: make(chan struct{}, 1),
	}
	b.timer = time.AfterFunc(timeout, func() {
		_ = b.flush() // Ignore error in timer callback
	})
	return b
}

// Add adds an event to the batch
func (b *Batch) Add(event *Event) error {
	b.events = append(b.events, event)

	// Check if batch is full
	if len(b.events) >= b.maxSize {
		return b.flush()
	}

	// Reset timer
	b.timer.Reset(b.timeout)

	return nil
}

// flush submits the batch
func (b *Batch) flush() error {
	if len(b.events) == 0 {
		return nil
	}

	events := b.events
	b.events = make([]*Event, 0, b.maxSize)

	return b.submitFn(events)
}

// Close closes the batch
func (b *Batch) Close() error {
	b.timer.Stop()
	return b.flush()
}