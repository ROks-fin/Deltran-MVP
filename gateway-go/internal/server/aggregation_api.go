package server

import (
	"context"
	"database/sql"
	"encoding/csv"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"strings"
	"time"

	"github.com/go-chi/chi/v5"
	"github.com/redis/go-redis/v9"
	"github.com/rs/zerolog/log"
)

// AggregationAPI provides dashboard and reporting endpoints
type AggregationAPI struct {
	db    *sql.DB
	redis *redis.Client
	wsHub *WebSocketHub
}

// NewAggregationAPI creates a new aggregation API handler
func NewAggregationAPI(db *sql.DB, redis *redis.Client, wsHub *WebSocketHub) *AggregationAPI {
	return &AggregationAPI{
		db:    db,
		redis: redis,
		wsHub: wsHub,
	}
}

// RegisterRoutes registers all aggregation API routes
func (api *AggregationAPI) RegisterRoutes(r chi.Router) {
	r.Get("/api/v1/metrics/realtime", api.GetRealtimeMetrics)
	r.Get("/api/v1/payments", api.ListPayments)
	r.Get("/api/v1/payments/{id}", api.GetPaymentDetails)
	r.Get("/api/v1/export/payments", api.ExportPayments)
	r.Get("/api/v1/metrics/daily", api.GetDailyMetrics)
	r.Get("/api/v1/metrics/banks", api.GetBankMetrics)

	// WebSocket endpoint for live updates
	if api.wsHub != nil {
		r.Get("/api/v1/ws", api.wsHub.HandleWebSocket)
	}
}

// ============================================
// REAL-TIME METRICS
// ============================================

// RealtimeMetrics represents current system metrics
type RealtimeMetrics struct {
	TPS              float64            `json:"tps"`               // Transactions per second
	Volume24h        map[string]float64 `json:"volume_24h"`        // Total volume by currency in last 24h
	SuccessRate      float64            `json:"success_rate"`      // Percentage of successful payments
	PendingCount     int                `json:"pending_count"`     // Payments in pending status
	ProcessingCount  int                `json:"processing_count"`  // Payments in processing status
	QueueDepth       int                `json:"queue_depth"`       // Total pending + processing
	AverageAmount    map[string]float64 `json:"average_amount"`    // Average payment amount by currency
	FailedLast1h     int                `json:"failed_last_1h"`    // Failed payments in last hour
	SettledToday     int                `json:"settled_today"`     // Payments settled today
	ActiveBanks      int                `json:"active_banks"`      // Number of active participating banks
	ComplianceReview int                `json:"compliance_review"` // Payments requiring compliance review
	Timestamp        time.Time          `json:"timestamp"`
}

// GetRealtimeMetrics returns current system performance metrics
func (api *AggregationAPI) GetRealtimeMetrics(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	metrics := &RealtimeMetrics{
		Timestamp:    time.Now().UTC(),
		Volume24h:    make(map[string]float64),
		AverageAmount: make(map[string]float64),
	}

	// Calculate TPS (transactions per second in last minute)
	tps, err := api.calculateTPS(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to calculate TPS")
	} else {
		metrics.TPS = tps
	}

	// Get 24h volume by currency
	volume24h, err := api.get24hVolume(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get 24h volume")
	} else {
		metrics.Volume24h = volume24h
	}

	// Get success rate (last 24 hours)
	successRate, err := api.getSuccessRate(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get success rate")
	} else {
		metrics.SuccessRate = successRate
	}

	// Get queue depth
	queueDepth, err := api.getQueueDepth(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get queue depth")
	} else {
		metrics.PendingCount = queueDepth["pending"]
		metrics.ProcessingCount = queueDepth["processing"]
		metrics.QueueDepth = queueDepth["pending"] + queueDepth["processing"]
	}

	// Get average amounts by currency
	avgAmounts, err := api.getAverageAmounts(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get average amounts")
	} else {
		metrics.AverageAmount = avgAmounts
	}

	// Get failed payments in last hour
	failedCount, err := api.getFailedLast1h(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get failed count")
	} else {
		metrics.FailedLast1h = failedCount
	}

	// Get settled today
	settledToday, err := api.getSettledToday(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get settled today")
	} else {
		metrics.SettledToday = settledToday
	}

	// Get active banks
	activeBanks, err := api.getActiveBanks(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get active banks")
	} else {
		metrics.ActiveBanks = activeBanks
	}

	// Get compliance review count
	complianceReview, err := api.getComplianceReviewCount(ctx)
	if err != nil {
		log.Error().Err(err).Msg("Failed to get compliance review count")
	} else {
		metrics.ComplianceReview = complianceReview
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(metrics)
}

// ============================================
// PAYMENT LISTING WITH FILTERS
// ============================================

// PaymentListItem represents a payment in the list view
type PaymentListItem struct {
	ID              string    `json:"id"`
	PaymentReference string    `json:"payment_reference"`
	SenderBIC       string    `json:"sender_bic"`
	SenderName      string    `json:"sender_name"`
	ReceiverBIC     string    `json:"receiver_bic"`
	ReceiverName    string    `json:"receiver_name"`
	Amount          float64   `json:"amount"`
	Currency        string    `json:"currency"`
	Status          string    `json:"status"`
	RiskScore       *float64  `json:"risk_score,omitempty"`
	CreatedAt       time.Time `json:"created_at"`
	SettledAt       *time.Time `json:"settled_at,omitempty"`
}

// PaymentListResponse represents paginated payment list
type PaymentListResponse struct {
	Payments   []PaymentListItem `json:"payments"`
	Total      int               `json:"total"`
	Page       int               `json:"page"`
	PageSize   int               `json:"page_size"`
	TotalPages int               `json:"total_pages"`
}

// ListPayments returns filtered and paginated payment list
func (api *AggregationAPI) ListPayments(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	// Parse query parameters
	query := r.URL.Query()

	// Pagination
	page, _ := strconv.Atoi(query.Get("page"))
	if page < 1 {
		page = 1
	}
	pageSize, _ := strconv.Atoi(query.Get("page_size"))
	if pageSize < 1 || pageSize > 100 {
		pageSize = 20
	}
	offset := (page - 1) * pageSize

	// Filters
	status := query.Get("status")
	currency := query.Get("currency")
	senderBIC := query.Get("sender_bic")
	receiverBIC := query.Get("receiver_bic")
	dateFrom := query.Get("date_from")
	dateTo := query.Get("date_to")
	minAmount := query.Get("min_amount")
	maxAmount := query.Get("max_amount")
	searchRef := query.Get("reference")

	// Build WHERE clause
	whereClauses := []string{}
	args := []interface{}{}
	argIdx := 1

	if status != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.status = $%d", argIdx))
		args = append(args, status)
		argIdx++
	}

	if currency != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.currency = $%d", argIdx))
		args = append(args, currency)
		argIdx++
	}

	if senderBIC != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("sb.bic_code = $%d", argIdx))
		args = append(args, senderBIC)
		argIdx++
	}

	if receiverBIC != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("rb.bic_code = $%d", argIdx))
		args = append(args, receiverBIC)
		argIdx++
	}

	if dateFrom != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.created_at >= $%d", argIdx))
		args = append(args, dateFrom)
		argIdx++
	}

	if dateTo != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.created_at <= $%d", argIdx))
		args = append(args, dateTo)
		argIdx++
	}

	if minAmount != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.amount >= $%d", argIdx))
		args = append(args, minAmount)
		argIdx++
	}

	if maxAmount != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.amount <= $%d", argIdx))
		args = append(args, maxAmount)
		argIdx++
	}

	if searchRef != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.payment_reference ILIKE $%d", argIdx))
		args = append(args, "%"+searchRef+"%")
		argIdx++
	}

	whereClause := ""
	if len(whereClauses) > 0 {
		whereClause = "WHERE " + strings.Join(whereClauses, " AND ")
	}

	// Get total count
	countQuery := fmt.Sprintf(`
		SELECT COUNT(*)
		FROM deltran.payments p
		JOIN deltran.banks sb ON p.sender_bank_id = sb.id
		JOIN deltran.banks rb ON p.receiver_bank_id = rb.id
		%s
	`, whereClause)

	var total int
	err := api.db.QueryRowContext(ctx, countQuery, args...).Scan(&total)
	if err != nil {
		log.Error().Err(err).Msg("Failed to count payments")
		http.Error(w, "Failed to fetch payments", http.StatusInternalServerError)
		return
	}

	// Get paginated payments
	args = append(args, pageSize, offset)
	listQuery := fmt.Sprintf(`
		SELECT
			p.id,
			p.payment_reference,
			sb.bic_code,
			sb.name,
			rb.bic_code,
			rb.name,
			p.amount,
			p.currency,
			p.status,
			p.risk_score,
			p.created_at,
			p.settled_at
		FROM deltran.payments p
		JOIN deltran.banks sb ON p.sender_bank_id = sb.id
		JOIN deltran.banks rb ON p.receiver_bank_id = rb.id
		%s
		ORDER BY p.created_at DESC
		LIMIT $%d OFFSET $%d
	`, whereClause, argIdx, argIdx+1)

	rows, err := api.db.QueryContext(ctx, listQuery, args...)
	if err != nil {
		log.Error().Err(err).Msg("Failed to query payments")
		http.Error(w, "Failed to fetch payments", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	payments := []PaymentListItem{}
	for rows.Next() {
		var p PaymentListItem
		err := rows.Scan(
			&p.ID, &p.PaymentReference,
			&p.SenderBIC, &p.SenderName,
			&p.ReceiverBIC, &p.ReceiverName,
			&p.Amount, &p.Currency, &p.Status,
			&p.RiskScore, &p.CreatedAt, &p.SettledAt,
		)
		if err != nil {
			log.Error().Err(err).Msg("Failed to scan payment row")
			continue
		}
		payments = append(payments, p)
	}

	totalPages := (total + pageSize - 1) / pageSize

	response := PaymentListResponse{
		Payments:   payments,
		Total:      total,
		Page:       page,
		PageSize:   pageSize,
		TotalPages: totalPages,
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(response)
}

// ============================================
// PAYMENT DETAILS
// ============================================

// PaymentDetails represents full payment information
type PaymentDetails struct {
	PaymentListItem
	SenderAccountID   *string   `json:"sender_account_id,omitempty"`
	ReceiverAccountID *string   `json:"receiver_account_id,omitempty"`
	BatchID           *string   `json:"batch_id,omitempty"`
	SWIFTMessageType  *string   `json:"swift_message_type,omitempty"`
	SWIFTMessageID    *string   `json:"swift_message_id,omitempty"`
	IdempotencyKey    *string   `json:"idempotency_key,omitempty"`
	ProcessedAt       *time.Time `json:"processed_at,omitempty"`
	RemittanceInfo    *string   `json:"remittance_info,omitempty"`
	ComplianceCheck   *ComplianceCheckInfo `json:"compliance_check,omitempty"`
	UpdatedAt         time.Time `json:"updated_at"`
}

// ComplianceCheckInfo represents compliance check details
type ComplianceCheckInfo struct {
	ID             string    `json:"id"`
	CheckType      string    `json:"check_type"`
	Status         string    `json:"status"`
	RiskScore      *float64  `json:"risk_score,omitempty"`
	RequiresReview bool      `json:"requires_review"`
	CompletedAt    *time.Time `json:"completed_at,omitempty"`
}

// GetPaymentDetails returns full details for a specific payment
func (api *AggregationAPI) GetPaymentDetails(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()
	paymentID := chi.URLParam(r, "id")

	query := `
		SELECT
			p.id,
			p.payment_reference,
			sb.bic_code,
			sb.name,
			rb.bic_code,
			rb.name,
			p.amount,
			p.currency,
			p.status,
			p.risk_score,
			p.created_at,
			p.settled_at,
			p.sender_account_id,
			p.receiver_account_id,
			p.batch_id,
			p.swift_message_type,
			p.swift_message_id,
			p.idempotency_key,
			p.processed_at,
			p.remittance_info,
			p.updated_at,
			p.compliance_check_id
		FROM deltran.payments p
		JOIN deltran.banks sb ON p.sender_bank_id = sb.id
		JOIN deltran.banks rb ON p.receiver_bank_id = rb.id
		WHERE p.id = $1 OR p.payment_reference = $1
	`

	var payment PaymentDetails
	var complianceCheckID *string

	err := api.db.QueryRowContext(ctx, query, paymentID).Scan(
		&payment.ID, &payment.PaymentReference,
		&payment.SenderBIC, &payment.SenderName,
		&payment.ReceiverBIC, &payment.ReceiverName,
		&payment.Amount, &payment.Currency, &payment.Status,
		&payment.RiskScore, &payment.CreatedAt, &payment.SettledAt,
		&payment.SenderAccountID, &payment.ReceiverAccountID,
		&payment.BatchID, &payment.SWIFTMessageType, &payment.SWIFTMessageID,
		&payment.IdempotencyKey, &payment.ProcessedAt, &payment.RemittanceInfo,
		&payment.UpdatedAt, &complianceCheckID,
	)

	if err == sql.ErrNoRows {
		http.Error(w, "Payment not found", http.StatusNotFound)
		return
	} else if err != nil {
		log.Error().Err(err).Msg("Failed to query payment details")
		http.Error(w, "Failed to fetch payment details", http.StatusInternalServerError)
		return
	}

	// Get compliance check details if exists
	if complianceCheckID != nil {
		complianceQuery := `
			SELECT id, check_type, status, risk_score, requires_review, completed_at
			FROM deltran.compliance_checks
			WHERE id = $1
		`
		var cc ComplianceCheckInfo
		err := api.db.QueryRowContext(ctx, complianceQuery, *complianceCheckID).Scan(
			&cc.ID, &cc.CheckType, &cc.Status, &cc.RiskScore, &cc.RequiresReview, &cc.CompletedAt,
		)
		if err == nil {
			payment.ComplianceCheck = &cc
		}
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(payment)
}

// ============================================
// EXPORT FUNCTIONALITY
// ============================================

// ExportPayments exports payments to CSV format
func (api *AggregationAPI) ExportPayments(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	// Use same filters as ListPayments
	query := r.URL.Query()
	status := query.Get("status")
	currency := query.Get("currency")
	senderBIC := query.Get("sender_bic")
	receiverBIC := query.Get("receiver_bic")
	dateFrom := query.Get("date_from")
	dateTo := query.Get("date_to")

	// Build WHERE clause
	whereClauses := []string{}
	args := []interface{}{}
	argIdx := 1

	if status != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.status = $%d", argIdx))
		args = append(args, status)
		argIdx++
	}
	if currency != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.currency = $%d", argIdx))
		args = append(args, currency)
		argIdx++
	}
	if senderBIC != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("sb.bic_code = $%d", argIdx))
		args = append(args, senderBIC)
		argIdx++
	}
	if receiverBIC != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("rb.bic_code = $%d", argIdx))
		args = append(args, receiverBIC)
		argIdx++
	}
	if dateFrom != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.created_at >= $%d", argIdx))
		args = append(args, dateFrom)
		argIdx++
	}
	if dateTo != "" {
		whereClauses = append(whereClauses, fmt.Sprintf("p.created_at <= $%d", argIdx))
		args = append(args, dateTo)
		argIdx++
	}

	whereClause := ""
	if len(whereClauses) > 0 {
		whereClause = "WHERE " + strings.Join(whereClauses, " AND ")
	}

	exportQuery := fmt.Sprintf(`
		SELECT
			p.id,
			p.payment_reference,
			sb.bic_code,
			sb.name,
			rb.bic_code,
			rb.name,
			p.amount,
			p.currency,
			p.status,
			COALESCE(p.risk_score, 0),
			p.created_at,
			COALESCE(p.settled_at, '1970-01-01'::timestamptz)
		FROM deltran.payments p
		JOIN deltran.banks sb ON p.sender_bank_id = sb.id
		JOIN deltran.banks rb ON p.receiver_bank_id = rb.id
		%s
		ORDER BY p.created_at DESC
		LIMIT 10000
	`, whereClause)

	rows, err := api.db.QueryContext(ctx, exportQuery, args...)
	if err != nil {
		log.Error().Err(err).Msg("Failed to query payments for export")
		http.Error(w, "Failed to export payments", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	// Set CSV headers
	w.Header().Set("Content-Type", "text/csv")
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=payments_%s.csv", time.Now().Format("20060102_150405")))

	writer := csv.NewWriter(w)
	defer writer.Flush()

	// Write CSV header
	writer.Write([]string{
		"ID", "Reference", "Sender BIC", "Sender Name", "Receiver BIC", "Receiver Name",
		"Amount", "Currency", "Status", "Risk Score", "Created At", "Settled At",
	})

	// Write data rows
	for rows.Next() {
		var (
			id, ref, senderBIC, senderName, receiverBIC, receiverName, currency, status string
			amount, riskScore float64
			createdAt, settledAt time.Time
		)

		err := rows.Scan(&id, &ref, &senderBIC, &senderName, &receiverBIC, &receiverName,
			&amount, &currency, &status, &riskScore, &createdAt, &settledAt)
		if err != nil {
			log.Error().Err(err).Msg("Failed to scan payment row for export")
			continue
		}

		settledAtStr := ""
		if !settledAt.IsZero() && settledAt.Year() > 1970 {
			settledAtStr = settledAt.Format(time.RFC3339)
		}

		writer.Write([]string{
			id, ref, senderBIC, senderName, receiverBIC, receiverName,
			fmt.Sprintf("%.2f", amount), currency, status, fmt.Sprintf("%.2f", riskScore),
			createdAt.Format(time.RFC3339), settledAtStr,
		})
	}
}

// ============================================
// DAILY METRICS
// ============================================

// DailyMetric represents daily aggregated metrics
type DailyMetric struct {
	Date          string             `json:"date"`
	PaymentCount  int                `json:"payment_count"`
	TotalVolume   map[string]float64 `json:"total_volume"`
	AverageAmount map[string]float64 `json:"average_amount"`
	StatusBreakdown map[string]int   `json:"status_breakdown"`
}

// GetDailyMetrics returns daily aggregated metrics for the last N days
func (api *AggregationAPI) GetDailyMetrics(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	// Parse days parameter (default 7 days)
	daysStr := r.URL.Query().Get("days")
	days, _ := strconv.Atoi(daysStr)
	if days < 1 || days > 90 {
		days = 7
	}

	query := `
		SELECT
			DATE(created_at) as date,
			currency,
			status,
			COUNT(*) as count,
			SUM(amount) as total_amount,
			AVG(amount) as avg_amount
		FROM deltran.payments
		WHERE created_at >= NOW() - INTERVAL '%d days'
		GROUP BY DATE(created_at), currency, status
		ORDER BY date DESC, currency, status
	`

	rows, err := api.db.QueryContext(ctx, fmt.Sprintf(query, days))
	if err != nil {
		log.Error().Err(err).Msg("Failed to query daily metrics")
		http.Error(w, "Failed to fetch daily metrics", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	// Aggregate by date
	metricsMap := make(map[string]*DailyMetric)

	for rows.Next() {
		var date, currency, status string
		var count int
		var totalAmount, avgAmount float64

		err := rows.Scan(&date, &currency, &status, &count, &totalAmount, &avgAmount)
		if err != nil {
			continue
		}

		if metricsMap[date] == nil {
			metricsMap[date] = &DailyMetric{
				Date:            date,
				TotalVolume:     make(map[string]float64),
				AverageAmount:   make(map[string]float64),
				StatusBreakdown: make(map[string]int),
			}
		}

		metric := metricsMap[date]
		metric.PaymentCount += count
		metric.TotalVolume[currency] += totalAmount
		metric.AverageAmount[currency] = avgAmount
		metric.StatusBreakdown[status] += count
	}

	// Convert map to slice
	dailyMetrics := []DailyMetric{}
	for _, metric := range metricsMap {
		dailyMetrics = append(dailyMetrics, *metric)
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(dailyMetrics)
}

// ============================================
// BANK METRICS
// ============================================

// BankMetric represents per-bank aggregated metrics
type BankMetric struct {
	BankID      string             `json:"bank_id"`
	BIC         string             `json:"bic"`
	BankName    string             `json:"bank_name"`
	Sent        int                `json:"sent"`
	Received    int                `json:"received"`
	SentVolume  map[string]float64 `json:"sent_volume"`
	RecvVolume  map[string]float64 `json:"received_volume"`
}

// GetBankMetrics returns per-bank activity metrics
func (api *AggregationAPI) GetBankMetrics(w http.ResponseWriter, r *http.Request) {
	ctx := r.Context()

	// Get sent metrics
	sentQuery := `
		SELECT
			b.id,
			b.bic_code,
			b.name,
			p.currency,
			COUNT(*) as count,
			SUM(p.amount) as total_amount
		FROM deltran.banks b
		JOIN deltran.payments p ON p.sender_bank_id = b.id
		WHERE p.created_at >= NOW() - INTERVAL '24 hours'
		GROUP BY b.id, b.bic_code, b.name, p.currency
	`

	rows, err := api.db.QueryContext(ctx, sentQuery)
	if err != nil {
		log.Error().Err(err).Msg("Failed to query bank sent metrics")
		http.Error(w, "Failed to fetch bank metrics", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	metricsMap := make(map[string]*BankMetric)

	for rows.Next() {
		var bankID, bic, name, currency string
		var count int
		var totalAmount float64

		err := rows.Scan(&bankID, &bic, &name, &currency, &count, &totalAmount)
		if err != nil {
			continue
		}

		if metricsMap[bankID] == nil {
			metricsMap[bankID] = &BankMetric{
				BankID:     bankID,
				BIC:        bic,
				BankName:   name,
				SentVolume: make(map[string]float64),
				RecvVolume: make(map[string]float64),
			}
		}

		metric := metricsMap[bankID]
		metric.Sent += count
		metric.SentVolume[currency] += totalAmount
	}

	rows.Close()

	// Get received metrics
	recvQuery := `
		SELECT
			b.id,
			b.bic_code,
			b.name,
			p.currency,
			COUNT(*) as count,
			SUM(p.amount) as total_amount
		FROM deltran.banks b
		JOIN deltran.payments p ON p.receiver_bank_id = b.id
		WHERE p.created_at >= NOW() - INTERVAL '24 hours'
		GROUP BY b.id, b.bic_code, b.name, p.currency
	`

	rows, err = api.db.QueryContext(ctx, recvQuery)
	if err != nil {
		log.Error().Err(err).Msg("Failed to query bank received metrics")
		http.Error(w, "Failed to fetch bank metrics", http.StatusInternalServerError)
		return
	}
	defer rows.Close()

	for rows.Next() {
		var bankID, bic, name, currency string
		var count int
		var totalAmount float64

		err := rows.Scan(&bankID, &bic, &name, &currency, &count, &totalAmount)
		if err != nil {
			continue
		}

		if metricsMap[bankID] == nil {
			metricsMap[bankID] = &BankMetric{
				BankID:     bankID,
				BIC:        bic,
				BankName:   name,
				SentVolume: make(map[string]float64),
				RecvVolume: make(map[string]float64),
			}
		}

		metric := metricsMap[bankID]
		metric.Received += count
		metric.RecvVolume[currency] += totalAmount
	}

	// Convert map to slice
	bankMetrics := []BankMetric{}
	for _, metric := range metricsMap {
		bankMetrics = append(bankMetrics, *metric)
	}

	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(http.StatusOK)
	json.NewEncoder(w).Encode(bankMetrics)
}

// ============================================
// HELPER FUNCTIONS
// ============================================

func (api *AggregationAPI) calculateTPS(ctx context.Context) (float64, error) {
	query := `
		SELECT COUNT(*)
		FROM deltran.payments
		WHERE created_at >= NOW() - INTERVAL '1 minute'
	`
	var count int
	err := api.db.QueryRowContext(ctx, query).Scan(&count)
	if err != nil {
		return 0, err
	}
	return float64(count) / 60.0, nil
}

func (api *AggregationAPI) get24hVolume(ctx context.Context) (map[string]float64, error) {
	query := `
		SELECT currency, SUM(amount)
		FROM deltran.payments
		WHERE created_at >= NOW() - INTERVAL '24 hours'
		GROUP BY currency
	`
	rows, err := api.db.QueryContext(ctx, query)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	result := make(map[string]float64)
	for rows.Next() {
		var currency string
		var amount float64
		if err := rows.Scan(&currency, &amount); err == nil {
			result[currency] = amount
		}
	}
	return result, nil
}

func (api *AggregationAPI) getSuccessRate(ctx context.Context) (float64, error) {
	query := `
		SELECT
			COUNT(*) FILTER (WHERE status = 'settled') as settled,
			COUNT(*) as total
		FROM deltran.payments
		WHERE created_at >= NOW() - INTERVAL '24 hours'
	`
	var settled, total int
	err := api.db.QueryRowContext(ctx, query).Scan(&settled, &total)
	if err != nil || total == 0 {
		return 0, err
	}
	return float64(settled) / float64(total) * 100.0, nil
}

func (api *AggregationAPI) getQueueDepth(ctx context.Context) (map[string]int, error) {
	query := `
		SELECT status, COUNT(*)
		FROM deltran.payments
		WHERE status IN ('pending', 'processing')
		GROUP BY status
	`
	rows, err := api.db.QueryContext(ctx, query)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	result := map[string]int{"pending": 0, "processing": 0}
	for rows.Next() {
		var status string
		var count int
		if err := rows.Scan(&status, &count); err == nil {
			result[status] = count
		}
	}
	return result, nil
}

func (api *AggregationAPI) getAverageAmounts(ctx context.Context) (map[string]float64, error) {
	query := `
		SELECT currency, AVG(amount)
		FROM deltran.payments
		WHERE created_at >= NOW() - INTERVAL '24 hours'
		GROUP BY currency
	`
	rows, err := api.db.QueryContext(ctx, query)
	if err != nil {
		return nil, err
	}
	defer rows.Close()

	result := make(map[string]float64)
	for rows.Next() {
		var currency string
		var avgAmount float64
		if err := rows.Scan(&currency, &avgAmount); err == nil {
			result[currency] = avgAmount
		}
	}
	return result, nil
}

func (api *AggregationAPI) getFailedLast1h(ctx context.Context) (int, error) {
	query := `
		SELECT COUNT(*)
		FROM deltran.payments
		WHERE status = 'failed' AND created_at >= NOW() - INTERVAL '1 hour'
	`
	var count int
	err := api.db.QueryRowContext(ctx, query).Scan(&count)
	return count, err
}

func (api *AggregationAPI) getSettledToday(ctx context.Context) (int, error) {
	query := `
		SELECT COUNT(*)
		FROM deltran.payments
		WHERE status = 'settled' AND DATE(settled_at) = CURRENT_DATE
	`
	var count int
	err := api.db.QueryRowContext(ctx, query).Scan(&count)
	return count, err
}

func (api *AggregationAPI) getActiveBanks(ctx context.Context) (int, error) {
	query := `SELECT COUNT(*) FROM deltran.banks WHERE is_active = true`
	var count int
	err := api.db.QueryRowContext(ctx, query).Scan(&count)
	return count, err
}

func (api *AggregationAPI) getComplianceReviewCount(ctx context.Context) (int, error) {
	query := `
		SELECT COUNT(*)
		FROM deltran.compliance_checks
		WHERE requires_review = true AND status = 'review'
	`
	var count int
	err := api.db.QueryRowContext(ctx, query).Scan(&count)
	return count, err
}
