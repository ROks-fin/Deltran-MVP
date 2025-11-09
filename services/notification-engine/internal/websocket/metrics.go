package websocket

import (
	"sync/atomic"
)

// Metrics holds WebSocket metrics
type Metrics struct {
	ClientsConnected int64
	MessagesSent     int64
	MessagesReceived int64
	Errors           int64
}

// NewMetrics creates a new Metrics instance
func NewMetrics() *Metrics {
	return &Metrics{}
}

// Inc increments clients connected
func (m *Metrics) Inc() {
	atomic.AddInt64(&m.ClientsConnected, 1)
}

// Dec decrements clients connected
func (m *Metrics) Dec() {
	atomic.AddInt64(&m.ClientsConnected, -1)
}

// AddMessages adds to messages sent
func (m *Metrics) AddMessages(count int64) {
	atomic.AddInt64(&m.MessagesSent, count)
}

// GetMetrics returns current metrics as a map
func (m *Metrics) GetMetrics() map[string]interface{} {
	return map[string]interface{}{
		"clients_connected":  atomic.LoadInt64(&m.ClientsConnected),
		"messages_sent":      atomic.LoadInt64(&m.MessagesSent),
		"messages_received":  atomic.LoadInt64(&m.MessagesReceived),
		"errors":             atomic.LoadInt64(&m.Errors),
	}
}
