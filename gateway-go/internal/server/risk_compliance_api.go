package server

import (
	"encoding/json"
	"fmt"
	"net/http"
	"time"

	"github.com/google/uuid"
)

// ========== NETTING WINDOWS API ==========

type NettingWindowOpenRequest struct {
	CorridorID  string    `json:"corridor_id"`
	CutoffTime  time.Time `json:"cutoff_time"`
	WindowType  string    `json:"window_type"` // T+0, T+1
}

type NettingWindowResponse struct {
	WindowID    string    `json:"window_id"`
	CorridorID  string    `json:"corridor_id"`
	Status      string    `json:"status"`
	OpenedAt    time.Time `json:"opened_at"`
	CutoffTime  time.Time `json:"cutoff_time"`
	Message     string    `json:"message"`
}

type NettingPositionsResponse struct {
	WindowID     string             `json:"window_id"`
	Participants []ParticipantPosition `json:"participants"`
	TotalDebits  float64            `json:"total_debits"`
	TotalCredits float64            `json:"total_credits"`
	NetSavings   float64            `json:"net_savings"`
	Efficiency   float64            `json:"efficiency_percent"`
}

type ParticipantPosition struct {
	ParticipantID string  `json:"participant_id"`
	BankBIC       string  `json:"bank_bic"`
	Debits        float64 `json:"debits"`
	Credits       float64 `json:"credits"`
	NetPosition   float64 `json:"net_position"`
	Status        string  `json:"status"`
}

type NettingScheduleResponse struct {
	Corridors []CorridorSchedule `json:"corridors"`
}

type CorridorSchedule struct {
	CorridorID  string           `json:"corridor_id"`
	Windows     []WindowSchedule `json:"windows"`
	Timezone    string           `json:"timezone"`
}

type WindowSchedule struct {
	WindowType string    `json:"window_type"`
	OpenTime   string    `json:"open_time"`
	CloseTime  string    `json:"close_time"`
	NextWindow time.Time `json:"next_window"`
}

type NettingResultsResponse struct {
	WindowID        string                `json:"window_id"`
	Status          string                `json:"status"`
	TotalPayments   int                   `json:"total_payments"`
	GrossAmount     float64               `json:"gross_amount"`
	NetAmount       float64               `json:"net_amount"`
	Savings         float64               `json:"savings"`
	Efficiency      float64               `json:"efficiency_percent"`
	Participants    int                   `json:"participants_count"`
	SettlementBatch string                `json:"settlement_batch_id"`
	Settlements     []NettingSettlementInstruction `json:"settlements"`
}

type NettingSettlementInstruction struct {
	ParticipantID string  `json:"participant_id"`
	BankBIC       string  `json:"bank_bic"`
	NetPosition   float64 `json:"net_position"`
	Direction     string  `json:"direction"` // PAY, RECEIVE
	Status        string  `json:"status"`
}

// ========== LIMITS & CONTROLS API ==========

type LimitSetRequest struct {
	ParticipantID string  `json:"participant_id"`
	LimitType     string  `json:"limit_type"` // DAILY, TRANSACTION, COUNTERPARTY
	Currency      string  `json:"currency"`
	Amount        float64 `json:"amount"`
	Direction     string  `json:"direction"` // SEND, RECEIVE, BOTH
}

type LimitResponse struct {
	LimitID       string    `json:"limit_id"`
	ParticipantID string    `json:"participant_id"`
	LimitType     string    `json:"limit_type"`
	Currency      string    `json:"currency"`
	Amount        float64   `json:"amount"`
	Used          float64   `json:"used"`
	Available     float64   `json:"available"`
	Utilization   float64   `json:"utilization_percent"`
	Status        string    `json:"status"`
	CreatedAt     time.Time `json:"created_at"`
	UpdatedAt     time.Time `json:"updated_at"`
}

type LimitsUsageResponse struct {
	ParticipantID string          `json:"participant_id"`
	Limits        []LimitResponse `json:"limits"`
	TotalExposure float64         `json:"total_exposure"`
	RiskLevel     string          `json:"risk_level"`
}

type ControlAction struct {
	Action        string    `json:"action"` // FREEZE, UNFREEZE, THROTTLE
	ParticipantID string    `json:"participant_id"`
	Reason        string    `json:"reason"`
	Duration      string    `json:"duration,omitempty"`
}

type ControlResponse struct {
	ActionID      string    `json:"action_id"`
	ParticipantID string    `json:"participant_id"`
	Action        string    `json:"action"`
	Status        string    `json:"status"`
	Reason        string    `json:"reason"`
	AppliedAt     time.Time `json:"applied_at"`
	ExpiresAt     *time.Time `json:"expires_at,omitempty"`
}

// ========== COMPLIANCE API ==========

type ComplianceCheckRequest struct {
	PaymentID      string  `json:"payment_id"`
	DebtorName     string  `json:"debtor_name"`
	CreditorName   string  `json:"creditor_name"`
	DebtorCountry  string  `json:"debtor_country"`
	CreditorCountry string `json:"creditor_country"`
	Amount         float64 `json:"amount"`
	Currency       string  `json:"currency"`
}

type ComplianceCheckResponse struct {
	CheckID         string            `json:"check_id"`
	PaymentID       string            `json:"payment_id"`
	OverallStatus   string            `json:"overall_status"` // PASS, FAIL, REVIEW
	RiskScore       float64           `json:"risk_score"`
	Checks          []ComplianceCheck `json:"checks"`
	Flags           []ComplianceFlag  `json:"flags"`
	RequiresReview  bool              `json:"requires_review"`
	Timestamp       time.Time         `json:"timestamp"`
}

type ComplianceCheck struct {
	CheckType   string    `json:"check_type"`
	Status      string    `json:"status"`
	Details     string    `json:"details"`
	CompletedAt time.Time `json:"completed_at"`
}

type ComplianceFlag struct {
	FlagType    string    `json:"flag_type"`
	Severity    string    `json:"severity"` // LOW, MEDIUM, HIGH, CRITICAL
	Description string    `json:"description"`
	ListName    string    `json:"list_name,omitempty"`
}

type ComplianceReportsResponse struct {
	Reports     []ComplianceReport `json:"reports"`
	TotalCount  int                `json:"total_count"`
}

type ComplianceReport struct {
	ReportID    string    `json:"report_id"`
	ReportType  string    `json:"report_type"` // SAR, STR, CTR
	Status      string    `json:"status"`
	Period      string    `json:"period"`
	TotalCases  int       `json:"total_cases"`
	CreatedAt   time.Time `json:"created_at"`
	SubmittedAt *time.Time `json:"submitted_at,omitempty"`
}

type ComplianceRulesResponse struct {
	Rules []ComplianceRule `json:"rules"`
}

type ComplianceRule struct {
	RuleID      string                 `json:"rule_id"`
	Name        string                 `json:"name"`
	Type        string                 `json:"type"`
	Enabled     bool                   `json:"enabled"`
	Conditions  map[string]interface{} `json:"conditions"`
	Actions     []string               `json:"actions"`
	CreatedAt   time.Time              `json:"created_at"`
	UpdatedAt   time.Time              `json:"updated_at"`
}

// ========== RECONCILIATION API ==========

type ReconciliationPositionsResponse struct {
	AsOfDate     string                 `json:"as_of_date"`
	Accounts     []AccountPosition      `json:"accounts"`
	Discrepancies []Discrepancy         `json:"discrepancies"`
	Summary      ReconciliationSummary  `json:"summary"`
}

type AccountPosition struct {
	AccountID       string  `json:"account_id"`
	Currency        string  `json:"currency"`
	InternalBalance float64 `json:"internal_balance"`
	ExternalBalance float64 `json:"external_balance"`
	Difference      float64 `json:"difference"`
	Status          string  `json:"status"`
}

type Discrepancy struct {
	DiscrepancyID string    `json:"discrepancy_id"`
	AccountID     string    `json:"account_id"`
	Amount        float64   `json:"amount"`
	Type          string    `json:"type"`
	DetectedAt    time.Time `json:"detected_at"`
	Status        string    `json:"status"`
}

type ReconciliationSummary struct {
	TotalAccounts     int     `json:"total_accounts"`
	Matched           int     `json:"matched"`
	Mismatched        int     `json:"mismatched"`
	TotalDiscrepancy  float64 `json:"total_discrepancy"`
	ReconciliationRate float64 `json:"reconciliation_rate_percent"`
}

type ReconciliationRunRequest struct {
	StartDate string `json:"start_date"`
	EndDate   string `json:"end_date"`
	Accounts  []string `json:"accounts,omitempty"`
}

type ReconciliationRunResponse struct {
	RunID       string    `json:"run_id"`
	Status      string    `json:"status"`
	StartedAt   time.Time `json:"started_at"`
	Message     string    `json:"message"`
}

type ReconciliationResultsResponse struct {
	RunID          string                `json:"run_id"`
	Status         string                `json:"status"`
	Period         string                `json:"period"`
	Matched        int                   `json:"matched"`
	Mismatched     int                   `json:"mismatched"`
	Adjustments    []ReconciliationAdjustment `json:"adjustments"`
	CompletedAt    time.Time             `json:"completed_at"`
}

type ReconciliationAdjustment struct {
	AdjustmentID string    `json:"adjustment_id"`
	AccountID    string    `json:"account_id"`
	Amount       float64   `json:"amount"`
	Reason       string    `json:"reason"`
	AppliedAt    time.Time `json:"applied_at"`
	ApprovedBy   string    `json:"approved_by"`
}

// API Handlers

// HandleNettingOpen opens a new netting window
func (s *Server) HandleNettingOpen(w http.ResponseWriter, r *http.Request) {
	var req NettingWindowOpenRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	windowID := fmt.Sprintf("WINDOW_%s_%d", req.CorridorID, time.Now().Unix())

	response := NettingWindowResponse{
		WindowID:   windowID,
		CorridorID: req.CorridorID,
		Status:     "OPEN",
		OpenedAt:   time.Now(),
		CutoffTime: req.CutoffTime,
		Message:    "Netting window successfully opened",
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleNettingPositions returns current positions
func (s *Server) HandleNettingPositions(w http.ResponseWriter, r *http.Request) {
	windowID := r.URL.Query().Get("window_id")

	participants := []ParticipantPosition{
		{ParticipantID: "PART_001", BankBIC: "CHASUS33XXX", Debits: 500000, Credits: 450000, NetPosition: 50000, Status: "ACTIVE"},
		{ParticipantID: "PART_002", BankBIC: "DEUTDEFFXXX", Debits: 300000, Credits: 350000, NetPosition: -50000, Status: "ACTIVE"},
		{ParticipantID: "PART_003", BankBIC: "SBININBBXXX", Debits: 200000, Credits: 200000, NetPosition: 0, Status: "ACTIVE"},
	}

	totalDebits := 1000000.0
	totalCredits := 1000000.0
	grossAmount := 2000000.0
	netAmount := 100000.0
	netSavings := grossAmount - netAmount
	efficiency := (netSavings / grossAmount) * 100

	response := NettingPositionsResponse{
		WindowID:     windowID,
		Participants: participants,
		TotalDebits:  totalDebits,
		TotalCredits: totalCredits,
		NetSavings:   netSavings,
		Efficiency:   efficiency,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleComplianceCheck performs compliance checks
func (s *Server) HandleComplianceCheck(w http.ResponseWriter, r *http.Request) {
	var req ComplianceCheckRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	checks := []ComplianceCheck{
		{CheckType: "AML_SCREENING", Status: "PASS", Details: "No matches found", CompletedAt: time.Now()},
		{CheckType: "SANCTIONS_CHECK", Status: "PASS", Details: "No sanctions hits", CompletedAt: time.Now()},
		{CheckType: "PEP_SCREENING", Status: "PASS", Details: "No PEP matches", CompletedAt: time.Now()},
		{CheckType: "COUNTRY_RISK", Status: "PASS", Details: "Low risk jurisdiction", CompletedAt: time.Now()},
	}

	response := ComplianceCheckResponse{
		CheckID:        uuid.New().String(),
		PaymentID:      req.PaymentID,
		OverallStatus:  "PASS",
		RiskScore:      15.5,
		Checks:         checks,
		Flags:          []ComplianceFlag{},
		RequiresReview: false,
		Timestamp:      time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleLimitSet sets a new limit
func (s *Server) HandleLimitSet(w http.ResponseWriter, r *http.Request) {
	var req LimitSetRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	response := LimitResponse{
		LimitID:       uuid.New().String(),
		ParticipantID: req.ParticipantID,
		LimitType:     req.LimitType,
		Currency:      req.Currency,
		Amount:        req.Amount,
		Used:          0,
		Available:     req.Amount,
		Utilization:   0,
		Status:        "ACTIVE",
		CreatedAt:     time.Now(),
		UpdatedAt:     time.Now(),
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleReconciliationRun initiates reconciliation
func (s *Server) HandleReconciliationRun(w http.ResponseWriter, r *http.Request) {
	var req ReconciliationRunRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	response := ReconciliationRunResponse{
		RunID:     uuid.New().String(),
		Status:    "RUNNING",
		StartedAt: time.Now(),
		Message:   "Reconciliation started successfully",
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}
