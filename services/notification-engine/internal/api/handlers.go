package api

import (
	"encoding/json"
	"net/http"

	"github.com/deltran/notification-engine/internal/dispatcher"
	"github.com/deltran/notification-engine/internal/storage"
	"github.com/deltran/notification-engine/internal/websocket"
	"github.com/deltran/notification-engine/pkg/types"
	"github.com/google/uuid"
	"github.com/gorilla/mux"
	ws "github.com/gorilla/websocket"
	"go.uber.org/zap"
	"time"
)

type Handler struct {
	logger     *zap.Logger
	dispatcher *dispatcher.Dispatcher
	storage    *storage.Storage
	wsHub      *websocket.Hub
	upgrader   ws.Upgrader
}

func NewHandler(logger *zap.Logger, disp *dispatcher.Dispatcher, stor *storage.Storage, hub *websocket.Hub) *Handler {
	return &Handler{
		logger:     logger,
		dispatcher: disp,
		storage:    stor,
		wsHub:      hub,
		upgrader: ws.Upgrader{
			CheckOrigin: func(r *http.Request) bool {
				return true // Allow all origins for MVP
			},
		},
	}
}

func (h *Handler) RegisterRoutes(router *mux.Router) {
	router.HandleFunc("/ws", h.handleWebSocket).Methods("GET")
	router.HandleFunc("/health", h.healthCheck).Methods("GET")

	api := router.PathPrefix("/api/v1").Subrouter()
	api.HandleFunc("/notifications", h.getNotifications).Methods("GET")
	api.HandleFunc("/notifications", h.sendNotification).Methods("POST")
	api.HandleFunc("/stats", h.getStats).Methods("GET")
}

func (h *Handler) handleWebSocket(w http.ResponseWriter, r *http.Request) {
	userID := r.URL.Query().Get("user_id")
	if userID == "" {
		http.Error(w, "user_id required", http.StatusBadRequest)
		return
	}

	conn, err := h.upgrader.Upgrade(w, r, nil)
	if err != nil {
		h.logger.Error("Failed to upgrade connection", zap.Error(err))
		return
	}

	client := websocket.NewClient(
		uuid.New().String(),
		userID,
		r.URL.Query().Get("bank_id"),
		conn,
		h.wsHub,
		h.logger,
	)

	h.wsHub.Register(client)

	go client.WritePump()
	go client.ReadPump()
}

func (h *Handler) getNotifications(w http.ResponseWriter, r *http.Request) {
	userID := r.URL.Query().Get("user_id")
	if userID == "" {
		http.Error(w, "user_id required", http.StatusBadRequest)
		return
	}

	notifications, err := h.storage.GetNotifications(r.Context(), userID, 50, 0)
	if err != nil {
		h.logger.Error("Failed to get notifications", zap.Error(err))
		http.Error(w, "Internal server error", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(notifications)
}

func (h *Handler) sendNotification(w http.ResponseWriter, r *http.Request) {
	var req types.NotificationRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid request", http.StatusBadRequest)
		return
	}

	notification := &types.Notification{
		ID:        uuid.New().String(),
		UserID:    req.UserID,
		BankID:    req.BankID,
		Type:      req.Type,
		Subject:   req.Subject,
		Content:   req.Content,
		Metadata:  req.Data,
		Status:    types.NotificationStatusPending,
		CreatedAt: time.Now(),
		UpdatedAt: time.Now(),
	}

	if err := h.dispatcher.Dispatch(r.Context(), notification); err != nil {
		h.logger.Error("Failed to dispatch notification", zap.Error(err))
		http.Error(w, "Failed to send notification", http.StatusInternalServerError)
		return
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"id":     notification.ID,
		"status": "sent",
	})
}

func (h *Handler) getStats(w http.ResponseWriter, r *http.Request) {
	stats := map[string]interface{}{
		"connected_clients": h.wsHub.GetClientCount(),
		"timestamp":         time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(stats)
}

func (h *Handler) healthCheck(w http.ResponseWriter, r *http.Request) {
	w.Header().Set("Content-Type", "application/json")
	json.NewEncoder(w).Encode(map[string]string{
		"status": "healthy",
		"service": "notification-engine",
	})
}
