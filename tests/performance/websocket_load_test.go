package performance

import (
	"fmt"
	"sync"
	"testing"
	"time"

	"github.com/gorilla/websocket"
)

// TestWebSocketLoad tests 1000+ concurrent WebSocket connections
func TestWebSocketLoad(t *testing.T) {
	numConnections := 1000
	wsURL := "ws://localhost:8089/ws"

	var wg sync.WaitGroup
	successCount := 0
	errorCount := 0
	var mu sync.Mutex

	start := time.Now()

	for i := 0; i < numConnections; i++ {
		wg.Add(1)

		go func(id int) {
			defer wg.Done()

			conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)
			if err != nil {
				mu.Lock()
				errorCount++
				mu.Unlock()
				return
			}
			defer conn.Close()

			mu.Lock()
			successCount++
			mu.Unlock()

			// Keep connection alive for 10 seconds
			time.Sleep(10 * time.Second)

			// Send ping
			if err := conn.WriteMessage(websocket.PingMessage, []byte{}); err == nil {
				// Read pong
				conn.SetReadDeadline(time.Now().Add(5 * time.Second))
				conn.ReadMessage()
			}
		}(i)

		// Small delay to avoid overwhelming the server
		if i%100 == 0 {
			time.Sleep(100 * time.Millisecond)
		}
	}

	wg.Wait()
	elapsed := time.Since(start)

	t.Logf("WebSocket Load Test Results:")
	t.Logf("  Total Connections Attempted: %d", numConnections)
	t.Logf("  Successful: %d", successCount)
	t.Logf("  Failed: %d", errorCount)
	t.Logf("  Success Rate: %.2f%%", float64(successCount)/float64(numConnections)*100)
	t.Logf("  Total Time: %v", elapsed)

	if successCount < numConnections*9/10 { // 90% success threshold
		t.Errorf("Too many failed connections: %d/%d", errorCount, numConnections)
	} else {
		t.Log("✓ WebSocket load test passed")
	}
}

// TestWebSocketThroughput tests message throughput
func TestWebSocketThroughput(t *testing.T) {
	wsURL := "ws://localhost:8089/ws"

	conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)
	if err != nil {
		t.Skipf("Cannot connect to WebSocket: %v", err)
		return
	}
	defer conn.Close()

	numMessages := 1000
	message := []byte(`{"type":"ping","data":"test"}`)

	start := time.Now()

	for i := 0; i < numMessages; i++ {
		if err := conn.WriteMessage(websocket.TextMessage, message); err != nil {
			t.Errorf("Failed to send message %d: %v", i, err)
			break
		}
	}

	elapsed := time.Since(start)
	messagesPerSecond := float64(numMessages) / elapsed.Seconds()

	t.Logf("✓ WebSocket Throughput: %d messages in %v (%.0f msg/sec)",
		numMessages, elapsed, messagesPerSecond)
}

// TestWebSocketReconnect tests connection resilience
func TestWebSocketReconnect(t *testing.T) {
	wsURL := "ws://localhost:8089/ws"

	for attempt := 1; attempt <= 5; attempt++ {
		conn, _, err := websocket.DefaultDialer.Dial(wsURL, nil)
		if err != nil {
			t.Logf("Attempt %d failed: %v", attempt, err)
			time.Sleep(1 * time.Second)
			continue
		}

		conn.Close()
		t.Logf("Attempt %d successful", attempt)
		time.Sleep(500 * time.Millisecond)
	}

	t.Log("✓ WebSocket reconnect test completed")
}
