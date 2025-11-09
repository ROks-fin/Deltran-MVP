package dispatcher

import (
	"context"
	"fmt"
	"net/smtp"

	"github.com/deltran/notification-engine/pkg/types"
	"go.uber.org/zap"
)

type EmailSender struct {
	logger      *zap.Logger
	smtpHost    string
	smtpPort    int
	fromAddress string
	fromName    string
}

func NewEmailSender(logger *zap.Logger, host string, port int, fromAddr, fromName string) *EmailSender {
	return &EmailSender{
		logger:      logger,
		smtpHost:    host,
		smtpPort:    port,
		fromAddress: fromAddr,
		fromName:    fromName,
	}
}

func (e *EmailSender) Send(ctx context.Context, notification *types.Notification) error {
	addr := fmt.Sprintf("%s:%d", e.smtpHost, e.smtpPort)
	
	msg := []byte(fmt.Sprintf("From: %s\r\nTo: %s\r\nSubject: %s\r\n\r\n%s\r\n",
		e.fromAddress,
		notification.Metadata["email"].(string),
		notification.Subject,
		notification.Content))

	// For MVP, use simple SMTP without auth
	err := smtp.SendMail(addr, nil, e.fromAddress, []string{notification.Metadata["email"].(string)}, msg)
	if err != nil {
		e.logger.Error("Failed to send email", zap.Error(err))
		return err
	}

	e.logger.Info("Email sent", zap.String("to", notification.Metadata["email"].(string)))
	return nil
}
