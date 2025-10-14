// Local integration test generator - 5 banks with real transaction flows
package main

import (
	"bytes"
	"context"
	"encoding/json"
	"fmt"
	"log"
	"math/rand"
	"net/http"
	"os"
	"os/signal"
	"strings"
	"sync"
	"syscall"
	"time"

	"github.com/google/uuid"
	"github.com/shopspring/decimal"
)

// Bank configuration
type Bank struct {
	ID               string
	Name             string
	Country          string
	Currency         string
	SWIFT            string
	AvailableLimits  decimal.Decimal
	DailyTransCount  int
	AvgTransAmount   decimal.Decimal
	TransAmountRange [2]decimal.Decimal // min, max
}

// Payment represents a payment request
type Payment struct {
	PaymentID       string          `json:"payment_id"`
	Amount          decimal.Decimal `json:"amount"`
	Currency        string          `json:"currency"`
	DebtorAccount   string          `json:"debtor_account"`
	DebtorName      string          `json:"debtor_name"`
	DebtorBank      string          `json:"debtor_bank"`
	CreditorAccount string          `json:"creditor_account"`
	CreditorName    string          `json:"creditor_name"`
	CreditorBank    string          `json:"creditor_bank"`
	Purpose         string          `json:"purpose"`
	CreatedAt       time.Time       `json:"created_at"`
}

// 5 test banks with different currencies
var TestBanks = []Bank{
	{
		ID:               "BANK_US_001",
		Name:             "First National Bank USA",
		Country:          "US",
		Currency:         "USD",
		SWIFT:            "FNBAUSNY",
		AvailableLimits:  decimal.NewFromInt(50000000), // $50M
		DailyTransCount:  150,
		AvgTransAmount:   decimal.NewFromInt(75000),   // $75k
		TransAmountRange: [2]decimal.Decimal{decimal.NewFromInt(1000), decimal.NewFromInt(500000)},
	},
	{
		ID:               "BANK_EU_001",
		Name:             "Deutsche Handelsbank",
		Country:          "DE",
		Currency:         "EUR",
		SWIFT:            "DHBADEFF",
		AvailableLimits:  decimal.NewFromInt(40000000), // €40M
		DailyTransCount:  200,
		AvgTransAmount:   decimal.NewFromInt(50000),   // €50k
		TransAmountRange: [2]decimal.Decimal{decimal.NewFromInt(500), decimal.NewFromInt(350000)},
	},
	{
		ID:               "BANK_UK_001",
		Name:             "London Sterling Bank",
		Country:          "GB",
		Currency:         "GBP",
		SWIFT:            "LSBNGB2L",
		AvailableLimits:  decimal.NewFromInt(30000000), // £30M
		DailyTransCount:  120,
		AvgTransAmount:   decimal.NewFromInt(60000),   // £60k
		TransAmountRange: [2]decimal.Decimal{decimal.NewFromInt(800), decimal.NewFromInt(400000)},
	},
	{
		ID:               "BANK_AE_001",
		Name:             "Emirates Commercial Bank",
		Country:          "AE",
		Currency:         "AED",
		SWIFT:            "ECBAAEAD",
		AvailableLimits:  decimal.NewFromInt(100000000), // 100M AED
		DailyTransCount:  180,
		AvgTransAmount:   decimal.NewFromInt(200000),   // 200k AED
		TransAmountRange: [2]decimal.Decimal{decimal.NewFromInt(5000), decimal.NewFromInt(2000000)},
	},
	{
		ID:               "BANK_IN_001",
		Name:             "Reserve Bank of Commerce India",
		Country:          "IN",
		Currency:         "INR",
		SWIFT:            "RBCIINBB",
		AvailableLimits:  decimal.NewFromInt(2000000000), // 2B INR
		DailyTransCount:  300,
		AvgTransAmount:   decimal.NewFromInt(5000000),   // 5M INR
		TransAmountRange: [2]decimal.Decimal{decimal.NewFromInt(10000), decimal.NewFromInt(50000000)},
	},
}

// Integration test generator
type IntegrationTestGenerator struct {
	gatewayURL      string
	banks           []Bank
	httpClient      *http.Client
	stats           *Statistics
	mu              sync.Mutex
	transactionPool []*Payment
}

// Statistics
type Statistics struct {
	TotalSubmitted  int
	TotalSucceeded  int
	TotalFailed     int
	ByBank          map[string]*BankStats
	ByCurrency      map[string]*CurrencyStats
	StartTime       time.Time
	mu              sync.Mutex
}

type BankStats struct {
	Submitted int
	Succeeded int
	Failed    int
	Volume    decimal.Decimal
}

type CurrencyStats struct {
	Count  int
	Volume decimal.Decimal
}

func NewIntegrationTestGenerator(gatewayURL string) *IntegrationTestGenerator {
	return &IntegrationTestGenerator{
		gatewayURL: gatewayURL,
		banks:      TestBanks,
		httpClient: &http.Client{Timeout: 10 * time.Second},
		stats: &Statistics{
			ByBank:     make(map[string]*BankStats),
			ByCurrency: make(map[string]*CurrencyStats),
			StartTime:  time.Now(),
		},
		transactionPool: make([]*Payment, 0, 1000),
	}
}

// Generate random amount within bank's range
func (g *IntegrationTestGenerator) randomAmount(bank *Bank) decimal.Decimal {
	min := bank.TransAmountRange[0].IntPart()
	max := bank.TransAmountRange[1].IntPart()
	amount := min + rand.Int63n(max-min+1)
	return decimal.NewFromInt(amount)
}

// Generate random transaction purpose
func randomPurpose() string {
	purposes := []string{
		"Trade Settlement",
		"Invoice Payment",
		"Service Fee",
		"Intercompany Transfer",
		"FX Conversion",
		"Supply Chain Payment",
		"Contract Settlement",
		"Commission Payment",
		"Dividend Distribution",
		"Loan Repayment",
	}
	return purposes[rand.Intn(len(purposes))]
}

// Generate account number
func generateAccountNumber(bank *Bank) string {
	return fmt.Sprintf("%s%010d", bank.Country, rand.Int63n(9999999999))
}

// Generate payment between two banks
func (g *IntegrationTestGenerator) generatePayment(debtorBank, creditorBank *Bank) *Payment {
	return &Payment{
		PaymentID:       uuid.New().String(),
		Amount:          g.randomAmount(debtorBank),
		Currency:        debtorBank.Currency,
		DebtorAccount:   generateAccountNumber(debtorBank),
		DebtorName:      fmt.Sprintf("%s Corporate Client %d", debtorBank.Name, rand.Intn(100)),
		DebtorBank:      debtorBank.SWIFT,
		CreditorAccount: generateAccountNumber(creditorBank),
		CreditorName:    fmt.Sprintf("%s Business Account %d", creditorBank.Name, rand.Intn(100)),
		CreditorBank:    creditorBank.SWIFT,
		Purpose:         randomPurpose(),
		CreatedAt:       time.Now(),
	}
}

// Submit payment to gateway
func (g *IntegrationTestGenerator) submitPayment(payment *Payment) error {
	jsonData, err := json.Marshal(payment)
	if err != nil {
		return fmt.Errorf("failed to marshal payment: %w", err)
	}

	resp, err := g.httpClient.Post(
		fmt.Sprintf("%s/api/v1/payments", g.gatewayURL),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err != nil {
		return fmt.Errorf("failed to submit payment: %w", err)
	}
	defer resp.Body.Close()

	if resp.StatusCode != http.StatusOK && resp.StatusCode != http.StatusAccepted {
		return fmt.Errorf("unexpected status code: %d", resp.StatusCode)
	}

	return nil
}

// Update statistics
func (g *IntegrationTestGenerator) updateStats(payment *Payment, success bool) {
	g.stats.mu.Lock()
	defer g.stats.mu.Unlock()

	g.stats.TotalSubmitted++

	// Bank stats
	debtorBankID := payment.DebtorBank
	if _, exists := g.stats.ByBank[debtorBankID]; !exists {
		g.stats.ByBank[debtorBankID] = &BankStats{Volume: decimal.Zero}
	}

	g.stats.ByBank[debtorBankID].Submitted++
	g.stats.ByBank[debtorBankID].Volume = g.stats.ByBank[debtorBankID].Volume.Add(payment.Amount)

	if success {
		g.stats.TotalSucceeded++
		g.stats.ByBank[debtorBankID].Succeeded++
	} else {
		g.stats.TotalFailed++
		g.stats.ByBank[debtorBankID].Failed++
	}

	// Currency stats
	currency := payment.Currency
	if _, exists := g.stats.ByCurrency[currency]; !exists {
		g.stats.ByCurrency[currency] = &CurrencyStats{Volume: decimal.Zero}
	}
	g.stats.ByCurrency[currency].Count++
	g.stats.ByCurrency[currency].Volume = g.stats.ByCurrency[currency].Volume.Add(payment.Amount)
}

// Run integration test scenario
func (g *IntegrationTestGenerator) Run(ctx context.Context, duration time.Duration) {
	log.Printf("🚀 Starting local integration test for %v\n", duration)
	log.Printf("📊 Configured %d banks with different currencies\n", len(g.banks))

	endTime := time.Now().Add(duration)
	var wg sync.WaitGroup

	// Generate transactions for each bank
	for _, bank := range g.banks {
		wg.Add(1)
		go func(b Bank) {
			defer wg.Done()
			g.generateBankTransactions(ctx, &b, endTime)
		}(bank)
	}

	// Print statistics every 10 seconds
	ticker := time.NewTicker(10 * time.Second)
	go func() {
		for {
			select {
			case <-ticker.C:
				g.printStatistics()
			case <-ctx.Done():
				ticker.Stop()
				return
			}
		}
	}()

	wg.Wait()
	log.Println("\n✅ Integration test completed")
	g.printFinalStatistics()
}

// Generate transactions for a bank
func (g *IntegrationTestGenerator) generateBankTransactions(ctx context.Context, bank *Bank, endTime time.Time) {
	transPerSecond := bank.DailyTransCount / (24 * 60 * 60)
	if transPerSecond < 1 {
		transPerSecond = 1
	}

	ticker := time.NewTicker(time.Second / time.Duration(transPerSecond))
	defer ticker.Stop()

	for {
		select {
		case <-ctx.Done():
			return
		case <-ticker.C:
			if time.Now().After(endTime) {
				return
			}

			// Choose random creditor bank
			creditorBank := &g.banks[rand.Intn(len(g.banks))]

			// Generate payment
			payment := g.generatePayment(bank, creditorBank)

			// Submit payment
			err := g.submitPayment(payment)
			success := err == nil

			if err != nil {
				log.Printf("❌ Failed to submit payment %s: %v\n", payment.PaymentID, err)
			}

			// Update stats
			g.updateStats(payment, success)

			// Store in pool
			g.mu.Lock()
			g.transactionPool = append(g.transactionPool, payment)
			g.mu.Unlock()
		}
	}
}

// Print statistics
func (g *IntegrationTestGenerator) printStatistics() {
	g.stats.mu.Lock()
	defer g.stats.mu.Unlock()

	elapsed := time.Since(g.stats.StartTime)
	tps := float64(g.stats.TotalSubmitted) / elapsed.Seconds()

	fmt.Println("\n" + strings.Repeat("=", 80))
	fmt.Printf("📈 INTEGRATION TEST STATISTICS (Elapsed: %v)\n", elapsed.Round(time.Second))
	fmt.Println(strings.Repeat("=", 80))
	fmt.Printf("Total Submitted: %d | Succeeded: %d | Failed: %d | TPS: %.2f\n",
		g.stats.TotalSubmitted, g.stats.TotalSucceeded, g.stats.TotalFailed, tps)

	fmt.Println("\n💰 BY CURRENCY:")
	for currency, stats := range g.stats.ByCurrency {
		fmt.Printf("  %s: %d transactions, Volume: %s\n",
			currency, stats.Count, stats.Volume.StringFixed(2))
	}

	fmt.Println("\n🏦 BY BANK:")
	for bankID, stats := range g.stats.ByBank {
		successRate := 0.0
		if stats.Submitted > 0 {
			successRate = float64(stats.Succeeded) / float64(stats.Submitted) * 100
		}
		fmt.Printf("  %s: Submitted: %d, Success: %d (%.1f%%), Volume: %s\n",
			bankID, stats.Submitted, stats.Succeeded, successRate, stats.Volume.StringFixed(2))
	}
	fmt.Println(strings.Repeat("=", 80))
}

// Print final statistics
func (g *IntegrationTestGenerator) printFinalStatistics() {
	g.printStatistics()

	fmt.Println("\n📊 FINAL REPORT:")
	fmt.Printf("Total Runtime: %v\n", time.Since(g.stats.StartTime).Round(time.Second))
	fmt.Printf("Total Transactions Generated: %d\n", len(g.transactionPool))
	fmt.Printf("Average TPS: %.2f\n", float64(g.stats.TotalSubmitted)/time.Since(g.stats.StartTime).Seconds())

	successRate := 0.0
	if g.stats.TotalSubmitted > 0 {
		successRate = float64(g.stats.TotalSucceeded) / float64(g.stats.TotalSubmitted) * 100
	}
	fmt.Printf("Success Rate: %.2f%%\n", successRate)

	// Export data for web interface
	g.exportDataForWeb()
}

// Export data for web interface
func (g *IntegrationTestGenerator) exportDataForWeb() {
	g.mu.Lock()
	defer g.mu.Unlock()

	data := map[string]interface{}{
		"timestamp":    time.Now(),
		"banks":        g.banks,
		"transactions": g.transactionPool,
		"statistics":   g.stats,
	}

	jsonData, err := json.MarshalIndent(data, "", "  ")
	if err != nil {
		log.Printf("Failed to export data: %v\n", err)
		return
	}

	// Save to file for web interface to read
	filename := fmt.Sprintf("integration_test_data_%s.json", time.Now().Format("20060102_150405"))
	if err := os.WriteFile(filename, jsonData, 0644); err != nil {
		log.Printf("Failed to write data file: %v\n", err)
		return
	}

	log.Printf("✅ Exported test data to: %s\n", filename)

	// Also send to gateway's analytics endpoint
	resp, err := g.httpClient.Post(
		fmt.Sprintf("%s/api/v1/analytics/import", g.gatewayURL),
		"application/json",
		bytes.NewBuffer(jsonData),
	)
	if err == nil {
		defer resp.Body.Close()
		log.Printf("✅ Sent test data to gateway analytics endpoint\n")
	}
}

func main() {
	rand.Seed(time.Now().UnixNano())

	gatewayURL := "http://localhost:8080"
	if len(os.Args) > 1 {
		gatewayURL = os.Args[1]
	}

	testDuration := 5 * time.Minute
	if len(os.Args) > 2 {
		if d, err := time.ParseDuration(os.Args[2]); err == nil {
			testDuration = d
		}
	}

	generator := NewIntegrationTestGenerator(gatewayURL)

	ctx, cancel := context.WithCancel(context.Background())
	defer cancel()

	// Handle graceful shutdown
	sigChan := make(chan os.Signal, 1)
	signal.Notify(sigChan, os.Interrupt, syscall.SIGTERM)

	go func() {
		<-sigChan
		log.Println("\n⚠️  Received shutdown signal, stopping test...")
		cancel()
	}()

	generator.Run(ctx, testDuration)
}
