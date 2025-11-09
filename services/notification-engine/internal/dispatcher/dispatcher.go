package dispatcher

import (
	"context"
	"fmt"
	"time"

	"github.com/deltran/notification-engine/internal/storage"
	"github.com/deltran/notification-engine/internal/templates"
	"github.com/deltran/notification-engine/internal/websocket"
	"github.com/deltran/notification-engine/pkg/types"
	"go.uber.org/zap"
	"golang.org/x/time/rate"
)

type Dispatcher struct {
	logger       *zap.Logger
	emailSender  *EmailSender
	smsSender    *SMSSender
	wsHub        *websocket.Hub
	storage      *storage.Storage
	templateMgr  *templates.Manager
	rateLimiters map[string]*rate.Limiter
}

func NewDispatcher(
	logger *zap.Logger,
	emailSender *EmailSender,
	smsSender *SMSSender,
	wsHub *websocket.Hub,
	storage *storage.Storage,
	templateMgr *templates.Manager,
) *Dispatcher {
	return &Dispatcher{
		logger:       logger,
		emailSender:  emailSender,
		smsSender:    smsSender,
		wsHub:        wsHub,
		storage:      storage,
		templateMgr:  templateMgr,
		rateLimiters: make(map[string]*rate.Limiter),
	}
}

func (d *Dispatcher) Dispatch(ctx context.Context, notification *types.Notification) error {
	// Check rate limit
	if !d.checkRateLimit(notification.UserID) {
		d.logger.Warn("Rate limit exceeded", zap.String("user_id", notification.UserID))
		return fmt.Errorf("rate limit exceeded for user %s", notification.UserID)
	}

	// Store notification
	if err := d.storage.SaveNotification(ctx, notification); err != nil {
		d.logger.Error("Failed to save notification", zap.Error(err))
	}

	// Dispatch based on type
	switch notification.Type {
	case types.NotificationTypeEmail:
		return d.emailSender.Send(ctx, notification)

	case types.NotificationTypeSMS:
		return d.smsSender.Send(ctx, notification)

	case types.NotificationTypeWebSocket:
		d.wsHub.SendToUser(notification.UserID, notification.Metadata)
		return nil

	case types.NotificationTypePush:
		// Push notification logic (stub for MVP)
		d.logger.Info("Push notification (not implemented)", zap.String("user_id", notification.UserID))
		return nil

	default:
		return fmt.Errorf("unknown notification type: %s", notification.Type)
	}
}

func (d *Dispatcher) checkRateLimit(userID string) bool {
	limiter, exists := d.rateLimiters[userID]
	if !exists {
		limiter = rate.NewLimiter(rate.Every(time.Hour/10), 10)
		d.rateLimiters[userID] = limiter
	}
	return limiter.Allow()
}
