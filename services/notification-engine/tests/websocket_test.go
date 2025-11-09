package tests

import (
	"context"
	"testing"
	"time"

	"github.com/deltran/notification-engine/internal/websocket"
	"github.com/deltran/notification-engine/pkg/types"
	"github.com/redis/go-redis/v9"
	"go.uber.org/zap"
)

func TestWebSocketHub(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	
	// Create mock Redis client
	redisClient := redis.NewClient(&redis.Options{
		Addr: "localhost:6379",
	})

	hub := websocket.NewHub(redisClient, "test-server", logger)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go hub.Run(ctx)

	// Test broadcast
	message := &types.WebSocketMessage{
		Type:      "test",
		Payload:   map[string]interface{}{"foo": "bar"},
		Timestamp: time.Now(),
	}

	hub.Broadcast(message)

	time.Sleep(100 * time.Millisecond)

	if hub.GetClientCount() != 0 {
		t.Errorf("Expected 0 clients, got %d", hub.GetClientCount())
	}
}

func TestWebSocketHubConcurrency(t *testing.T) {
	logger, _ := zap.NewDevelopment()
	
	redisClient := redis.NewClient(&redis.Options{
		Addr: "localhost:6379",
	})

	hub := websocket.NewHub(redisClient, "test-server", logger)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	go hub.Run(ctx)

	// Send 1000 messages concurrently
	for i := 0; i < 1000; i++ {
		go func(i int) {
			message := &types.WebSocketMessage{
				Type:      "test",
				UserID:    "user-1",
				Payload:   map[string]interface{}{"index": i},
				Timestamp: time.Now(),
			}
			hub.Broadcast(message)
		}(i)
	}

	time.Sleep(1 * time.Second)
}
