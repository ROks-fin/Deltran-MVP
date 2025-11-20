package orchestration

import (
	"context"
	"fmt"
	"log"
	"time"

	"deltran/gateway/internal/clients"
	"deltran/gateway/internal/models"
	"deltran/gateway/internal/repository"
)

// TransactionOrchestrator orchestrates the full transaction flow
type TransactionOrchestrator struct {
	complianceClient   *clients.ComplianceClient
	riskClient         *clients.RiskClient
	liquidityClient    *clients.LiquidityClient
	obligationClient   *clients.ObligationClient
	tokenClient        *clients.TokenClient
	notificationClient *clients.NotificationClient
	bankRepo           *repository.BankRepository
}

// NewTransactionOrchestrator creates a new transaction orchestrator
func NewTransactionOrchestrator(
	complianceClient *clients.ComplianceClient,
	riskClient *clients.RiskClient,
	liquidityClient *clients.LiquidityClient,
	obligationClient *clients.ObligationClient,
	tokenClient *clients.TokenClient,
	notificationClient *clients.NotificationClient,
	bankRepo *repository.BankRepository,
) *TransactionOrchestrator {
	return &TransactionOrchestrator{
		complianceClient:   complianceClient,
		riskClient:         riskClient,
		liquidityClient:    liquidityClient,
		obligationClient:   obligationClient,
		tokenClient:        tokenClient,
		notificationClient: notificationClient,
		bankRepo:           bankRepo,
	}
}

// ProcessTransfer processes a transfer request through the full flow
// Flow: Compliance → Risk → Liquidity → Obligation → Token → Success
func (o *TransactionOrchestrator) ProcessTransfer(ctx context.Context, req models.TransferRequest, transactionID string) (*models.TransferResponse, error) {
	startTime := time.Now()

	log.Printf("[%s] Starting transaction flow for %s -> %s, amount: %.2f %s",
		transactionID, req.SenderBank, req.ReceiverBank, req.Amount, req.FromCurrency)

	// Step 1: Compliance Check
	complianceResult, err := o.checkCompliance(ctx, req, transactionID)
	if err != nil {
		return nil, fmt.Errorf("compliance check failed: %w", err)
	}

	if !complianceResult.Passed {
		log.Printf("[%s] Transaction blocked by compliance", transactionID)
		return &models.TransferResponse{
			TransactionID:   transactionID,
			Status:          string(models.StatusBlocked),
			Message:         "Transaction blocked by compliance checks",
			CreatedAt:       time.Now(),
			ComplianceCheck: complianceResult,
		}, nil
	}

	log.Printf("[%s] Compliance check passed (%.2fs)", transactionID, time.Since(startTime).Seconds())

	// Step 2: Risk Evaluation
	riskResult, err := o.evaluateRisk(ctx, req, transactionID)
	if err != nil {
		return nil, fmt.Errorf("risk evaluation failed: %w", err)
	}

	if !riskResult.Approved {
		log.Printf("[%s] Transaction blocked by risk engine: %s", transactionID, riskResult.Level)
		return &models.TransferResponse{
			TransactionID:   transactionID,
			Status:          string(models.StatusBlocked),
			Message:         fmt.Sprintf("Transaction blocked: Risk level %s", riskResult.Level),
			CreatedAt:       time.Now(),
			ComplianceCheck: complianceResult,
			RiskScore:       riskResult,
		}, nil
	}

	log.Printf("[%s] Risk evaluation passed: %s (score: %.2f) (%.2fs)",
		transactionID, riskResult.Level, riskResult.Score, time.Since(startTime).Seconds())

	// Step 3: Liquidity Check
	liquidityResult, err := o.checkLiquidity(ctx, req, transactionID)
	if err != nil {
		return nil, fmt.Errorf("liquidity check failed: %w", err)
	}

	log.Printf("[%s] Liquidity check: instant=%v, confidence=%.2f (%.2fs)",
		transactionID, liquidityResult.InstantSettlement, liquidityResult.Confidence, time.Since(startTime).Seconds())

	// Step 4: Create Obligation
	obligationResult, err := o.createObligation(ctx, req, transactionID)
	if err != nil {
		return nil, fmt.Errorf("obligation creation failed: %w", err)
	}

	log.Printf("[%s] Obligation created: %s, window: %d (%.2fs)",
		transactionID, obligationResult.ObligationID, obligationResult.ClearingWindow, time.Since(startTime).Seconds())

	// Step 5: Tokenize (optional - for instant settlement)
	if liquidityResult.InstantSettlement {
		_, err = o.tokenizePayment(ctx, req, transactionID)
		if err != nil {
			log.Printf("[%s] Warning: Tokenization failed: %v", transactionID, err)
			// Continue anyway - will be processed in clearing window
		} else {
			log.Printf("[%s] Payment tokenized (%.2fs)", transactionID, time.Since(startTime).Seconds())
		}
	}

	// Step 6: Send notification (async)
	go o.sendNotification(context.Background(), req, transactionID, "transaction_initiated")

	estimatedTime := "6 hours (next clearing window)"
	if liquidityResult.InstantSettlement {
		estimatedTime = "5-30 seconds"
	}

	log.Printf("[%s] Transaction flow completed successfully (%.2fs total)",
		transactionID, time.Since(startTime).Seconds())

	return &models.TransferResponse{
		TransactionID:     transactionID,
		Status:            string(models.StatusProcessing),
		Message:           "Transaction initiated successfully",
		InstantSettlement: liquidityResult.InstantSettlement,
		EstimatedTime:     estimatedTime,
		CreatedAt:         time.Now(),
		ComplianceCheck:   complianceResult,
		RiskScore:         riskResult,
	}, nil
}

// checkCompliance performs compliance checks
func (o *TransactionOrchestrator) checkCompliance(ctx context.Context, req models.TransferRequest, txID string) (*models.ComplianceResult, error) {
	// Get sender bank UUID from database
	senderBank, err := o.bankRepo.GetBankByCode(ctx, req.SenderBank)
	if err != nil {
		return nil, fmt.Errorf("failed to get sender bank: %w", err)
	}

	// Get receiver bank UUID from database
	receiverBank, err := o.bankRepo.GetBankByCode(ctx, req.ReceiverBank)
	if err != nil {
		return nil, fmt.Errorf("failed to get receiver bank: %w", err)
	}

	// Build compliance check request with proper UUIDs and all required fields
	checkReq := clients.ComplianceCheckRequest{
		TransactionID:    txID,
		SenderName:       req.SenderName,
		SenderAccount:    req.SenderAccount,
		SenderCountry:    senderBank.Country,
		SenderBankID:     senderBank.ID,
		ReceiverName:     req.ReceiverName,
		ReceiverAccount:  req.ReceiverAccount,
		ReceiverCountry:  receiverBank.Country,
		ReceiverBankID:   receiverBank.ID,
		Amount:           fmt.Sprintf("%.2f", req.Amount),
		Currency:         req.FromCurrency,
		Purpose:          req.Reference,
	}

	resp, err := o.complianceClient.CheckCompliance(ctx, checkReq)
	if err != nil {
		return nil, err
	}

	checkedAt, _ := time.Parse(time.RFC3339, resp.CheckedAt)

	return &models.ComplianceResult{
		Passed:         resp.Passed,
		SanctionsCheck: resp.SanctionsCheck,
		AMLCheck:       resp.AMLCheck,
		Alerts:         resp.Alerts,
		CheckedAt:      checkedAt,
	}, nil
}

// evaluateRisk performs risk evaluation
func (o *TransactionOrchestrator) evaluateRisk(ctx context.Context, req models.TransferRequest, txID string) (*models.RiskResult, error) {
	corridor := fmt.Sprintf("%s-%s", req.FromCurrency, req.ToCurrency)

	riskReq := clients.RiskEvaluationRequest{
		TransactionID: txID,
		BankID:        req.SenderBank,
		Amount:        req.Amount,
		Currency:      req.FromCurrency,
		Corridor:      corridor,
	}

	resp, err := o.riskClient.EvaluateRisk(ctx, riskReq)
	if err != nil {
		return nil, err
	}

	evaluatedAt, _ := time.Parse(time.RFC3339, resp.EvaluatedAt)

	return &models.RiskResult{
		Score:       resp.Score,
		Level:       resp.Level,
		Approved:    resp.Approved,
		Reasons:     resp.Reasons,
		EvaluatedAt: evaluatedAt,
	}, nil
}

// checkLiquidity checks liquidity for instant settlement
func (o *TransactionOrchestrator) checkLiquidity(ctx context.Context, req models.TransferRequest, txID string) (*models.LiquidityResult, error) {
	corridor := fmt.Sprintf("%s-%s", req.FromCurrency, req.ToCurrency)

	liquidityReq := clients.LiquidityPredictionRequest{
		Corridor:     corridor,
		Amount:       req.Amount,
		FromCurrency: req.FromCurrency,
		ToCurrency:   req.ToCurrency,
		TimeHorizon:  3600, // 1 hour
	}

	resp, err := o.liquidityClient.PredictLiquidity(ctx, liquidityReq)
	if err != nil {
		return nil, err
	}

	checkedAt, _ := time.Parse(time.RFC3339, resp.CheckedAt)

	return &models.LiquidityResult{
		Available:         resp.Available,
		InstantSettlement: resp.InstantSettlement,
		Confidence:        resp.Confidence,
		EstimatedTime:     resp.EstimatedTime,
		CheckedAt:         checkedAt,
	}, nil
}

// createObligation creates an obligation in the system
func (o *TransactionOrchestrator) createObligation(ctx context.Context, req models.TransferRequest, txID string) (*models.ObligationResult, error) {
	obligationReq := clients.CreateObligationRequest{
		TransactionID: txID,
		PayerBank:     req.SenderBank,
		PayeeBank:     req.ReceiverBank,
		Amount:        req.Amount,
		Currency:      req.FromCurrency,
		Reference:     req.Reference,
	}

	resp, err := o.obligationClient.CreateObligation(ctx, obligationReq)
	if err != nil {
		return nil, err
	}

	createdAt, _ := time.Parse(time.RFC3339, resp.CreatedAt)

	return &models.ObligationResult{
		ObligationID:   resp.ObligationID,
		Status:         resp.Status,
		ClearingWindow: resp.ClearingWindow,
		CreatedAt:      createdAt,
	}, nil
}

// tokenizePayment tokenizes the payment for instant settlement
func (o *TransactionOrchestrator) tokenizePayment(ctx context.Context, req models.TransferRequest, txID string) (*models.TokenResult, error) {
	transferReq := clients.TransferTokenRequest{
		FromBank:  req.SenderBank,
		ToBank:    req.ReceiverBank,
		Currency:  req.FromCurrency,
		Amount:    req.Amount,
		Reference: txID,
	}

	resp, err := o.tokenClient.TransferToken(ctx, transferReq)
	if err != nil {
		return nil, err
	}

	createdAt, _ := time.Parse(time.RFC3339, resp.CreatedAt)

	return &models.TokenResult{
		TokenID:   resp.TransferID,
		Amount:    resp.Amount,
		Currency:  resp.Currency,
		Status:    resp.Status,
		CreatedAt: createdAt,
	}, nil
}

// sendNotification sends a notification (async)
func (o *TransactionOrchestrator) sendNotification(ctx context.Context, req models.TransferRequest, txID, eventType string) {
	notifReq := clients.SendNotificationRequest{
		Type:      "websocket",
		Recipient: req.SenderBank,
		Template:  eventType,
		Data: map[string]interface{}{
			"transaction_id": txID,
			"sender_bank":    req.SenderBank,
			"receiver_bank":  req.ReceiverBank,
			"amount":         req.Amount,
			"currency":       req.FromCurrency,
		},
		Priority: "normal",
	}

	_, err := o.notificationClient.SendNotification(ctx, notifReq)
	if err != nil {
		log.Printf("[%s] Warning: Failed to send notification: %v", txID, err)
	}
}
