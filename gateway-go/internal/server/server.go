// Gateway server implementation
package server

import (
	"context"
	"fmt"
	"sync"
	"time"

	"github.com/deltran/gateway/internal/config"
	"github.com/deltran/gateway/internal/ledger"
	"github.com/deltran/gateway/internal/types"
	"github.com/deltran/gateway/internal/validation"
	"github.com/google/uuid"
	"github.com/prometheus/client_golang/prometheus"
	"github.com/prometheus/client_golang/prometheus/promauto"
	"go.uber.org/zap"
	"google.golang.org/grpc"
)

var (
	// Metrics
	paymentsTotal = promauto.NewCounterVec(
		prometheus.CounterOpts{
			Name: "gateway_payments_total",
			Help: "Total number of payments processed",
		},
		[]string{"status"},
	)

	paymentDuration = promauto.NewHistogramVec(
		prometheus.HistogramOpts{
			Name:    "gateway_payment_duration_seconds",
			Help:    "Payment processing duration",
			Buckets: []float64{0.01, 0.05, 0.1, 0.25, 0.5, 1, 2.5, 5, 10},
		},
		[]string{"operation"},
	)

	queueDepth = promauto.NewGauge(
		prometheus.GaugeOpts{
			Name: "gateway_queue_depth",
			Help: "Current payment queue depth",
		},
	)
)

// Server represents the gateway server
type Server struct {
	config              *config.Config
	logger              *zap.Logger
	ledger              *ledger.Client
	validator           *validation.Validator
	workers             []*Worker
	queue               chan *types.Payment
	paymentQueue        chan *types.Payment
	wg                  sync.WaitGroup
	shutdown            chan struct{}
	totalTransactions   int64
	failedTransactions  int64
	startTime           time.Time
}

// New creates a new gateway server
func New(cfg *config.Config, logger *zap.Logger) (*Server, error) {
	// Validate config
	if err := cfg.Validate(); err != nil {
		return nil, fmt.Errorf("invalid config: %w", err)
	}

	// Create ledger client
	ledgerClient, err := ledger.NewClient(
		cfg.Ledger.Addr,
		cfg.Ledger.RequestTimeout,
		logger,
	)
	if err != nil {
		return nil, fmt.Errorf("failed to create ledger client: %w", err)
	}

	// Create validator
	validator := validation.New(cfg, logger)

	// Create payment queue
	queue := make(chan *types.Payment, cfg.Limits.QueueSize)

	s := &Server{
		config:              cfg,
		logger:              logger,
		ledger:              ledgerClient,
		validator:           validator,
		queue:               queue,
		paymentQueue:        queue,
		shutdown:            make(chan struct{}),
		totalTransactions:   0,
		failedTransactions:  0,
		startTime:           time.Now(),
	}

	// Start worker pool
	s.startWorkers()

	return s, nil
}

// startWorkers starts the worker pool
func (s *Server) startWorkers() {
	s.workers = make([]*Worker, s.config.Limits.WorkerPoolSize)

	for i := 0; i < s.config.Limits.WorkerPoolSize; i++ {
		worker := &Worker{
			id:       i,
			server:   s,
			logger:   s.logger.With(zap.Int("worker_id", i)),
			shutdown: make(chan struct{}),
		}
		s.workers[i] = worker

		s.wg.Add(1)
		go worker.run()
	}

	s.logger.Info("Started worker pool",
		zap.Int("workers", s.config.Limits.WorkerPoolSize),
		zap.Int("queue_size", s.config.Limits.QueueSize),
	)
}

// RegisterServices registers gRPC services
func (s *Server) RegisterServices(grpcServer *grpc.Server) {
	// TODO: Register protobuf services
	// pb.RegisterGatewayServiceServer(grpcServer, s)
	s.logger.Info("Registered gRPC services")
}

// SubmitPayment submits a payment for processing
func (s *Server) SubmitPayment(ctx context.Context, payment *types.Payment) error {
	start := time.Now()
	defer func() {
		paymentDuration.WithLabelValues("submit").Observe(time.Since(start).Seconds())
	}()

	// Generate payment ID if not present
	if payment.PaymentID == uuid.Nil {
		payment.PaymentID = uuid.New()
	}

	// Set timestamps
	now := time.Now()
	payment.CreatedAt = now
	payment.UpdatedAt = now
	payment.Status = types.PaymentStatusInitiated

	s.logger.Info("Payment submitted",
		zap.String("payment_id", payment.PaymentID.String()),
		zap.String("amount", payment.Amount.String()),
		zap.String("currency", payment.Currency),
	)

	// Enqueue payment
	select {
	case s.queue <- payment:
		queueDepth.Set(float64(len(s.queue)))
		return nil
	case <-ctx.Done():
		return ctx.Err()
	case <-s.shutdown:
		return fmt.Errorf("server shutting down")
	}
}

// GetPaymentStatus retrieves payment status
func (s *Server) GetPaymentStatus(ctx context.Context, paymentID uuid.UUID) (*types.Payment, error) {
	start := time.Now()
	defer func() {
		paymentDuration.WithLabelValues("get_status").Observe(time.Since(start).Seconds())
	}()

	return s.ledger.GetPaymentState(ctx, paymentID)
}

// Close closes the server
func (s *Server) Close() error {
	s.logger.Info("Shutting down gateway server...")

	// Signal shutdown
	close(s.shutdown)

	// Close queue
	close(s.queue)

	// Wait for workers
	s.wg.Wait()

	// Close ledger client
	if err := s.ledger.Close(); err != nil {
		s.logger.Error("Error closing ledger client", zap.Error(err))
	}

	s.logger.Info("Gateway server shutdown complete")
	return nil
}

// Worker processes payments from the queue
type Worker struct {
	id       int
	server   *Server
	logger   *zap.Logger
	shutdown chan struct{}
}

// run runs the worker loop
func (w *Worker) run() {
	defer w.server.wg.Done()

	w.logger.Debug("Worker started")

	for {
		select {
		case payment, ok := <-w.server.queue:
			if !ok {
				w.logger.Debug("Queue closed, worker exiting")
				return
			}

			queueDepth.Set(float64(len(w.server.queue)))

			if err := w.processPayment(payment); err != nil {
				w.logger.Error("Failed to process payment",
					zap.String("payment_id", payment.PaymentID.String()),
					zap.Error(err),
				)
				paymentsTotal.WithLabelValues("failed").Inc()
			} else {
				paymentsTotal.WithLabelValues("succeeded").Inc()
			}

		case <-w.shutdown:
			w.logger.Debug("Worker shutting down")
			return
		}
	}
}

// processPayment processes a single payment
func (w *Worker) processPayment(payment *types.Payment) error {
	start := time.Now()
	defer func() {
		paymentDuration.WithLabelValues("process").Observe(time.Since(start).Seconds())
	}()

	ctx := context.Background()

	// Step 1: Validate payment
	validationResult := w.server.validator.ValidatePayment(payment)
	if !validationResult.Valid {
		w.logger.Warn("Payment validation failed",
			zap.String("payment_id", payment.PaymentID.String()),
			zap.Strings("errors", validationResult.Errors),
		)

		// Record validation failure event
		_, err := w.server.ledger.AppendEvent(ctx, payment, types.EventTypeValidationFailed)
		payment.Status = types.PaymentStatusRejected
		return err
	}

	// Record validation success event
	_, err := w.server.ledger.AppendEvent(ctx, payment, types.EventTypeValidationPassed)
	if err != nil {
		return fmt.Errorf("failed to record validation event: %w", err)
	}
	payment.Status = types.PaymentStatusValidated

	// Step 2: Sanctions screening
	sanctionsCheck := w.server.validator.CheckSanctions(payment)
	if !sanctionsCheck.Cleared {
		w.logger.Warn("Sanctions check failed",
			zap.String("payment_id", payment.PaymentID.String()),
			zap.Strings("hits", sanctionsCheck.Hits),
		)

		_, err := w.server.ledger.AppendEvent(ctx, payment, types.EventTypeSanctionsHit)
		payment.Status = types.PaymentStatusRejected
		return err
	}

	_, err = w.server.ledger.AppendEvent(ctx, payment, types.EventTypeSanctionsCleared)
	if err != nil {
		return fmt.Errorf("failed to record sanctions event: %w", err)
	}
	payment.Status = types.PaymentStatusScreened

	// Step 3: Risk assessment
	riskAssessment := w.server.validator.AssessRisk(payment)
	if !riskAssessment.Approved {
		w.logger.Warn("Risk assessment rejected",
			zap.String("payment_id", payment.PaymentID.String()),
			zap.Float64("risk_score", riskAssessment.RiskScore),
			zap.Strings("reasons", riskAssessment.Reasons),
		)

		_, err := w.server.ledger.AppendEvent(ctx, payment, types.EventTypeRiskRejected)
		payment.Status = types.PaymentStatusRejected
		return err
	}

	_, err = w.server.ledger.AppendEvent(ctx, payment, types.EventTypeRiskApproved)
	if err != nil {
		return fmt.Errorf("failed to record risk event: %w", err)
	}
	payment.Status = types.PaymentStatusApproved

	// Step 4: Queue for settlement
	_, err = w.server.ledger.AppendEvent(ctx, payment, types.EventTypeQueuedForSettlement)
	if err != nil {
		return fmt.Errorf("failed to queue for settlement: %w", err)
	}
	payment.Status = types.PaymentStatusQueued

	w.logger.Info("Payment processed successfully",
		zap.String("payment_id", payment.PaymentID.String()),
		zap.Duration("duration", time.Since(start)),
	)

	return nil
}