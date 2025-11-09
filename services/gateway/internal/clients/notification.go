package clients

import (
	"context"
	"time"
)

// NotificationClient handles communication with Notification Engine
type NotificationClient struct {
	*BaseClient
}

// NewNotificationClient creates a new notification client
func NewNotificationClient(baseURL string) *NotificationClient {
	return &NotificationClient{
		BaseClient: NewBaseClient(baseURL, "notification-engine", 3*time.Second),
	}
}

// SendNotificationRequest represents a notification send request
type SendNotificationRequest struct {
	Type      string                 `json:"type"` // email, sms, websocket, push
	Recipient string                 `json:"recipient"`
	Template  string                 `json:"template"`
	Data      map[string]interface{} `json:"data"`
	Priority  string                 `json:"priority,omitempty"` // low, normal, high
}

// SendNotificationResponse represents a notification send response
type SendNotificationResponse struct {
	NotificationID string `json:"notification_id"`
	Status         string `json:"status"`
	SentAt         string `json:"sent_at"`
}

// SendNotification sends a notification
func (c *NotificationClient) SendNotification(ctx context.Context, req SendNotificationRequest) (*SendNotificationResponse, error) {
	var result SendNotificationResponse
	err := c.Post(ctx, "/api/v1/notifications/send", req, &result)
	if err != nil {
		return nil, err
	}
	return &result, nil
}
