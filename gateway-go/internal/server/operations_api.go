package server

import (
	"encoding/json"
	"net/http"
	"time"

	"github.com/google/uuid"
)

// ========== PAYMENTS API ==========

type PaymentInitiateRequest struct {
	DebtorBank      string  `json:"debtor_bank"`
	CreditorBank    string  `json:"creditor_bank"`
	DebtorAccount   string  `json:"debtor_account"`
	CreditorAccount string  `json:"creditor_account"`
	DebtorName      string  `json:"debtor_name"`
	CreditorName    string  `json:"creditor_name"`
	Amount          float64 `json:"amount"`
	Currency        string  `json:"currency"`
	Reference       string  `json:"reference"`
}

type PaymentInitiateResponse struct {
	PaymentID string    `json:"payment_id"`
	UETR      string    `json:"uetr"`
	Status    string    `json:"status"`
	CreatedAt time.Time `json:"created_at"`
	Message   string    `json:"message"`
}

type PaymentStatusResponse struct {
	PaymentID       string    `json:"payment_id"`
	Status          string    `json:"status"`
	Amount          float64   `json:"amount"`
	Currency        string    `json:"currency"`
	DebtorBank      string    `json:"debtor_bank"`
	CreditorBank    string    `json:"creditor_bank"`
	CreatedAt       time.Time `json:"created_at"`
	UpdatedAt       time.Time `json:"updated_at"`
	EstimatedTime   string    `json:"estimated_completion_time"`
	CurrentStep     string    `json:"current_step"`
	Timeline        []StatusTimeline `json:"timeline"`
}

type StatusTimeline struct {
	Step      string    `json:"step"`
	Status    string    `json:"status"`
	Timestamp time.Time `json:"timestamp"`
}

type PaymentQuoteRequest struct {
	FromCurrency string  `json:"from_currency"`
	ToCurrency   string  `json:"to_currency"`
	Amount       float64 `json:"amount"`
	Corridor     string  `json:"corridor"`
}

type PaymentQuoteResponse struct {
	QuoteID      string    `json:"quote_id"`
	FromCurrency string    `json:"from_currency"`
	ToCurrency   string    `json:"to_currency"`
	FromAmount   float64   `json:"from_amount"`
	ToAmount     float64   `json:"to_amount"`
	ExchangeRate float64   `json:"exchange_rate"`
	Fee          float64   `json:"fee"`
	TotalCost    float64   `json:"total_cost"`
	ExpiresAt    time.Time `json:"expires_at"`
	Route        []string  `json:"route"`
}

type FeeCalculationRequest struct {
	Amount   float64 `json:"amount"`
	Currency string  `json:"currency"`
	Corridor string  `json:"corridor"`
	Priority string  `json:"priority"` // NORMAL, EXPRESS, INSTANT
}

type FeeCalculationResponse struct {
	BaseFee        float64 `json:"base_fee"`
	CorridorFee    float64 `json:"corridor_fee"`
	PriorityFee    float64 `json:"priority_fee"`
	NettingDiscount float64 `json:"netting_discount"`
	TotalFee       float64 `json:"total_fee"`
	Breakdown      []FeeBreakdown `json:"breakdown"`
}

type FeeBreakdown struct {
	Type        string  `json:"type"`
	Description string  `json:"description"`
	Amount      float64 `json:"amount"`
}

// HandlePaymentInitiate initiates a new payment
func (s *Server) HandlePaymentInitiate(w http.ResponseWriter, r *http.Request) {
	var req PaymentInitiateRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	paymentID := uuid.New().String()
	uetr := uuid.New().String()

	response := PaymentInitiateResponse{
		PaymentID: paymentID,
		UETR:      uetr,
		Status:    "INITIATED",
		CreatedAt: time.Now(),
		Message:   "Payment successfully initiated and queued for processing",
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandlePaymentStatus returns payment status with timeline
func (s *Server) HandlePaymentStatus(w http.ResponseWriter, r *http.Request) {
	paymentID := r.URL.Query().Get("id")
	if paymentID == "" {
		http.Error(w, "payment_id required", http.StatusBadRequest)
		return
	}

	now := time.Now()
	timeline := []StatusTimeline{
		{Step: "INITIATED", Status: "COMPLETED", Timestamp: now.Add(-5 * time.Minute)},
		{Step: "VALIDATED", Status: "COMPLETED", Timestamp: now.Add(-4 * time.Minute)},
		{Step: "SCREENED", Status: "COMPLETED", Timestamp: now.Add(-3 * time.Minute)},
		{Step: "APPROVED", Status: "COMPLETED", Timestamp: now.Add(-2 * time.Minute)},
		{Step: "QUEUED", Status: "IN_PROGRESS", Timestamp: now.Add(-1 * time.Minute)},
		{Step: "SETTLING", Status: "PENDING", Timestamp: time.Time{}},
		{Step: "SETTLED", Status: "PENDING", Timestamp: time.Time{}},
	}

	response := PaymentStatusResponse{
		PaymentID:     paymentID,
		Status:        "QUEUED",
		Amount:        50000.00,
		Currency:      "USD",
		DebtorBank:    "CHASUS33XXX",
		CreditorBank:  "DEUTDEFFXXX",
		CreatedAt:     now.Add(-5 * time.Minute),
		UpdatedAt:     now,
		EstimatedTime: "15 minutes",
		CurrentStep:   "Waiting for netting window",
		Timeline:      timeline,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandlePaymentQuote returns FX quote with route
func (s *Server) HandlePaymentQuote(w http.ResponseWriter, r *http.Request) {
	var req PaymentQuoteRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	rate := 1.0
	if req.FromCurrency != req.ToCurrency {
		rate = 0.92 // USD to EUR example
	}

	toAmount := req.Amount * rate
	fee := req.Amount * 0.001 // 0.1% fee

	response := PaymentQuoteResponse{
		QuoteID:      uuid.New().String(),
		FromCurrency: req.FromCurrency,
		ToCurrency:   req.ToCurrency,
		FromAmount:   req.Amount,
		ToAmount:     toAmount,
		ExchangeRate: rate,
		Fee:          fee,
		TotalCost:    req.Amount + fee,
		ExpiresAt:    time.Now().Add(5 * time.Minute),
		Route:        []string{"CHASUS33XXX", "DELTRAN_HUB", "DEUTDEFFXXX"},
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleFeeCalculation calculates fees with breakdown
func (s *Server) HandleFeeCalculation(w http.ResponseWriter, r *http.Request) {
	var req FeeCalculationRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	baseFee := req.Amount * 0.0005 // 0.05%
	corridorFee := req.Amount * 0.0003 // 0.03%
	priorityFee := 0.0
	if req.Priority == "EXPRESS" {
		priorityFee = req.Amount * 0.0002
	} else if req.Priority == "INSTANT" {
		priorityFee = req.Amount * 0.0005
	}

	nettingDiscount := (baseFee + corridorFee) * 0.15 // 15% discount for netting
	totalFee := baseFee + corridorFee + priorityFee - nettingDiscount

	breakdown := []FeeBreakdown{
		{Type: "BASE_FEE", Description: "Base transaction fee", Amount: baseFee},
		{Type: "CORRIDOR_FEE", Description: "Corridor-specific fee", Amount: corridorFee},
		{Type: "PRIORITY_FEE", Description: "Priority processing fee", Amount: priorityFee},
		{Type: "NETTING_DISCOUNT", Description: "Multilateral netting discount", Amount: -nettingDiscount},
	}

	response := FeeCalculationResponse{
		BaseFee:        baseFee,
		CorridorFee:    corridorFee,
		PriorityFee:    priorityFee,
		NettingDiscount: nettingDiscount,
		TotalFee:       totalFee,
		Breakdown:      breakdown,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandlePaymentCancel cancels a payment
func (s *Server) HandlePaymentCancel(w http.ResponseWriter, r *http.Request) {
	var req struct {
		PaymentID string `json:"payment_id"`
		Reason    string `json:"reason"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	response := map[string]interface{}{
		"payment_id": req.PaymentID,
		"status":     "CANCELLED",
		"message":    "Payment successfully cancelled",
		"timestamp":  time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandlePaymentsList returns list of recent payments
func (s *Server) HandlePaymentsList(w http.ResponseWriter, r *http.Request) {
	limit := r.URL.Query().Get("limit")
	if limit == "" {
		limit = "50"
	}

	payments := []map[string]interface{}{
		{
			"payment_id":     uuid.New().String(),
			"amount":         50000.00,
			"currency":       "USD",
			"debtor_bank":    "CHASUS33XXX",
			"creditor_bank":  "DEUTDEFFXXX",
			"status":         "SETTLING",
			"created_at":     time.Now().Add(-10 * time.Minute),
			"estimated_time": "5 minutes",
		},
		{
			"payment_id":     uuid.New().String(),
			"amount":         25000.00,
			"currency":       "EUR",
			"debtor_bank":    "DEUTDEFFXXX",
			"creditor_bank":  "SBININBBXXX",
			"status":         "SETTLED",
			"created_at":     time.Now().Add(-30 * time.Minute),
			"estimated_time": "0 minutes",
		},
	}

	response := map[string]interface{}{
		"payments":    payments,
		"total_count": 2,
		"page":        1,
		"limit":       limit,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}
