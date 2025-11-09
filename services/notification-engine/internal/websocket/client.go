package websocket

import (
	"time"

	"github.com/deltran/notification-engine/pkg/types"
	"github.com/gorilla/websocket"
	"go.uber.org/zap"
)

const (
	// Time allowed to write a message to the peer
	writeWait = 10 * time.Second

	// Time allowed to read the next pong message from the peer
	pongWait = 60 * time.Second

	// Send pings to peer with this period
	pingPeriod = 30 * time.Second

	// Maximum message size allowed from peer
	maxMessageSize = 512 * 1024 // 512KB
)

// Client represents a single websocket connection
type Client struct {
	// Unique client identifier
	ID string

	// User ID associated with this client
	UserID string

	// Bank ID (optional)
	BankID string

	// The websocket connection
	conn *websocket.Conn

	// Hub that owns this client
	hub *Hub

	// Buffered channel of outbound messages
	send chan *types.WebSocketMessage

	// Ping channel
	ping chan bool

	// Logger
	logger *zap.Logger
}

// NewClient creates a new client instance
func NewClient(id, userID, bankID string, conn *websocket.Conn, hub *Hub, logger *zap.Logger) *Client {
	return &Client{
		ID:     id,
		UserID: userID,
		BankID: bankID,
		conn:   conn,
		hub:    hub,
		send:   make(chan *types.WebSocketMessage, 256),
		ping:   make(chan bool, 1),
		logger: logger,
	}
}

// ReadPump pumps messages from the websocket connection to the hub
func (c *Client) ReadPump() {
	defer func() {
		c.hub.Unregister(c)
		c.conn.Close()
	}()

	c.conn.SetReadDeadline(time.Now().Add(pongWait))
	c.conn.SetReadLimit(maxMessageSize)
	c.conn.SetPongHandler(func(string) error {
		c.conn.SetReadDeadline(time.Now().Add(pongWait))
		return nil
	})

	for {
		var msg types.WebSocketMessage
		err := c.conn.ReadJSON(&msg)
		if err != nil {
			if websocket.IsUnexpectedCloseError(err, websocket.CloseGoingAway, websocket.CloseAbnormalClosure) {
				c.logger.Error("WebSocket read error",
					zap.String("client_id", c.ID),
					zap.Error(err),
				)
			}
			break
		}

		// Handle client messages (e.g., acks, subscription updates)
		c.handleMessage(&msg)
	}
}

// WritePump pumps messages from the hub to the websocket connection
func (c *Client) WritePump() {
	ticker := time.NewTicker(pingPeriod)
	defer func() {
		ticker.Stop()
		c.conn.Close()
	}()

	for {
		select {
		case message, ok := <-c.send:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if !ok {
				// The hub closed the channel
				c.conn.WriteMessage(websocket.CloseMessage, []byte{})
				return
			}

			err := c.conn.WriteJSON(message)
			if err != nil {
				c.logger.Error("WebSocket write error",
					zap.String("client_id", c.ID),
					zap.Error(err),
				)
				return
			}

		case <-c.ping:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				c.logger.Error("WebSocket ping error",
					zap.String("client_id", c.ID),
					zap.Error(err),
				)
				return
			}

		case <-ticker.C:
			c.conn.SetWriteDeadline(time.Now().Add(writeWait))
			if err := c.conn.WriteMessage(websocket.PingMessage, nil); err != nil {
				return
			}
		}
	}
}

// handleMessage handles incoming messages from the client
func (c *Client) handleMessage(msg *types.WebSocketMessage) {
	switch msg.Type {
	case "ping":
		// Respond with pong
		response := &types.WebSocketMessage{
			Type:      "pong",
			Timestamp: time.Now(),
		}
		c.send <- response

	case "ack":
		// Handle acknowledgment of received notification
		if notifID, ok := msg.Payload["notification_id"].(string); ok {
			c.logger.Debug("Notification acknowledged",
				zap.String("notification_id", notifID),
				zap.String("user_id", c.UserID),
			)
		}

	case "subscribe":
		// Handle subscription to specific topics
		c.logger.Debug("Subscribe request",
			zap.String("user_id", c.UserID),
			zap.Any("payload", msg.Payload),
		)

	default:
		c.logger.Warn("Unknown message type",
			zap.String("type", msg.Type),
			zap.String("user_id", c.UserID),
		)
	}
}

// Send sends a message to the client
func (c *Client) Send(message *types.WebSocketMessage) {
	select {
	case c.send <- message:
	default:
		c.logger.Warn("Client send buffer full",
			zap.String("client_id", c.ID),
		)
	}
}
