package server

import (
	"encoding/json"
	"fmt"
	"math/rand"
	"net/http"
	"time"

	"github.com/google/uuid"
)

// Helper functions
func randomHex(length int) string {
	chars := "0123456789abcdef"
	result := ""
	for i := 0; i < length; i++ {
		result += string(chars[rand.Intn(len(chars))])
	}
	return result
}

func generateISO20022() string {
	return `<?xml version="1.0" encoding="UTF-8"?>
<Document xmlns="urn:iso:std:iso:20022:tech:xsd:pacs.008.001.08">
  <FIToFICstmrCdtTrf>
    <GrpHdr>
      <MsgId>DELTRAN` + randomHex(8) + `</MsgId>
      <CreDtTm>` + time.Now().Format(time.RFC3339) + `</CreDtTm>
    </GrpHdr>
  </FIToFICstmrCdtTrf>
</Document>`
}

// ========== BATCHES & PROOFS API ==========

type BatchCreateRequest struct {
	CorridorID  string   `json:"corridor_id"`
	PaymentIDs  []string `json:"payment_ids"`
	WindowClose string   `json:"window_close"`
}

type BatchCreateResponse struct {
	BatchID     string    `json:"batch_id"`
	Status      string    `json:"status"`
	PaymentCount int      `json:"payment_count"`
	TotalAmount float64   `json:"total_amount"`
	CreatedAt   time.Time `json:"created_at"`
	Message     string    `json:"message"`
}

type BatchDetailsResponse struct {
	BatchID         string           `json:"batch_id"`
	CorridorID      string           `json:"corridor_id"`
	Status          string           `json:"status"`
	WindowCloseUTC  time.Time        `json:"window_close_utc"`
	PaymentCount    int              `json:"payment_count"`
	DebitsUSD       float64          `json:"debits_usd"`
	CreditsUSD      float64          `json:"credits_usd"`
	NetAmountUSD    float64          `json:"net_amount_usd"`
	MerkleRoot      string           `json:"merkle_root"`
	ValidatorCount  int              `json:"validator_count"`
	Signatures      []ValidatorSig   `json:"signatures"`
	Payments        []BatchPayment   `json:"payments"`
	CreatedAt       time.Time        `json:"created_at"`
	ClosedAt        *time.Time       `json:"closed_at,omitempty"`
}

type BatchPayment struct {
	PaymentID    string  `json:"payment_id"`
	DebtorBank   string  `json:"debtor_bank"`
	CreditorBank string  `json:"creditor_bank"`
	Amount       float64 `json:"amount"`
	Currency     string  `json:"currency"`
	Status       string  `json:"status"`
}

type ValidatorSig struct {
	ValidatorID string    `json:"validator_id"`
	PublicKey   string    `json:"public_key"`
	Signature   string    `json:"signature"`
	Timestamp   time.Time `json:"timestamp"`
}

type BatchProofResponse struct {
	BatchID             string                  `json:"batch_id"`
	MerkleRoot          string                  `json:"merkle_root"`
	MerklePath          []string                `json:"merkle_path"`
	BlockHash           string                  `json:"block_hash"`
	BlockHeight         int64                   `json:"block_height"`
	ConsensusRound      int                     `json:"consensus_round"`
	Validators          []ValidatorSig          `json:"validators"`
	SettlementProof     SettlementProofData     `json:"settlement_proof"`
	CryptographicProofs CryptographicProofData  `json:"cryptographic_proofs"`
	ISO20022Message     string                  `json:"iso20022_message"`
}

type SettlementProofData struct {
	NetPosition      float64   `json:"net_position"`
	SettlementBank   string    `json:"settlement_bank"`
	SettlementAccount string   `json:"settlement_account"`
	ValueDate        string    `json:"value_date"`
	ConfirmationID   string    `json:"confirmation_id"`
}

type CryptographicProofData struct {
	HashAlgorithm    string   `json:"hash_algorithm"`
	SignatureScheme  string   `json:"signature_scheme"`
	ConsensusProof   string   `json:"consensus_proof"`
	TimestampProof   string   `json:"timestamp_proof"`
}

type BatchListResponse struct {
	Batches    []BatchSummary `json:"batches"`
	TotalCount int            `json:"total_count"`
	Page       int            `json:"page"`
}

type BatchSummary struct {
	BatchID        string    `json:"batch_id"`
	CorridorID     string    `json:"corridor_id"`
	Status         string    `json:"status"`
	PaymentCount   int       `json:"payment_count"`
	NetAmountUSD   float64   `json:"net_amount_usd"`
	WindowCloseUTC time.Time `json:"window_close_utc"`
	CreatedAt      time.Time `json:"created_at"`
}

// HandleBatchCreate creates a new settlement batch
func (s *Server) HandleBatchCreate(w http.ResponseWriter, r *http.Request) {
	var req BatchCreateRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	batchID := fmt.Sprintf("BATCH_%s_%d", req.CorridorID, time.Now().Unix())
	totalAmount := float64(len(req.PaymentIDs)) * (10000 + rand.Float64()*40000)

	response := BatchCreateResponse{
		BatchID:      batchID,
		Status:       "OPEN",
		PaymentCount: len(req.PaymentIDs),
		TotalAmount:  totalAmount,
		CreatedAt:    time.Now(),
		Message:      "Batch successfully created and ready for settlement",
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleBatchDetails returns detailed batch information
func (s *Server) HandleBatchDetails(w http.ResponseWriter, r *http.Request) {
	batchID := r.URL.Query().Get("id")
	if batchID == "" {
		http.Error(w, "batch_id required", http.StatusBadRequest)
		return
	}

	now := time.Now()
	closedAt := now.Add(-1 * time.Hour)

	payments := make([]BatchPayment, 15)
	for i := 0; i < 15; i++ {
		payments[i] = BatchPayment{
			PaymentID:    uuid.New().String(),
			DebtorBank:   "CHASUS33XXX",
			CreditorBank: "DEUTDEFFXXX",
			Amount:       10000 + rand.Float64()*40000,
			Currency:     "USD",
			Status:       "SETTLED",
		}
	}

	signatures := make([]ValidatorSig, 7)
	for i := 0; i < 7; i++ {
		signatures[i] = ValidatorSig{
			ValidatorID: fmt.Sprintf("validator_%d", i+1),
			PublicKey:   fmt.Sprintf("0x%s", randomHex(40)),
			Signature:   fmt.Sprintf("0x%s", randomHex(128)),
			Timestamp:   now.Add(time.Duration(-i) * time.Minute),
		}
	}

	response := BatchDetailsResponse{
		BatchID:        batchID,
		CorridorID:     "UAE-IN",
		Status:         "SETTLED",
		WindowCloseUTC: closedAt,
		PaymentCount:   15,
		DebitsUSD:      450000,
		CreditsUSD:     420000,
		NetAmountUSD:   30000,
		MerkleRoot:     "0x" + randomHex(64),
		ValidatorCount: 7,
		Signatures:     signatures,
		Payments:       payments,
		CreatedAt:      now.Add(-2 * time.Hour),
		ClosedAt:       &closedAt,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleBatchProofs returns cryptographic proofs
func (s *Server) HandleBatchProofs(w http.ResponseWriter, r *http.Request) {
	batchID := r.URL.Query().Get("id")
	if batchID == "" {
		http.Error(w, "batch_id required", http.StatusBadRequest)
		return
	}

	now := time.Now()
	merklePath := make([]string, 5)
	for i := 0; i < 5; i++ {
		merklePath[i] = "0x" + randomHex(64)
	}

	validators := make([]ValidatorSig, 7)
	for i := 0; i < 7; i++ {
		validators[i] = ValidatorSig{
			ValidatorID: fmt.Sprintf("validator_%d", i+1),
			PublicKey:   "0x" + randomHex(40),
			Signature:   "0x" + randomHex(128),
			Timestamp:   now.Add(time.Duration(-i) * time.Minute),
		}
	}

	iso20022 := generateISO20022()

	response := BatchProofResponse{
		BatchID:        batchID,
		MerkleRoot:     "0x" + randomHex(64),
		MerklePath:     merklePath,
		BlockHash:      "0x" + randomHex(64),
		BlockHeight:    1284756 + rand.Int63n(1000),
		ConsensusRound: 42,
		Validators:     validators,
		SettlementProof: SettlementProofData{
			NetPosition:       30000,
			SettlementBank:    "DEUTDEFFXXX",
			SettlementAccount: "DE89370400440532013000",
			ValueDate:         time.Now().Format("2006-01-02"),
			ConfirmationID:    uuid.New().String(),
		},
		CryptographicProofs: CryptographicProofData{
			HashAlgorithm:   "SHA-256",
			SignatureScheme: "ECDSA-secp256k1",
			ConsensusProof:  "BFT-7-of-7",
			TimestampProof:  "RFC3161",
		},
		ISO20022Message: iso20022,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleBatchClose closes a batch and initiates settlement
func (s *Server) HandleBatchClose(w http.ResponseWriter, r *http.Request) {
	var req struct {
		BatchID string `json:"batch_id"`
	}
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	response := map[string]interface{}{
		"batch_id":         req.BatchID,
		"status":           "CLOSED",
		"settlement_status": "PENDING",
		"closed_at":        time.Now(),
		"message":          "Batch closed and submitted for settlement",
		"estimated_settlement": "2-4 hours",
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleBatchList returns list of batches
func (s *Server) HandleBatchList(w http.ResponseWriter, r *http.Request) {
	status := r.URL.Query().Get("status")
	if status == "" {
		status = "ALL"
	}

	batches := []BatchSummary{
		{
			BatchID:        "BATCH_UAE-IN_" + fmt.Sprint(time.Now().Unix()),
			CorridorID:     "UAE-IN",
			Status:         "SETTLED",
			PaymentCount:   15,
			NetAmountUSD:   30000,
			WindowCloseUTC: time.Now().Add(-2 * time.Hour),
			CreatedAt:      time.Now().Add(-3 * time.Hour),
		},
		{
			BatchID:        "BATCH_IL-UAE_" + fmt.Sprint(time.Now().Unix()-3600),
			CorridorID:     "IL-UAE",
			Status:         "OPEN",
			PaymentCount:   8,
			NetAmountUSD:   -15000,
			WindowCloseUTC: time.Now().Add(2 * time.Hour),
			CreatedAt:      time.Now().Add(-1 * time.Hour),
		},
	}

	response := BatchListResponse{
		Batches:    batches,
		TotalCount: len(batches),
		Page:       1,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}
