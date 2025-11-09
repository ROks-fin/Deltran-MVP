package types

import (
	"time"
)

// NotificationType defines the type of notification
type NotificationType string

const (
	NotificationTypeEmail     NotificationType = "email"
	NotificationTypeSMS       NotificationType = "sms"
	NotificationTypeWebSocket NotificationType = "websocket"
	NotificationTypePush      NotificationType = "push"
)

// NotificationStatus defines the delivery status
type NotificationStatus string

const (
	NotificationStatusPending   NotificationStatus = "pending"
	NotificationStatusSent      NotificationStatus = "sent"
	NotificationStatusDelivered NotificationStatus = "delivered"
	NotificationStatusFailed    NotificationStatus = "failed"
	NotificationStatusRetrying  NotificationStatus = "retrying"
)

// Notification represents a notification to be sent
type Notification struct {
	ID          string                 `json:"id"`
	UserID      string                 `json:"user_id"`
	BankID      string                 `json:"bank_id,omitempty"`
	Type        NotificationType       `json:"type"`
	TemplateID  string                 `json:"template_id,omitempty"`
	Subject     string                 `json:"subject,omitempty"`
	Content     string                 `json:"content"`
	Metadata    map[string]interface{} `json:"metadata,omitempty"`
	Status      NotificationStatus     `json:"status"`
	SentAt      *time.Time             `json:"sent_at,omitempty"`
	ReadAt      *time.Time             `json:"read_at,omitempty"`
	RetryCount  int                    `json:"retry_count"`
	ErrorMsg    string                 `json:"error_message,omitempty"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
}

// NotificationPreferences stores user notification preferences
type NotificationPreferences struct {
	UserID            string                 `json:"user_id"`
	EmailEnabled      bool                   `json:"email_enabled"`
	SMSEnabled        bool                   `json:"sms_enabled"`
	PushEnabled       bool                   `json:"push_enabled"`
	WebSocketEnabled  bool                   `json:"websocket_enabled"`
	EmailAddress      string                 `json:"email_address,omitempty"`
	PhoneNumber       string                 `json:"phone_number,omitempty"`
	Locale            string                 `json:"locale"`
	Timezone          string                 `json:"timezone"`
	QuietHoursStart   string                 `json:"quiet_hours_start,omitempty"`
	QuietHoursEnd     string                 `json:"quiet_hours_end,omitempty"`
	Preferences       map[string]interface{} `json:"preferences,omitempty"`
	CreatedAt         time.Time              `json:"created_at"`
	UpdatedAt         time.Time              `json:"updated_at"`
}

// NotificationTemplate represents a notification template
type NotificationTemplate struct {
	ID        string           `json:"id"`
	Name      string           `json:"name"`
	Type      NotificationType `json:"type"`
	Subject   string           `json:"subject,omitempty"`
	Body      string           `json:"body"`
	Locale    string           `json:"locale"`
	Variables []string         `json:"variables,omitempty"`
	Active    bool             `json:"active"`
	Version   int              `json:"version"`
	CreatedAt time.Time        `json:"created_at"`
	UpdatedAt time.Time        `json:"updated_at"`
}

// WebSocketMessage represents a message sent through WebSocket
type WebSocketMessage struct {
	Type      string                 `json:"type"`
	UserID    string                 `json:"user_id,omitempty"`
	BankID    string                 `json:"bank_id,omitempty"`
	Payload   map[string]interface{} `json:"payload"`
	Timestamp time.Time              `json:"timestamp"`
}

// Event represents an event from NATS JetStream
type Event struct {
	ID          string                 `json:"id"`
	Type        string                 `json:"type"`
	Source      string                 `json:"source"`
	Data        map[string]interface{} `json:"data"`
	Timestamp   time.Time              `json:"timestamp"`
	UserID      string                 `json:"user_id,omitempty"`
	BankID      string                 `json:"bank_id,omitempty"`
	CorrelationID string               `json:"correlation_id,omitempty"`
}

// NotificationRequest is the request to send a notification
type NotificationRequest struct {
	UserID     string                 `json:"user_id"`
	BankID     string                 `json:"bank_id,omitempty"`
	Type       NotificationType       `json:"type"`
	TemplateID string                 `json:"template_id,omitempty"`
	Subject    string                 `json:"subject,omitempty"`
	Content    string                 `json:"content,omitempty"`
	Data       map[string]interface{} `json:"data,omitempty"`
}

// NotificationResponse is the response after sending a notification
type NotificationResponse struct {
	ID        string             `json:"id"`
	Status    NotificationStatus `json:"status"`
	Message   string             `json:"message,omitempty"`
	CreatedAt time.Time          `json:"created_at"`
}
