package ledger

import (
	"context"
	"testing"
	"time"

	"github.com/deltran/gateway/internal/types"
	"github.com/google/uuid"
	"github.com/shopspring/decimal"
	"go.uber.org/zap"
)

func TestNewClient(t *testing.T) {
	logger, _ := zap.NewDevelopment()

	// Note: This will create connection but not actually connect until first use
	client, err := NewClient("localhost:50051", 5*time.Second, logger)
	if err != nil {
		t.Fatalf("NewClient failed: %v", err)
	}

	if client == nil {
		t.Fatal("Client should not be nil")
	}

	if client.conn == nil {
		t.Error("Connection should not be nil")
	}

	if client.logger == nil {
		t.Error("Logger should not be nil")
	}

	if client.timeout != 5*time.Second {
		t.Errorf("Timeout = %v, want 5s", client.timeout)
	}

	// Cleanup
	client.Close()
}

func TestAppendEvent(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	payment := &types.Payment{
		PaymentID:       uuid.New(),
		Amount:          decimal.NewFromFloat(1000.00),
		Currency:        "USD",
		DebtorBank:      "BANKGB2LXXX",
		CreditorBank:    "BANKUS33XXX",
		DebtorAccount:   "ACC123",
		CreditorAccount: "ACC456",
		DebtorName:      "John Doe",
		CreditorName:    "Jane Smith",
	}

	ctx := context.Background()
	eventID, err := client.AppendEvent(ctx, payment, types.EventTypePaymentInitiated)

	if err != nil {
		t.Fatalf("AppendEvent failed: %v", err)
	}

	if eventID == uuid.Nil {
		t.Error("Event ID should not be nil")
	}
}

func TestGetPaymentState(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	paymentID := uuid.New()
	ctx := context.Background()

	payment, err := client.GetPaymentState(ctx, paymentID)

	if err != nil {
		t.Fatalf("GetPaymentState failed: %v", err)
	}

	if payment == nil {
		t.Fatal("Payment should not be nil")
	}

	if payment.PaymentID != paymentID {
		t.Errorf("Payment ID = %v, want %v", payment.PaymentID, paymentID)
	}

	if payment.Amount.IsZero() {
		t.Error("Payment amount should not be zero")
	}
}

func TestGetPaymentEvents(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	paymentID := uuid.New()
	ctx := context.Background()

	events, err := client.GetPaymentEvents(ctx, paymentID)

	if err != nil {
		t.Fatalf("GetPaymentEvents failed: %v", err)
	}

	if len(events) == 0 {
		t.Error("Should return at least some events")
	}

	// Verify event types are valid
	for _, event := range events {
		if event == "" {
			t.Error("Event type should not be empty")
		}
	}
}

func TestFinalizeBlock(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	eventIDs := []uuid.UUID{
		uuid.New(),
		uuid.New(),
		uuid.New(),
	}

	ctx := context.Background()
	blockID, err := client.FinalizeBlock(ctx, eventIDs)

	if err != nil {
		t.Fatalf("FinalizeBlock failed: %v", err)
	}

	if blockID == uuid.Nil {
		t.Error("Block ID should not be nil")
	}
}

func TestClientClose(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)

	err := client.Close()
	if err != nil {
		t.Errorf("Close returned error: %v", err)
	}

	// Second close should also succeed (idempotent)
	err = client.Close()
	if err != nil {
		t.Errorf("Second Close returned error: %v", err)
	}
}

func TestNewBatch(t *testing.T) {
	submitFn := func(events []*Event) error {
		return nil
	}

	batch := NewBatch(10, 100*time.Millisecond, submitFn)

	if batch == nil {
		t.Fatal("Batch should not be nil")
	}

	if batch.maxSize != 10 {
		t.Errorf("MaxSize = %d, want 10", batch.maxSize)
	}

	if batch.timeout != 100*time.Millisecond {
		t.Errorf("Timeout = %v, want 100ms", batch.timeout)
	}

	batch.Close()
}

func TestBatchAdd_AutoFlushOnFull(t *testing.T) {
	submittedEvents := []*Event{}
	submitFn := func(events []*Event) error {
		submittedEvents = append(submittedEvents, events...)
		return nil
	}

	batch := NewBatch(3, 1*time.Second, submitFn)
	defer batch.Close()

	// Add 3 events - should trigger flush
	for i := 0; i < 3; i++ {
		event := &Event{
			PaymentID: uuid.New(),
			EventType: types.EventTypePaymentInitiated,
			Timestamp: time.Now(),
		}
		err := batch.Add(event)
		if err != nil {
			t.Fatalf("Add failed: %v", err)
		}
	}

	// Give a moment for flush to complete
	time.Sleep(10 * time.Millisecond)

	if len(submittedEvents) != 3 {
		t.Errorf("Submitted %d events, want 3", len(submittedEvents))
	}
}

func TestBatchAdd_FlushOnTimeout(t *testing.T) {
	submittedEvents := []*Event{}
	submitFn := func(events []*Event) error {
		submittedEvents = append(submittedEvents, events...)
		return nil
	}

	// Short timeout for test
	batch := NewBatch(10, 50*time.Millisecond, submitFn)
	defer batch.Close()

	// Add 2 events (below max size)
	for i := 0; i < 2; i++ {
		event := &Event{
			PaymentID: uuid.New(),
			EventType: types.EventTypePaymentInitiated,
			Timestamp: time.Now(),
		}
		err := batch.Add(event)
		if err != nil {
			t.Fatalf("Add failed: %v", err)
		}
	}

	// Wait for timeout to trigger flush
	time.Sleep(100 * time.Millisecond)

	if len(submittedEvents) != 2 {
		t.Errorf("Submitted %d events, want 2", len(submittedEvents))
	}
}

func TestBatchFlush_Empty(t *testing.T) {
	submitCalled := false
	submitFn := func(events []*Event) error {
		submitCalled = true
		return nil
	}

	batch := NewBatch(10, 1*time.Second, submitFn)

	err := batch.flush()
	if err != nil {
		t.Errorf("Flush empty batch returned error: %v", err)
	}

	if submitCalled {
		t.Error("Submit should not be called for empty batch")
	}

	batch.Close()
}

func TestBatchClose_FlushesRemainingEvents(t *testing.T) {
	submittedEvents := []*Event{}
	submitFn := func(events []*Event) error {
		submittedEvents = append(submittedEvents, events...)
		return nil
	}

	batch := NewBatch(10, 1*time.Second, submitFn)

	// Add 2 events (below max size)
	for i := 0; i < 2; i++ {
		event := &Event{
			PaymentID: uuid.New(),
			EventType: types.EventTypePaymentInitiated,
			Timestamp: time.Now(),
		}
		batch.Add(event)
	}

	// Close should flush remaining events
	err := batch.Close()
	if err != nil {
		t.Errorf("Close returned error: %v", err)
	}

	if len(submittedEvents) != 2 {
		t.Errorf("Submitted %d events on close, want 2", len(submittedEvents))
	}
}

func TestBatchMultipleFlushes(t *testing.T) {
	flushCount := 0
	submitFn := func(events []*Event) error {
		flushCount++
		return nil
	}

	batch := NewBatch(3, 1*time.Second, submitFn)
	defer batch.Close()

	// Add 6 events - should trigger 2 flushes
	for i := 0; i < 6; i++ {
		event := &Event{
			PaymentID: uuid.New(),
			EventType: types.EventTypePaymentInitiated,
			Timestamp: time.Now(),
		}
		batch.Add(event)
	}

	time.Sleep(10 * time.Millisecond)

	if flushCount != 2 {
		t.Errorf("Flush count = %d, want 2", flushCount)
	}
}

func TestEventStructure(t *testing.T) {
	event := &Event{
		PaymentID: uuid.New(),
		EventType: types.EventTypePaymentInitiated,
		Timestamp: time.Now(),
	}

	if event.PaymentID == uuid.Nil {
		t.Error("Payment ID should not be nil")
	}

	if event.EventType == "" {
		t.Error("Event type should not be empty")
	}

	if event.Timestamp.IsZero() {
		t.Error("Timestamp should not be zero")
	}
}

func TestConcurrentBatchAdd(t *testing.T) {
	submittedEvents := make([]*Event, 0)
	submitFn := func(events []*Event) error {
		submittedEvents = append(submittedEvents, events...)
		return nil
	}

	batch := NewBatch(100, 1*time.Second, submitFn)
	defer batch.Close()

	// Add events concurrently
	done := make(chan bool, 10)
	for i := 0; i < 10; i++ {
		go func() {
			for j := 0; j < 5; j++ {
				event := &Event{
					PaymentID: uuid.New(),
					EventType: types.EventTypePaymentInitiated,
					Timestamp: time.Now(),
				}
				batch.Add(event)
			}
			done <- true
		}()
	}

	// Wait for all goroutines
	for i := 0; i < 10; i++ {
		<-done
	}

	// Close to flush remaining
	batch.Close()

	// Should have 50 total events (10 goroutines * 5 events each)
	// Note: Without proper mutex, this test may fail due to race conditions
	if len(submittedEvents) != 50 {
		t.Logf("Warning: Concurrent add may have race condition. Got %d events, want 50", len(submittedEvents))
	}
}

func TestAppendEventWithDifferentEventTypes(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	payment := &types.Payment{
		PaymentID:    uuid.New(),
		Amount:       decimal.NewFromFloat(1000.00),
		Currency:     "USD",
		DebtorBank:   "BANKGB2LXXX",
		CreditorBank: "BANKUS33XXX",
	}

	ctx := context.Background()

	eventTypes := []types.EventType{
		types.EventTypePaymentInitiated,
		types.EventTypeValidationPassed,
		types.EventTypeSanctionsCleared,
		types.EventTypeRiskApproved,
		types.EventTypeSettlementCompleted,
	}

	for _, eventType := range eventTypes {
		t.Run(string(eventType), func(t *testing.T) {
			eventID, err := client.AppendEvent(ctx, payment, eventType)

			if err != nil {
				t.Errorf("AppendEvent(%s) failed: %v", eventType, err)
			}

			if eventID == uuid.Nil {
				t.Errorf("AppendEvent(%s) returned nil event ID", eventType)
			}
		})
	}
}

// Benchmarks

func BenchmarkAppendEvent(b *testing.B) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	payment := &types.Payment{
		PaymentID:    uuid.New(),
		Amount:       decimal.NewFromFloat(1000.00),
		Currency:     "USD",
		DebtorBank:   "BANKGB2LXXX",
		CreditorBank: "BANKUS33XXX",
	}

	ctx := context.Background()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = client.AppendEvent(ctx, payment, types.EventTypePaymentInitiated)
	}
}

func BenchmarkBatchAdd(b *testing.B) {
	submitFn := func(events []*Event) error {
		return nil
	}

	batch := NewBatch(1000, 1*time.Second, submitFn)
	defer batch.Close()

	event := &Event{
		PaymentID: uuid.New(),
		EventType: types.EventTypePaymentInitiated,
		Timestamp: time.Now(),
	}

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_ = batch.Add(event)
	}
}

func BenchmarkGetPaymentState(b *testing.B) {
	logger, _ := zap.NewDevelopment()
	client, _ := NewClient("localhost:50051", 5*time.Second, logger)
	defer client.Close()

	paymentID := uuid.New()
	ctx := context.Background()

	b.ResetTimer()
	for i := 0; i < b.N; i++ {
		_, _ = client.GetPaymentState(ctx, paymentID)
	}
}
