package websocket

import (
	"context"
	"encoding/json"
	"sync"
	"time"

	"github.com/deltran/notification-engine/pkg/types"
	"github.com/redis/go-redis/v9"
	"go.uber.org/zap"
)

// Hub maintains the set of active clients and broadcasts messages to clients
type Hub struct {
	// Registered clients
	clients map[string]*Client

	// Inbound messages from clients
	broadcast chan *types.WebSocketMessage

	// Register requests from clients
	register chan *Client

	// Unregister requests from clients
	unregister chan *Client

	// Mutex for thread-safe operations
	mutex sync.RWMutex

	// Redis client for horizontal scaling
	redis *redis.Client

	// Server ID for this instance
	serverID string

	// Logger
	logger *zap.Logger

	// Metrics
	metrics *Metrics
}

// NewHub creates a new Hub instance
func NewHub(redisClient *redis.Client, serverID string, logger *zap.Logger) *Hub {
	return &Hub{
		clients:    make(map[string]*Client),
		broadcast:  make(chan *types.WebSocketMessage, 1000),
		register:   make(chan *Client, 100),
		unregister: make(chan *Client, 100),
		redis:      redisClient,
		serverID:   serverID,
		logger:     logger,
		metrics:    NewMetrics(),
	}
}

// Run starts the hub's main loop
func (h *Hub) Run(ctx context.Context) {
	ticker := time.NewTicker(30 * time.Second)
	defer ticker.Stop()

	// Subscribe to Redis pub/sub for cross-server broadcasts
	pubsub := h.redis.Subscribe(ctx, "ws:broadcast")
	defer pubsub.Close()

	h.logger.Info("WebSocket Hub started", zap.String("server_id", h.serverID))

	for {
		select {
		case <-ctx.Done():
			h.logger.Info("WebSocket Hub shutting down")
			h.closeAllClients()
			return

		case client := <-h.register:
			h.registerClient(ctx, client)

		case client := <-h.unregister:
			h.unregisterClient(ctx, client)

		case message := <-h.broadcast:
			h.broadcastMessage(message)

		case msg := <-pubsub.Channel():
			// Handle messages from Redis pub/sub (from other servers)
			h.handleRedisBroadcast(msg.Payload)

		case <-ticker.C:
			h.pingClients()
		}
	}
}

// registerClient registers a new client
func (h *Hub) registerClient(ctx context.Context, client *Client) {
	h.mutex.Lock()
	h.clients[client.ID] = client
	h.mutex.Unlock()

	// Store connection in Redis for horizontal scaling
	connData := map[string]interface{}{
		"server_id":   h.serverID,
		"user_id":     client.UserID,
		"bank_id":     client.BankID,
		"connected_at": time.Now().Unix(),
	}

	data, _ := json.Marshal(connData)
	h.redis.Set(ctx, "ws:conn:"+client.ID, data, 5*time.Minute)

	h.metrics.Inc()

	h.logger.Info("Client registered",
		zap.String("client_id", client.ID),
		zap.String("user_id", client.UserID),
		zap.Int("total_clients", len(h.clients)),
	)
}

// unregisterClient unregisters a client
func (h *Hub) unregisterClient(ctx context.Context, client *Client) {
	h.mutex.Lock()
	if _, ok := h.clients[client.ID]; ok {
		delete(h.clients, client.ID)
		close(client.send)
	}
	h.mutex.Unlock()

	// Remove from Redis
	h.redis.Del(ctx, "ws:conn:"+client.ID)

	h.metrics.Dec()

	h.logger.Info("Client unregistered",
		zap.String("client_id", client.ID),
		zap.String("user_id", client.UserID),
		zap.Int("total_clients", len(h.clients)),
	)
}

// broadcastMessage broadcasts a message to relevant clients
func (h *Hub) broadcastMessage(message *types.WebSocketMessage) {
	h.mutex.RLock()
	defer h.mutex.RUnlock()

	sentCount := 0

	for _, client := range h.clients {
		if h.shouldReceive(client, message) {
			select {
			case client.send <- message:
				sentCount++
			default:
				// Client buffer full, log and skip
				h.logger.Warn("Client buffer full, skipping message",
					zap.String("client_id", client.ID),
				)
			}
		}
	}

	h.metrics.AddMessages(int64(sentCount))

	h.logger.Debug("Message broadcast",
		zap.String("message_type", message.Type),
		zap.Int("recipients", sentCount),
	)
}

// shouldReceive determines if a client should receive a message
func (h *Hub) shouldReceive(client *Client, message *types.WebSocketMessage) bool {
	// Broadcast to all
	if message.UserID == "" && message.BankID == "" {
		return true
	}

	// User-specific message
	if message.UserID != "" && client.UserID == message.UserID {
		return true
	}

	// Bank-specific message
	if message.BankID != "" && client.BankID == message.BankID {
		return true
	}

	return false
}

// pingClients sends ping to all connected clients
func (h *Hub) pingClients() {
	h.mutex.RLock()
	defer h.mutex.RUnlock()

	for _, client := range h.clients {
		select {
		case client.ping <- true:
		default:
			h.logger.Warn("Failed to ping client", zap.String("client_id", client.ID))
		}
	}
}

// closeAllClients closes all client connections
func (h *Hub) closeAllClients() {
	h.mutex.Lock()
	defer h.mutex.Unlock()

	for _, client := range h.clients {
		close(client.send)
	}

	h.clients = make(map[string]*Client)
}

// Broadcast sends a message to the broadcast channel
func (h *Hub) Broadcast(message *types.WebSocketMessage) {
	// Also publish to Redis for other server instances
	data, _ := json.Marshal(message)
	h.redis.Publish(context.Background(), "ws:broadcast", data)

	// Send to local clients
	h.broadcast <- message
}

// handleRedisBroadcast handles broadcasts from Redis pub/sub
func (h *Hub) handleRedisBroadcast(payload string) {
	var message types.WebSocketMessage
	if err := json.Unmarshal([]byte(payload), &message); err != nil {
		h.logger.Error("Failed to unmarshal Redis broadcast", zap.Error(err))
		return
	}

	h.broadcast <- &message
}

// GetClientCount returns the number of connected clients
func (h *Hub) GetClientCount() int {
	h.mutex.RLock()
	defer h.mutex.RUnlock()
	return len(h.clients)
}

// SendToUser sends a message to a specific user
func (h *Hub) SendToUser(userID string, payload map[string]interface{}) {
	message := &types.WebSocketMessage{
		Type:      "notification",
		UserID:    userID,
		Payload:   payload,
		Timestamp: time.Now(),
	}
	h.Broadcast(message)
}

// SendToBank sends a message to all users of a bank
func (h *Hub) SendToBank(bankID string, payload map[string]interface{}) {
	message := &types.WebSocketMessage{
		Type:      "notification",
		BankID:    bankID,
		Payload:   payload,
		Timestamp: time.Now(),
	}
	h.Broadcast(message)
}

// Register registers a new client with the hub
func (h *Hub) Register(client *Client) {
	h.register <- client
}

// Unregister removes a client from the hub
func (h *Hub) Unregister(client *Client) {
	h.unregister <- client
}
