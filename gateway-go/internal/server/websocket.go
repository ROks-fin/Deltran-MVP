package server

import (
	"context"
	"encoding/json"
	"net/http"
	"sync"
	"time"

	"github.com/gorilla/websocket"
	"github.com/nats-io/nats.go"
	"github.com/rs/zerolog/log"
)

var upgrader = websocket.Upgrader{
	ReadBufferSize:  1024,
	WriteBufferSize: 1024,
	CheckOrigin: func(r *http.Request) bool {
		// In production, check r.Header.Get("Origin") against allowed origins
		return true
	},
}

// WebSocketHub manages WebSocket connections and broadcasts
type WebSocketHub struct {
	clients    map[*WebSocketClient]bool
	broadcast  chan []byte
	register   chan *WebSocketClient
	unregister chan *WebSocketClient
	mu         sync.RWMutex
	natsConn   *nats.Conn
}

// WebSocketClient represents a connected WebSocket client
type WebSocketClient struct {
	hub  *WebSocketHub
	conn *websocket.Conn
	send chan []byte
}

// NewWebSocketHub creates a new WebSocket hub
func NewWebSocketHub(natsConn *nats.Conn) *WebSocketHub {
	return &WebSocketHub{
		clients:    make(map[*WebSocketClient]bool),
		broadcast:  make(chan []byte, 256),
		register:   make(chan *WebSocketClient),
		unregister: make(chan *WebSocketClient),
		natsConn:   natsConn,
	}
}

// Run starts the WebSocket hub
func (h *WebSocketHub) Run(ctx context.Context) {
	// Subscribe to NATS payment events
	if h.natsConn != nil {
		go h.subscribeToPaymentEvents(ctx)
	}

	for {
		select {
		case <-ctx.Done():
			// Close all client connections
			h.mu.Lock()
			for client := range h.clients {
				close(client.send)
				client.conn.Close()
			}
			h.mu.Unlock()
			return

		case client := <-h.register:
			h.mu.Lock()
			h.clients[client] = true
			h.mu.Unlock()
			log.Info().Msg("WebSocket client connected")

		case client := <-h.unregister:
			h.mu.Lock()
			if _, ok := h.clients[client]; ok {
				delete(h.clients, client)
				close(client.send)
			}
			h.mu.Unlock()
			log.Info().Msg("WebSocket client disconnected")

		case message := <-h.broadcast:
			h.mu.RLock()
			for client := range h.clients {
				select {
				case client.send <- message:
				default:
					// Client's send buffer is full, disconnect
					close(client.send)
					delete(h.clients, client)
				}
			}
			h.mu.RUnlock()
		}
	}
}

// subscribeToPaymentEvents subscribes to NATS payment events and broadcasts to WebSocket clients
func (h *WebSocketHub) subscribeToPaymentEvents(ctx context.Context) {
	if h.natsConn == nil {
		log.Warn().Msg("NATS connection not available, WebSocket will not receive live updates")
		return
	}

	// Subscribe to payment events
	subject := "deltran.payments.>"
	sub, err := h.natsConn.Subscribe(subject, func(msg *nats.Msg) {
		// Parse payment event
		var event map[string]interface{}
		if err := json.Unmarshal(msg.Data, &event); err != nil {
			log.Error().Err(err).Msg("Failed to parse payment event")
			return
		}

		// Add event type based on NATS subject
		event["event_type"] = msg.Subject
		event["timestamp"] = time.Now().UTC().Format(time.RFC3339)

		// Broadcast to all WebSocket clients
		data, err := json.Marshal(event)
		if err != nil {
			log.Error().Err(err).Msg("Failed to marshal payment event")
			return
		}

		select {
		case h.broadcast <- data:
		default:
			log.Warn().Msg("Broadcast channel full, dropping event")
		}
	})

	if err != nil {
		log.Error().Err(err).Msg("Failed to subscribe to NATS payment events")
		return
	}

	log.Info().Str("subject", subject).Msg("Subscribed to NATS payment events")

	// Wait for context cancellation
	<-ctx.Done()
	sub.Unsubscribe()
	log.Info().Msg("Unsubscribed from NATS payment events")
}

// HandleWebSocket handles WebSocket connection requests
func (h *WebSocketHub) HandleWebSocket(w http.ResponseWriter, r *http.Request) {
	// Upgrade HTTP connection to WebSocket
	conn, err := upgrader.Upgrade(w, r, nil)
	if err != nil {
		log.Error().Err(err).Msg("Failed to upgrade WebSocket connection")
		return
	}

	client := &WebSocketClient{
		hub:  h,
		conn: conn,
		send: make(chan []byte, 256),
	}

	client.hub.register <- client

	// Start goroutines for reading and writing
	go client.writePump()
	go client.readPump()
}

// readPump reads messages from the WebSocket connection
func (c *WebSocketClient) readPump() {
	defer func() {
		c.hub.unregister <- c
		c.conn.Close()
	}()

	c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(60 * time.Second))
		return nil
	})

	for {
		_, message, err := c.conn.ReadMessage()
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				log.Error().Err(err).Msg("WebSocket read error")
			}
			break
		}

		// Handle client messages (e.g., subscribe to specific payment IDs)
		var msg map[string]interface{}
		if err := json.Unmarshal(message, &msg); err == nil {
			if msgType, ok := msg["type"].(string); ok {
				switch msgType {
				case "ping":
					// Respond with pong
					pong := map[string]interface{}{
						"type":      "pong",
						"timestamp": time.Now().UTC().Format(time.RFC3339),
					}
					if data, err := json.Marshal(pong); err == nil {
						c.send <- data
					}
				case "subscribe":
					// Handle subscription to specific payment IDs or filters
					// This can be extended based on requirements
					log.Info().Interface("msg", msg).Msg("Client subscription request")
				}
			}
		}
	}
}

// writePump writes messages to the WebSocket connection
func (c *WebSocketClient) writePump() {
	ticker := time.NewTicker(54 * time.Second)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if !ok {
				// Hub closed the channel
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			w, err := c.conn.NextWriter(websocket.TextMessage)
			if err != nil {
				return
			}
			w.Write(message)

			// Add queued messages to the current message
			n := len(c.send)
			for i := 0; i < n; i++ {
				w.Write([]byte{'\n'})
				w.Write(<-c.send)
			}

			if err := w.Close(); err != nil {
				return
			}

		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(10 * time.Second))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

// BroadcastMetricsUpdate broadcasts metrics update to all connected clients
func (h *WebSocketHub) BroadcastMetricsUpdate(metrics interface{}) {
	data, err := json.Marshal(map[string]interface{}{
		"type":      "metrics_update",
		"data":      metrics,
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
	if err != nil {
		log.Error().Err(err).Msg("Failed to marshal metrics update")
		return
	}

	select {
	case h.broadcast <- data:
	default:
		log.Warn().Msg("Broadcast channel full, dropping metrics update")
	}
}

// BroadcastPaymentUpdate broadcasts payment update to all connected clients
func (h *WebSocketHub) BroadcastPaymentUpdate(payment interface{}) {
	data, err := json.Marshal(map[string]interface{}{
		"type":      "payment_update",
		"data":      payment,
		"timestamp": time.Now().UTC().Format(time.RFC3339),
	})
	if err != nil {
		log.Error().Err(err).Msg("Failed to marshal payment update")
		return
	}

	select {
	case h.broadcast <- data:
	default:
		log.Warn().Msg("Broadcast channel full, dropping payment update")
	}
}

// GetConnectedClientsCount returns the number of currently connected WebSocket clients
func (h *WebSocketHub) GetConnectedClientsCount() int {
	h.mu.RLock()
	defer h.mu.RUnlock()
	return len(h.clients)
}
