// HTTP REST API handlers
package server

import (
	"encoding/json"
	"net/http"
	"sync"
	"time"

	"github.com/deltran/gateway/internal/types"
	"github.com/google/uuid"
	"github.com/shopspring/decimal"
)

// LiveMetrics stores real-time metrics
type LiveMetrics struct {
	mu                sync.RWMutex
	TPS               int       `json:"tps"`
	LatencyP95        int       `json:"latency_p95_ms"`
	ErrorRate         float64   `json:"error_rate"`
	TotalPayments     int64     `json:"total_payments"`
	SuccessfulPayments int64    `json:"successful_payments"`
	FailedPayments    int64     `json:"failed_payments"`
	QueueDepth        int       `json:"queue_depth"`
	LastUpdate        time.Time `json:"last_update"`
}

// RecentTransaction represents a recent transaction for the feed
type RecentTransaction struct {
	PaymentID string    `json:"payment_id"`
	Amount    string    `json:"amount"`
	Currency  string    `json:"currency"`
	FromBank  string    `json:"from_bank"`
	ToBank    string    `json:"to_bank"`
	Status    string    `json:"status"`
	Timestamp time.Time `json:"timestamp"`
}

var (
	globalMetrics = &LiveMetrics{
		TPS:        0,
		LatencyP95: 75,
		ErrorRate:  0.5,
		LastUpdate: time.Now(),
	}
	recentTransactions = make([]RecentTransaction, 0, 100)
	transactionsMu     sync.RWMutex
)

// UpdateMetrics updates the global metrics
func (s *Server) UpdateMetrics(tps int, latency int, queueDepth int) {
	globalMetrics.mu.Lock()
	defer globalMetrics.mu.Unlock()

	globalMetrics.TPS = tps
	globalMetrics.LatencyP95 = latency
	globalMetrics.QueueDepth = queueDepth
	globalMetrics.LastUpdate = time.Now()
}

// AddRecentTransaction adds a transaction to the recent feed
func AddRecentTransaction(tx RecentTransaction) {
	transactionsMu.Lock()
	defer transactionsMu.Unlock()

	recentTransactions = append([]RecentTransaction{tx}, recentTransactions...)
	if len(recentTransactions) > 100 {
		recentTransactions = recentTransactions[:100]
	}
}

// HandleMetricsAPI returns live metrics as JSON
func (s *Server) HandleMetricsAPI(w http.ResponseWriter, r *http.Request) {
	globalMetrics.mu.RLock()
	defer globalMetrics.mu.RUnlock()

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")

	json.NewEncoder(w).Encode(globalMetrics)
}

// HandleRecentTransactions returns recent transactions
func (s *Server) HandleRecentTransactions(w http.ResponseWriter, r *http.Request) {
	transactionsMu.RLock()
	defer transactionsMu.RUnlock()

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")

	json.NewEncoder(w).Encode(recentTransactions)
}

// HandleSubmitPayment handles payment submission via REST
func (s *Server) HandleSubmitPayment(w http.ResponseWriter, r *http.Request) {
	if r.Method != http.MethodPost {
		http.Error(w, "Method not allowed", http.StatusMethodNotAllowed)
		return
	}

	var req struct {
		FromAccount string `json:"from_account"`
		ToAccount   string `json:"to_account"`
		Amount      string `json:"amount"`
		Currency    string `json:"currency"`
		Reference   string `json:"reference"`
	}

	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, "Invalid JSON", http.StatusBadRequest)
		return
	}

	// Parse amount
	amount, err := decimal.NewFromString(req.Amount)
	if err != nil {
		http.Error(w, "Invalid amount", http.StatusBadRequest)
		return
	}

	// Create payment
	payment := &types.Payment{
		PaymentID:    uuid.New(),
		DebtorBank:   req.FromAccount,
		CreditorBank: req.ToAccount,
		Amount:       amount,
		Currency:     req.Currency,
		Reference:    req.Reference,
		CreatedAt:    time.Now(),
		UpdatedAt:    time.Now(),
		Status:       types.PaymentStatusInitiated,
	}

	// Submit to queue
	if err := s.SubmitPayment(r.Context(), payment); err != nil {
		http.Error(w, err.Error(), http.StatusInternalServerError)
		return
	}

	// Add to recent transactions
	AddRecentTransaction(RecentTransaction{
		PaymentID: payment.PaymentID.String(),
		Amount:    amount.String(),
		Currency:  req.Currency,
		FromBank:  req.FromAccount,
		ToBank:    req.ToAccount,
		Status:    "initiated",
		Timestamp: time.Now(),
	})

	// Update metrics
	globalMetrics.mu.Lock()
	globalMetrics.TotalPayments++
	globalMetrics.mu.Unlock()

	// Return response
	resp := map[string]interface{}{
		"payment_id": payment.PaymentID.String(),
		"status":     "initiated",
		"created_at": payment.CreatedAt,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(resp)
}

// HandleGetPayment retrieves payment status
func (s *Server) HandleGetPayment(w http.ResponseWriter, r *http.Request) {
	// Extract payment ID from URL path
	paymentIDStr := r.URL.Path[len("/api/v1/payments/"):]

	paymentID, err := uuid.Parse(paymentIDStr)
	if err != nil {
		http.Error(w, "Invalid payment ID", http.StatusBadRequest)
		return
	}

	// Get payment from ledger
	payment, err := s.GetPaymentStatus(r.Context(), paymentID)
	if err != nil {
		http.Error(w, "Payment not found", http.StatusNotFound)
		return
	}

	resp := map[string]interface{}{
		"payment_id":     payment.PaymentID.String(),
		"status":         payment.Status,
		"amount":         payment.Amount.String(),
		"currency":       payment.Currency,
		"debtor_bank":    payment.DebtorBank,
		"creditor_bank":  payment.CreditorBank,
		"created_at":     payment.CreatedAt,
		"updated_at":     payment.UpdatedAt,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(resp)
}
