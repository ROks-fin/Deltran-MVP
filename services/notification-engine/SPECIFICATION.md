# Notification Engine Service Specification

## Overview
High-performance real-time notification system handling WebSocket connections, email/SMS dispatch, and event distribution across the DelTran platform.

## Core Responsibilities
1. WebSocket connection management for real-time updates
2. Email/SMS notification dispatch with templates
3. Event aggregation and filtering
4. Delivery guarantees and retry logic
5. Notification preference management
6. Audit trail for all notifications

## Technology Stack
- **Language**: Go 1.21+
- **WebSocket**: gorilla/websocket for real-time connections
- **Message Queue**: NATS JetStream for event consumption
- **Database**: PostgreSQL 16 for notification history
- **Cache**: Redis for connection state and rate limiting
- **Email**: SendGrid/AWS SES integration
- **SMS**: Twilio/AWS SNS integration
- **Template Engine**: Go templates with i18n support

## Service Architecture

### Directory Structure
```
services/notification-engine/
├── cmd/
│   └── server/
│       └── main.go              # Service entry point
├── internal/
│   ├── config/
│   │   └── config.go            # Configuration management
│   ├── websocket/
│   │   ├── hub.go               # WebSocket connection hub
│   │   ├── client.go            # WebSocket client handler
│   │   └── message.go           # Message types
│   ├── dispatcher/
│   │   ├── dispatcher.go        # Main notification dispatcher
│   │   ├── email.go             # Email dispatcher
│   │   └── sms.go              # SMS dispatcher
│   ├── templates/
│   │   ├── manager.go           # Template management
│   │   └── i18n.go             # Internationalization
│   ├── storage/
│   │   ├── postgres.go          # Notification history
│   │   └── redis.go            # Connection state cache
│   ├── consumer/
│   │   └── nats.go             # NATS event consumer
│   └── api/
│       ├── handlers.go          # REST API handlers
│       └── middleware.go        # Authentication middleware
├── pkg/
│   └── types/
│       └── notification.go      # Shared types
├── templates/                    # Email/SMS templates
│   ├── email/
│   └── sms/
├── Dockerfile
├── Makefile
└── go.mod
```

## Implementation Details

### 1. WebSocket Hub Implementation

```go
// internal/websocket/hub.go
package websocket

import (
    "context"
    "sync"
    "time"

    "github.com/gorilla/websocket"
    "github.com/redis/go-redis/v9"
)

type Hub struct {
    clients    map[string]*Client
    broadcast  chan *Message
    register   chan *Client
    unregister chan *Client
    mutex      sync.RWMutex
    redis      *redis.Client
}

func NewHub(redis *redis.Client) *Hub {
    return &Hub{
        clients:    make(map[string]*Client),
        broadcast:  make(chan *Message, 1000),
        register:   make(chan *Client, 100),
        unregister: make(chan *Client, 100),
        redis:      redis,
    }
}

func (h *Hub) Run(ctx context.Context) {
    ticker := time.NewTicker(30 * time.Second)
    defer ticker.Stop()

    for {
        select {
        case <-ctx.Done():
            return

        case client := <-h.register:
            h.mutex.Lock()
            h.clients[client.ID] = client
            h.mutex.Unlock()

            // Store connection in Redis for horizontal scaling
            h.redis.Set(ctx, "ws:conn:"+client.ID, client.ServerID, 5*time.Minute)

        case client := <-h.unregister:
            h.mutex.Lock()
            if _, ok := h.clients[client.ID]; ok {
                delete(h.clients, client.ID)
                close(client.send)
            }
            h.mutex.Unlock()

            h.redis.Del(ctx, "ws:conn:"+client.ID)

        case message := <-h.broadcast:
            h.sendToClients(message)

        case <-ticker.C:
            h.pingClients()
        }
    }
}

func (h *Hub) sendToClients(message *Message) {
    h.mutex.RLock()
    defer h.mutex.RUnlock()

    for _, client := range h.clients {
        if h.shouldReceive(client, message) {
            select {
            case client.send <- message:
            default:
                // Client buffer full, close connection
                close(client.send)
                delete(h.clients, client.ID)
            }
        }
    }
}
```

### 2. Notification Dispatcher

```go
// internal/dispatcher/dispatcher.go
package dispatcher

import (
    "context"
    "encoding/json"
    "fmt"
    "time"

    "github.com/nats-io/nats.go"
    "go.uber.org/zap"
)

type Dispatcher struct {
    logger        *zap.Logger
    emailSender   *EmailSender
    smsSender     *SMSSender
    wsHub         *websocket.Hub
    natsConn      *nats.Conn
    jetStream     nats.JetStreamContext
    rateLimiter   *RateLimiter
}

func (d *Dispatcher) Start(ctx context.Context) error {
    // Subscribe to notification events
    sub, err := d.jetStream.PullSubscribe(
        "notifications.*",
        "notification-engine",
        nats.MaxDeliver(3),
        nats.AckWait(30*time.Second),
    )
    if err != nil {
        return fmt.Errorf("failed to subscribe: %w", err)
    }

    go d.processNotifications(ctx, sub)
    return nil
}

func (d *Dispatcher) processNotifications(ctx context.Context, sub *nats.Subscription) {
    for {
        select {
        case <-ctx.Done():
            return
        default:
            msgs, err := sub.Fetch(10, nats.MaxWait(1*time.Second))
            if err != nil && err != nats.ErrTimeout {
                d.logger.Error("failed to fetch messages", zap.Error(err))
                continue
            }

            for _, msg := range msgs {
                go d.handleMessage(ctx, msg)
            }
        }
    }
}

func (d *Dispatcher) handleMessage(ctx context.Context, msg *nats.Msg) {
    defer msg.Ack()

    var notification Notification
    if err := json.Unmarshal(msg.Data, &notification); err != nil {
        d.logger.Error("failed to unmarshal notification", zap.Error(err))
        return
    }

    // Check rate limits
    if !d.rateLimiter.Allow(notification.UserID) {
        d.logger.Warn("rate limit exceeded", zap.String("user_id", notification.UserID))
        return
    }

    // Dispatch based on type
    switch notification.Type {
    case NotificationTypeEmail:
        if err := d.emailSender.Send(ctx, &notification); err != nil {
            d.logger.Error("failed to send email", zap.Error(err))
            d.scheduleRetry(ctx, &notification)
        }

    case NotificationTypeSMS:
        if err := d.smsSender.Send(ctx, &notification); err != nil {
            d.logger.Error("failed to send SMS", zap.Error(err))
            d.scheduleRetry(ctx, &notification)
        }

    case NotificationTypeWebSocket:
        d.wsHub.Broadcast(&websocket.Message{
            Type:    "notification",
            UserID:  notification.UserID,
            Payload: notification.Payload,
        })

    case NotificationTypePush:
        // Mobile push notification logic
    }

    // Store in history
    d.storeNotificationHistory(ctx, &notification)
}

func (d *Dispatcher) scheduleRetry(ctx context.Context, notification *Notification) {
    notification.RetryCount++
    if notification.RetryCount > 3 {
        d.logger.Error("max retries exceeded",
            zap.String("notification_id", notification.ID))
        return
    }

    // Exponential backoff
    delay := time.Duration(notification.RetryCount) * 5 * time.Minute

    data, _ := json.Marshal(notification)
    d.jetStream.PublishAsync(
        fmt.Sprintf("notifications.retry.%s", notification.Type),
        data,
        nats.MsgId(notification.ID),
        nats.RetryAttempts(3),
    )
}
```

### 3. Template Management

```go
// internal/templates/manager.go
package templates

import (
    "bytes"
    "html/template"
    "sync"

    "github.com/nicksnyder/go-i18n/v2/i18n"
)

type TemplateManager struct {
    emailTemplates map[string]*template.Template
    smsTemplates   map[string]*template.Template
    localizer      map[string]*i18n.Localizer
    mutex          sync.RWMutex
}

func (tm *TemplateManager) RenderEmail(
    templateName string,
    locale string,
    data interface{},
) (string, error) {
    tm.mutex.RLock()
    defer tm.mutex.RUnlock()

    tmpl, ok := tm.emailTemplates[templateName]
    if !ok {
        return "", fmt.Errorf("template not found: %s", templateName)
    }

    // Localize data
    localizedData := tm.localizeData(locale, data)

    var buf bytes.Buffer
    if err := tmpl.Execute(&buf, localizedData); err != nil {
        return "", fmt.Errorf("failed to render template: %w", err)
    }

    return buf.String(), nil
}

// Email templates
const transactionConfirmationTemplate = `
<!DOCTYPE html>
<html>
<head>
    <style>
        .container { max-width: 600px; margin: 0 auto; font-family: Arial, sans-serif; }
        .header { background-color: #2c3e50; color: white; padding: 20px; text-align: center; }
        .content { padding: 20px; background-color: #f8f9fa; }
        .amount { font-size: 24px; font-weight: bold; color: #27ae60; }
        .details { margin-top: 20px; border-top: 1px solid #ddd; padding-top: 20px; }
        .footer { text-align: center; padding: 20px; color: #7f8c8d; }
    </style>
</head>
<body>
    <div class="container">
        <div class="header">
            <h1>{{.Title}}</h1>
        </div>
        <div class="content">
            <p>{{.Greeting}} {{.RecipientName}},</p>

            <p>{{.Message}}</p>

            <div class="amount">
                {{.Amount}} {{.Currency}}
            </div>

            <div class="details">
                <p><strong>{{.TransactionIDLabel}}:</strong> {{.TransactionID}}</p>
                <p><strong>{{.DateLabel}}:</strong> {{.Date}}</p>
                <p><strong>{{.StatusLabel}}:</strong> {{.Status}}</p>
                {{if .EstimatedSettlement}}
                <p><strong>{{.EstimatedSettlementLabel}}:</strong> {{.EstimatedSettlement}}</p>
                {{end}}
            </div>

            <p>{{.SecurityNote}}</p>
        </div>
        <div class="footer">
            <p>{{.FooterText}}</p>
            <p>© 2024 DelTran. All rights reserved.</p>
        </div>
    </div>
</body>
</html>
`
```

### 4. NATS Consumer Configuration

```go
// internal/consumer/nats.go
package consumer

import (
    "context"
    "encoding/json"
    "fmt"

    "github.com/nats-io/nats.go"
)

type EventConsumer struct {
    conn      *nats.Conn
    jetStream nats.JetStreamContext
    handlers  map[string]EventHandler
}

func NewEventConsumer(natsURL string) (*EventConsumer, error) {
    conn, err := nats.Connect(
        natsURL,
        nats.MaxReconnects(10),
        nats.ReconnectWait(2*time.Second),
        nats.DisconnectErrHandler(func(_ *nats.Conn, err error) {
            if err != nil {
                log.Printf("Disconnected from NATS: %v", err)
            }
        }),
    )
    if err != nil {
        return nil, err
    }

    js, err := conn.JetStream(
        nats.PublishAsyncMaxPending(256),
        nats.PublishAsyncErrHandler(func(_ nats.JetStream, _ *nats.Msg, err error) {
            log.Printf("Async publish error: %v", err)
        }),
    )
    if err != nil {
        return nil, err
    }

    return &EventConsumer{
        conn:      conn,
        jetStream: js,
        handlers:  make(map[string]EventHandler),
    }, nil
}

func (ec *EventConsumer) SubscribeToEvents(ctx context.Context) error {
    // Create durable consumers for each event type
    eventTypes := []string{
        "payment.initiated",
        "payment.completed",
        "payment.failed",
        "settlement.completed",
        "compliance.alert",
        "risk.threshold.exceeded",
        "netting.cycle.completed",
        "liquidity.low",
    }

    for _, eventType := range eventTypes {
        streamName := fmt.Sprintf("events.%s", eventType)

        // Create or update stream
        _, err := ec.jetStream.AddStream(&nats.StreamConfig{
            Name:       streamName,
            Subjects:   []string{streamName},
            Retention:  nats.WorkQueuePolicy,
            MaxAge:     7 * 24 * time.Hour,
            Storage:    nats.FileStorage,
            Replicas:   3,
        })
        if err != nil {
            return fmt.Errorf("failed to create stream %s: %w", streamName, err)
        }

        // Create durable consumer
        _, err = ec.jetStream.AddConsumer(streamName, &nats.ConsumerConfig{
            Durable:       "notification-engine",
            DeliverPolicy: nats.DeliverAllPolicy,
            AckPolicy:     nats.AckExplicitPolicy,
            MaxDeliver:    3,
            AckWait:       30 * time.Second,
        })
        if err != nil {
            return fmt.Errorf("failed to create consumer: %w", err)
        }
    }

    return nil
}
```

### 5. API Handlers

```go
// internal/api/handlers.go
package api

import (
    "encoding/json"
    "net/http"

    "github.com/gorilla/mux"
    "github.com/gorilla/websocket"
)

type Handler struct {
    wsHub      *websocket.Hub
    dispatcher *dispatcher.Dispatcher
    storage    *storage.Storage
    upgrader   websocket.Upgrader
}

func (h *Handler) RegisterRoutes(router *mux.Router) {
    // WebSocket endpoint
    router.HandleFunc("/ws", h.handleWebSocket).Methods("GET")

    // REST endpoints
    api := router.PathPrefix("/api/v1").Subrouter()
    api.Use(AuthMiddleware)

    api.HandleFunc("/notifications", h.getNotifications).Methods("GET")
    api.HandleFunc("/notifications/{id}", h.getNotification).Methods("GET")
    api.HandleFunc("/notifications/preferences", h.getPreferences).Methods("GET")
    api.HandleFunc("/notifications/preferences", h.updatePreferences).Methods("PUT")
    api.HandleFunc("/notifications/test", h.sendTestNotification).Methods("POST")
    api.HandleFunc("/notifications/bulk", h.sendBulkNotification).Methods("POST")

    // Admin endpoints
    admin := api.PathPrefix("/admin").Subrouter()
    admin.Use(AdminAuthMiddleware)

    admin.HandleFunc("/templates", h.getTemplates).Methods("GET")
    admin.HandleFunc("/templates", h.createTemplate).Methods("POST")
    admin.HandleFunc("/templates/{id}", h.updateTemplate).Methods("PUT")
    admin.HandleFunc("/stats", h.getStats).Methods("GET")
}

func (h *Handler) handleWebSocket(w http.ResponseWriter, r *http.Request) {
    // Extract user ID from JWT token
    userID := r.Context().Value("user_id").(string)

    conn, err := h.upgrader.Upgrade(w, r, nil)
    if err != nil {
        http.Error(w, "Failed to upgrade connection", http.StatusBadRequest)
        return
    }

    client := &websocket.Client{
        ID:     userID,
        Conn:   conn,
        Hub:    h.wsHub,
        Send:   make(chan *websocket.Message, 256),
    }

    h.wsHub.Register(client)

    go client.WritePump()
    go client.ReadPump()
}

func (h *Handler) getNotifications(w http.ResponseWriter, r *http.Request) {
    userID := r.Context().Value("user_id").(string)

    // Parse query parameters
    limit := 50
    offset := 0
    if l := r.URL.Query().Get("limit"); l != "" {
        fmt.Sscanf(l, "%d", &limit)
    }
    if o := r.URL.Query().Get("offset"); o != "" {
        fmt.Sscanf(o, "%d", &offset)
    }

    notifications, err := h.storage.GetNotifications(r.Context(), userID, limit, offset)
    if err != nil {
        http.Error(w, "Failed to fetch notifications", http.StatusInternalServerError)
        return
    }

    w.Header().Set("Content-Type", "application/json")
    json.NewEncoder(w).Encode(notifications)
}
```

### 6. Database Schema

```sql
-- Notification history
CREATE TABLE notifications (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    user_id VARCHAR(50) NOT NULL,
    type VARCHAR(20) NOT NULL CHECK (type IN ('email', 'sms', 'websocket', 'push')),
    template_id VARCHAR(50),
    subject VARCHAR(255),
    content TEXT,
    metadata JSONB,
    status VARCHAR(20) NOT NULL DEFAULT 'pending',
    sent_at TIMESTAMP,
    read_at TIMESTAMP,
    retry_count INT DEFAULT 0,
    error_message TEXT,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_notifications_user_id ON notifications(user_id);
CREATE INDEX idx_notifications_status ON notifications(status);
CREATE INDEX idx_notifications_created_at ON notifications(created_at DESC);

-- User preferences
CREATE TABLE notification_preferences (
    user_id VARCHAR(50) PRIMARY KEY,
    email_enabled BOOLEAN DEFAULT true,
    sms_enabled BOOLEAN DEFAULT true,
    push_enabled BOOLEAN DEFAULT true,
    websocket_enabled BOOLEAN DEFAULT true,
    email_address VARCHAR(255),
    phone_number VARCHAR(20),
    locale VARCHAR(10) DEFAULT 'en',
    timezone VARCHAR(50) DEFAULT 'UTC',
    quiet_hours_start TIME,
    quiet_hours_end TIME,
    preferences JSONB DEFAULT '{}',
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

-- Notification templates
CREATE TABLE notification_templates (
    id VARCHAR(50) PRIMARY KEY,
    name VARCHAR(100) NOT NULL,
    type VARCHAR(20) NOT NULL CHECK (type IN ('email', 'sms')),
    subject VARCHAR(255),
    body TEXT NOT NULL,
    locale VARCHAR(10) DEFAULT 'en',
    variables JSONB DEFAULT '[]',
    active BOOLEAN DEFAULT true,
    version INT DEFAULT 1,
    created_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP,
    updated_at TIMESTAMP DEFAULT CURRENT_TIMESTAMP
);

CREATE INDEX idx_templates_type_locale ON notification_templates(type, locale);
```

### 7. Docker Configuration

```dockerfile
# Dockerfile
FROM golang:1.21-alpine AS builder

RUN apk add --no-cache git make

WORKDIR /app
COPY go.mod go.sum ./
RUN go mod download

COPY . .
RUN CGO_ENABLED=0 GOOS=linux go build -a -installsuffix cgo -o notification-engine cmd/server/main.go

FROM alpine:latest

RUN apk --no-cache add ca-certificates tzdata
WORKDIR /root/

COPY --from=builder /app/notification-engine .
COPY --from=builder /app/templates ./templates

EXPOSE 8085 8086

CMD ["./notification-engine"]
```

### 8. Configuration

```yaml
# config/config.yaml
server:
  http_port: 8085
  ws_port: 8086
  read_timeout: 10s
  write_timeout: 10s
  idle_timeout: 120s

nats:
  url: nats://nats:4222
  cluster_id: deltran-cluster
  client_id: notification-engine

database:
  host: postgres
  port: 5432
  name: deltran
  user: deltran
  password: ${DB_PASSWORD}
  max_connections: 25

redis:
  address: redis:6379
  password: ${REDIS_PASSWORD}
  db: 2

email:
  provider: sendgrid
  api_key: ${SENDGRID_API_KEY}
  from_address: noreply@deltran.com
  from_name: DelTran

sms:
  provider: twilio
  account_sid: ${TWILIO_ACCOUNT_SID}
  auth_token: ${TWILIO_AUTH_TOKEN}
  from_number: ${TWILIO_PHONE_NUMBER}

websocket:
  ping_interval: 30s
  pong_wait: 60s
  write_wait: 10s
  max_message_size: 512KB

rate_limiting:
  email_per_hour: 10
  sms_per_hour: 5
  push_per_hour: 50

monitoring:
  prometheus_enabled: true
  metrics_port: 9095
```

## Integration Points

### 1. Event Sources
- Payment Service: Transaction status updates
- Settlement Engine: Settlement completions
- Risk Engine: Risk alerts and threshold breaches
- Compliance Engine: AML/sanctions alerts
- Clearing Engine: Netting cycle completions

### 2. External Services
- SendGrid/AWS SES for email
- Twilio/AWS SNS for SMS
- Firebase/APNS for push notifications
- Grafana for alert management

## Performance Requirements

- WebSocket connections: 10,000+ concurrent
- Message throughput: 5,000 msg/sec
- Email delivery: < 2 seconds
- SMS delivery: < 5 seconds
- WebSocket latency: < 100ms
- History query: < 200ms for 1000 records

## Security Considerations

1. **WebSocket Security**
   - JWT authentication required
   - Rate limiting per connection
   - Message size limits

2. **Template Security**
   - XSS prevention in HTML templates
   - SQL injection prevention
   - Template variable sanitization

3. **Delivery Security**
   - SPF/DKIM for emails
   - Phone number verification for SMS
   - Encrypted storage of sensitive data

## Monitoring & Metrics

### Key Metrics
```prometheus
# WebSocket metrics
notification_websocket_connections_total
notification_websocket_messages_sent_total
notification_websocket_errors_total

# Delivery metrics
notification_emails_sent_total
notification_sms_sent_total
notification_delivery_latency_seconds
notification_delivery_errors_total

# Queue metrics
notification_queue_depth
notification_queue_processing_rate
```

## Testing Strategy

1. **Unit Tests**
   - Template rendering
   - Message formatting
   - Rate limiting logic

2. **Integration Tests**
   - NATS consumer/producer
   - Database operations
   - External service mocking

3. **Load Tests**
   - 10,000 WebSocket connections
   - 5,000 msg/sec throughput
   - Bulk notification sending

## Deployment Checklist

- [ ] Configure NATS streams and consumers
- [ ] Set up email/SMS provider accounts
- [ ] Load notification templates
- [ ] Configure rate limits
- [ ] Set up monitoring dashboards
- [ ] Test WebSocket connectivity
- [ ] Verify template rendering
- [ ] Test failover scenarios