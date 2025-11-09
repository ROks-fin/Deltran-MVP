package storage

import (
	"context"
	"database/sql"
	"encoding/json"
	"fmt"

	"github.com/deltran/notification-engine/pkg/types"
	_ "github.com/lib/pq"
	"go.uber.org/zap"
)

type Storage struct {
	db     *sql.DB
	logger *zap.Logger
}

func NewStorage(connStr string, logger *zap.Logger) (*Storage, error) {
	db, err := sql.Open("postgres", connStr)
	if err != nil {
		return nil, fmt.Errorf("failed to open database: %w", err)
	}

	if err := db.Ping(); err != nil {
		return nil, fmt.Errorf("failed to ping database: %w", err)
	}

	logger.Info("Connected to PostgreSQL")

	return &Storage{
		db:     db,
		logger: logger,
	}, nil
}

func (s *Storage) SaveNotification(ctx context.Context, notification *types.Notification) error {
	metadata, _ := json.Marshal(notification.Metadata)

	query := `
		INSERT INTO notifications (id, user_id, bank_id, type, template_id, subject, content, metadata, status, created_at, updated_at)
		VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10, $11)
	`

	_, err := s.db.ExecContext(ctx, query,
		notification.ID,
		notification.UserID,
		notification.BankID,
		notification.Type,
		notification.TemplateID,
		notification.Subject,
		notification.Content,
		metadata,
		notification.Status,
		notification.CreatedAt,
		notification.UpdatedAt,
	)

	return err
}

func (s *Storage) GetNotifications(ctx context.Context, userID string, limit, offset int) ([]*types.Notification, error) {
	query := `
		SELECT id, user_id, bank_id, type, template_id, subject, content, metadata, status, sent_at, read_at, retry_count, error_message, created_at, updated_at
		FROM notifications
		WHERE user_id = $1
		ORDER BY created_at DESC
		LIMIT $2 OFFSET $3
	`

	rows, err := s.db.QueryContext(ctx, query, userID, limit, offset)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	var notifications []*types.Notification
	for rows.Next() {
		var n types.Notification
		var metadata []byte

		err := rows.Scan(
			&n.ID, &n.UserID, &n.BankID, &n.Type, &n.TemplateID, &n.Subject, &n.Content,
			&metadata, &n.Status, &n.SentAt, &n.ReadAt, &n.RetryCount, &n.ErrorMsg,
			&n.CreatedAt, &n.UpdatedAt,
		)
		if err != nil {
			return nil, err
		}

		json.Unmarshal(metadata, &n.Metadata)
		notifications = append(notifications, &n)
	}

	return notifications, nil
}

func (s *Storage) Close() error {
	return s.db.Close()
}
