package tests

import (
	"fmt"
	"net/url"
	"sync"
	"testing"
	"time"

	"github.com/gorilla/websocket"
)

func TestWebSocketLoad(t *testing.T) {
	if testing.Short() {
		t.Skip("Skipping load test in short mode")
	}

	numClients := 1000
	serverURL := "ws://localhost:8086/ws"

	var wg sync.WaitGroup
	successCount := 0
	var mu sync.Mutex

	for i := 0; i < numClients; i++ {
		wg.Add(1)
		go func(id int) {
			defer wg.Done()

			u, _ := url.Parse(serverURL)
			q := u.Query()
			q.Set("user_id", fmt.Sprintf("user-%d", id))
			u.RawQuery = q.Encode()

			conn, _, err := websocket.DefaultDialer.Dial(u.String(), nil)
			if err != nil {
				t.Logf("Failed to connect client %d: %v", id, err)
				return
			}
			defer conn.Close()

			mu.Lock()
			successCount++
			mu.Unlock()

			// Keep connection alive for 5 seconds
			time.Sleep(5 * time.Second)
		}(i)

		// Stagger connection attempts
		time.Sleep(10 * time.Millisecond)
	}

	wg.Wait()

	t.Logf("Successfully connected %d/%d clients", successCount, numClients)

	if successCount < numClients*90/100 {
		t.Errorf("Only %d/%d clients connected (expected >90%%)", successCount, numClients)
	}
}
