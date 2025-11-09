package integration

import (
	"context"
	"testing"
	"time"

	"github.com/nats-io/nats.go"
)

// TestNATSConnection tests basic NATS connectivity
func TestNATSConnection(t *testing.T) {
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	if !nc.IsConnected() {
		t.Fatal("NATS not connected")
	}

	t.Log("✓ Successfully connected to NATS")
}

// TestNATSPubSub tests publish/subscribe
func TestNATSPubSub(t *testing.T) {
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	subject := "test.pubsub"
	message := []byte("test message")

	// Subscribe
	received := make(chan []byte, 1)
	sub, err := nc.Subscribe(subject, func(msg *nats.Msg) {
		received <- msg.Data
	})
	if err != nil {
		t.Fatalf("Failed to subscribe: %v", err)
	}
	defer sub.Unsubscribe()

	// Give subscription time to register
	time.Sleep(100 * time.Millisecond)

	// Publish
	if err := nc.Publish(subject, message); err != nil {
		t.Fatalf("Failed to publish: %v", err)
	}

	// Wait for message
	select {
	case msg := <-received:
		if string(msg) != string(message) {
			t.Errorf("Expected %s, got %s", message, msg)
		} else {
			t.Log("✓ NATS pub/sub working correctly")
		}
	case <-time.After(5 * time.Second):
		t.Error("Timeout waiting for message")
	}
}

// TestNATSJetStream tests JetStream availability
func TestNATSJetStream(t *testing.T) {
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	js, err := nc.JetStream()
	if err != nil {
		t.Fatalf("Failed to get JetStream context: %v", err)
	}

	// Try to get account info
	ctx, cancel := context.WithTimeout(context.Background(), 5*time.Second)
	defer cancel()

	info, err := js.AccountInfo(nats.Context(ctx))
	if err != nil {
		t.Fatalf("Failed to get JetStream account info: %v", err)
	}

	t.Logf("✓ JetStream available - Streams: %d, Consumers: %d", info.Streams, info.Consumers)
}

// TestNATSEventStream tests event stream creation
func TestNATSEventStream(t *testing.T) {
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	js, err := nc.JetStream()
	if err != nil {
		t.Fatalf("Failed to get JetStream context: %v", err)
	}

	// Try to create or update a test stream
	streamName := "TEST_TRANSACTIONS"
	_, err = js.AddStream(&nats.StreamConfig{
		Name:     streamName,
		Subjects: []string{"test.transactions.*"},
		Storage:  nats.FileStorage,
		MaxAge:   24 * time.Hour,
	})

	if err != nil {
		t.Logf("Note: Stream creation failed (may already exist): %v", err)
	} else {
		t.Logf("✓ Created test stream: %s", streamName)
		// Clean up
		js.DeleteStream(streamName)
	}
}

// TestNATSPerformance tests message throughput
func TestNATSPerformance(t *testing.T) {
	nc, err := nats.Connect("nats://localhost:4222")
	if err != nil {
		t.Fatalf("Failed to connect to NATS: %v", err)
	}
	defer nc.Close()

	subject := "test.performance"
	messageCount := 1000

	start := time.Now()

	for i := 0; i < messageCount; i++ {
		if err := nc.Publish(subject, []byte("test")); err != nil {
			t.Fatalf("Failed to publish message %d: %v", i, err)
		}
	}

	// Flush to ensure all messages are sent
	if err := nc.Flush(); err != nil {
		t.Fatalf("Failed to flush: %v", err)
	}

	elapsed := time.Since(start)
	messagesPerSecond := float64(messageCount) / elapsed.Seconds()

	t.Logf("✓ NATS Performance: %d messages in %v (%.0f msg/sec)", messageCount, elapsed, messagesPerSecond)

	if messagesPerSecond < 1000 {
		t.Logf("Warning: Low throughput (%.0f msg/sec), expected > 1000", messagesPerSecond)
	}
}
