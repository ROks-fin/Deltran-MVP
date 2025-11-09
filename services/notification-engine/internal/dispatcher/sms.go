package dispatcher

import (
	"context"

	"github.com/deltran/notification-engine/pkg/types"
	"go.uber.org/zap"
)

type SMSSender struct {
	logger     *zap.Logger
	mockMode   bool
	fromNumber string
}

func NewSMSSender(logger *zap.Logger, mockMode bool, fromNumber string) *SMSSender {
	return &SMSSender{
		logger:     logger,
		mockMode:   mockMode,
		fromNumber: fromNumber,
	}
}

func (s *SMSSender) Send(ctx context.Context, notification *types.Notification) error {
	if s.mockMode {
		s.logger.Info("SMS sent (MOCK)",
			zap.String("to", notification.Metadata["phone"].(string)),
			zap.String("content", notification.Content),
		)
		return nil
	}

	// Real Twilio/SNS integration would go here
	s.logger.Info("SMS sent", zap.String("to", notification.Metadata["phone"].(string)))
	return nil
}
