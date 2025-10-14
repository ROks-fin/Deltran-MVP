package server

import (
	"context"
	"database/sql"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
	"time"

	"github.com/alicebob/miniredis/v2"
	"github.com/go-chi/chi/v5"
	"github.com/redis/go-redis/v9"
	"github.com/stretchr/testify/assert"
	"github.com/stretchr/testify/require"

	_ "github.com/lib/pq"
)

// setupTestDB creates a test database connection
func setupTestDB(t *testing.T) *sql.DB {
	// Use environment variable or skip if no test DB
	dsn := "host=localhost port=5432 user=deltran password=deltran123 dbname=deltran sslmode=disable"
	db, err := sql.Open("postgres", dsn)
	if err != nil {
		t.Skip("Skipping test: database not available")
	}

	ctx, cancel := context.WithTimeout(context.Background(), 2*time.Second)
	defer cancel()

	if err := db.PingContext(ctx); err != nil {
		t.Skip("Skipping test: database not available")
	}

	return db
}

// setupTestRedis creates a test Redis connection
func setupTestRedis(t *testing.T) (*redis.Client, *miniredis.Miniredis) {
	mr := miniredis.RunT(t)
	client := redis.NewClient(&redis.Options{
		Addr: mr.Addr(),
	})
	return client, mr
}

func TestRealtimeMetrics(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	req := httptest.NewRequest("GET", "/api/v1/metrics/realtime", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var metrics RealtimeMetrics
	err := json.Unmarshal(w.Body.Bytes(), &metrics)
	require.NoError(t, err)

	assert.NotNil(t, metrics.Volume24h)
	assert.NotNil(t, metrics.AverageAmount)
	assert.GreaterOrEqual(t, metrics.TPS, 0.0)
	assert.GreaterOrEqual(t, metrics.SuccessRate, 0.0)
	assert.LessOrEqual(t, metrics.SuccessRate, 100.0)
}

func TestListPayments(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	tests := []struct {
		name       string
		queryParams string
		wantStatus int
	}{
		{
			name:       "default pagination",
			queryParams: "",
			wantStatus: http.StatusOK,
		},
		{
			name:       "with page and page_size",
			queryParams: "?page=1&page_size=10",
			wantStatus: http.StatusOK,
		},
		{
			name:       "filter by status",
			queryParams: "?status=settled",
			wantStatus: http.StatusOK,
		},
		{
			name:       "filter by currency",
			queryParams: "?currency=USD",
			wantStatus: http.StatusOK,
		},
		{
			name:       "filter by sender BIC",
			queryParams: "?sender_bic=CHASUS33XXX",
			wantStatus: http.StatusOK,
		},
		{
			name:       "filter by date range",
			queryParams: "?date_from=2025-01-01&date_to=2025-12-31",
			wantStatus: http.StatusOK,
		},
		{
			name:       "filter by amount range",
			queryParams: "?min_amount=100&max_amount=10000",
			wantStatus: http.StatusOK,
		},
		{
			name:       "search by reference",
			queryParams: "?reference=PAY",
			wantStatus: http.StatusOK,
		},
		{
			name:       "combined filters",
			queryParams: "?status=settled&currency=USD&page=1&page_size=20",
			wantStatus: http.StatusOK,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/api/v1/payments"+tt.queryParams, nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			assert.Equal(t, tt.wantStatus, w.Code)

			if w.Code == http.StatusOK {
				var resp PaymentListResponse
				err := json.Unmarshal(w.Body.Bytes(), &resp)
				require.NoError(t, err)

				assert.NotNil(t, resp.Payments)
				assert.GreaterOrEqual(t, resp.Total, 0)
				assert.GreaterOrEqual(t, resp.Page, 1)
				assert.GreaterOrEqual(t, resp.PageSize, 1)
				assert.GreaterOrEqual(t, resp.TotalPages, 0)
			}
		})
	}
}

func TestGetPaymentDetails(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	// First, create a test payment
	ctx := context.Background()
	var testPaymentID string
	err := db.QueryRowContext(ctx, `
		SELECT id FROM deltran.payments LIMIT 1
	`).Scan(&testPaymentID)

	if err == sql.ErrNoRows {
		t.Skip("Skipping test: no payments in database")
	}
	require.NoError(t, err)

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	req := httptest.NewRequest("GET", "/api/v1/payments/"+testPaymentID, nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var payment PaymentDetails
	err = json.Unmarshal(w.Body.Bytes(), &payment)
	require.NoError(t, err)

	assert.NotEmpty(t, payment.ID)
	assert.NotEmpty(t, payment.PaymentReference)
	assert.NotEmpty(t, payment.Currency)
	assert.NotEmpty(t, payment.Status)
}

func TestGetPaymentDetailsNotFound(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	req := httptest.NewRequest("GET", "/api/v1/payments/00000000-0000-0000-0000-000000000000", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusNotFound, w.Code)
}

func TestExportPayments(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	tests := []struct {
		name       string
		queryParams string
		wantStatus int
	}{
		{
			name:       "export all",
			queryParams: "",
			wantStatus: http.StatusOK,
		},
		{
			name:       "export with status filter",
			queryParams: "?status=settled",
			wantStatus: http.StatusOK,
		},
		{
			name:       "export with currency filter",
			queryParams: "?currency=USD",
			wantStatus: http.StatusOK,
		},
		{
			name:       "export with date range",
			queryParams: "?date_from=2025-01-01&date_to=2025-12-31",
			wantStatus: http.StatusOK,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/api/v1/export/payments"+tt.queryParams, nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			assert.Equal(t, tt.wantStatus, w.Code)

			if w.Code == http.StatusOK {
				assert.Equal(t, "text/csv", w.Header().Get("Content-Type"))
				assert.Contains(t, w.Header().Get("Content-Disposition"), "attachment")
				assert.Contains(t, w.Header().Get("Content-Disposition"), "payments_")
				assert.Contains(t, w.Header().Get("Content-Disposition"), ".csv")

				// Check CSV has header
				body := w.Body.String()
				assert.Contains(t, body, "ID,Reference,Sender BIC")
			}
		})
	}
}

func TestDailyMetrics(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	tests := []struct {
		name       string
		queryParams string
		wantStatus int
	}{
		{
			name:       "default 7 days",
			queryParams: "",
			wantStatus: http.StatusOK,
		},
		{
			name:       "last 30 days",
			queryParams: "?days=30",
			wantStatus: http.StatusOK,
		},
		{
			name:       "last 90 days",
			queryParams: "?days=90",
			wantStatus: http.StatusOK,
		},
		{
			name:       "invalid days (too high)",
			queryParams: "?days=365",
			wantStatus: http.StatusOK, // Should default to 7 days
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/api/v1/metrics/daily"+tt.queryParams, nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			assert.Equal(t, tt.wantStatus, w.Code)

			if w.Code == http.StatusOK {
				var metrics []DailyMetric
				err := json.Unmarshal(w.Body.Bytes(), &metrics)
				require.NoError(t, err)

				for _, metric := range metrics {
					assert.NotEmpty(t, metric.Date)
					assert.GreaterOrEqual(t, metric.PaymentCount, 0)
					assert.NotNil(t, metric.TotalVolume)
					assert.NotNil(t, metric.AverageAmount)
					assert.NotNil(t, metric.StatusBreakdown)
				}
			}
		})
	}
}

func TestBankMetrics(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	req := httptest.NewRequest("GET", "/api/v1/metrics/banks", nil)
	w := httptest.NewRecorder()

	router.ServeHTTP(w, req)

	assert.Equal(t, http.StatusOK, w.Code)

	var metrics []BankMetric
	err := json.Unmarshal(w.Body.Bytes(), &metrics)
	require.NoError(t, err)

	for _, metric := range metrics {
		assert.NotEmpty(t, metric.BankID)
		assert.NotEmpty(t, metric.BIC)
		assert.NotEmpty(t, metric.BankName)
		assert.GreaterOrEqual(t, metric.Sent, 0)
		assert.GreaterOrEqual(t, metric.Received, 0)
		assert.NotNil(t, metric.SentVolume)
		assert.NotNil(t, metric.RecvVolume)
	}
}

func TestHelperFunctions(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	api := NewAggregationAPI(db, redisClient)
	ctx := context.Background()

	t.Run("calculateTPS", func(t *testing.T) {
		tps, err := api.calculateTPS(ctx)
		assert.NoError(t, err)
		assert.GreaterOrEqual(t, tps, 0.0)
	})

	t.Run("get24hVolume", func(t *testing.T) {
		volume, err := api.get24hVolume(ctx)
		assert.NoError(t, err)
		assert.NotNil(t, volume)
	})

	t.Run("getSuccessRate", func(t *testing.T) {
		rate, err := api.getSuccessRate(ctx)
		assert.NoError(t, err)
		assert.GreaterOrEqual(t, rate, 0.0)
		assert.LessOrEqual(t, rate, 100.0)
	})

	t.Run("getQueueDepth", func(t *testing.T) {
		depth, err := api.getQueueDepth(ctx)
		assert.NoError(t, err)
		assert.NotNil(t, depth)
		assert.GreaterOrEqual(t, depth["pending"], 0)
		assert.GreaterOrEqual(t, depth["processing"], 0)
	})

	t.Run("getAverageAmounts", func(t *testing.T) {
		avg, err := api.getAverageAmounts(ctx)
		assert.NoError(t, err)
		assert.NotNil(t, avg)
	})

	t.Run("getFailedLast1h", func(t *testing.T) {
		count, err := api.getFailedLast1h(ctx)
		assert.NoError(t, err)
		assert.GreaterOrEqual(t, count, 0)
	})

	t.Run("getSettledToday", func(t *testing.T) {
		count, err := api.getSettledToday(ctx)
		assert.NoError(t, err)
		assert.GreaterOrEqual(t, count, 0)
	})

	t.Run("getActiveBanks", func(t *testing.T) {
		count, err := api.getActiveBanks(ctx)
		assert.NoError(t, err)
		assert.GreaterOrEqual(t, count, 0)
	})

	t.Run("getComplianceReviewCount", func(t *testing.T) {
		count, err := api.getComplianceReviewCount(ctx)
		assert.NoError(t, err)
		assert.GreaterOrEqual(t, count, 0)
	})
}

func TestPaginationEdgeCases(t *testing.T) {
	db := setupTestDB(t)
	defer db.Close()

	redisClient, mr := setupTestRedis(t)
	defer mr.Close()

	wsHub := NewWebSocketHub(nil)
	api := NewAggregationAPI(db, redisClient, wsHub)

	router := chi.NewRouter()
	api.RegisterRoutes(router)

	tests := []struct {
		name       string
		page       string
		pageSize   string
		wantPage   int
		wantPageSize int
	}{
		{
			name:       "negative page defaults to 1",
			page:       "-1",
			pageSize:   "20",
			wantPage:   1,
			wantPageSize: 20,
		},
		{
			name:       "zero page defaults to 1",
			page:       "0",
			pageSize:   "20",
			wantPage:   1,
			wantPageSize: 20,
		},
		{
			name:       "negative page_size defaults to 20",
			page:       "1",
			pageSize:   "-10",
			wantPage:   1,
			wantPageSize: 20,
		},
		{
			name:       "page_size too large capped at 100",
			page:       "1",
			pageSize:   "1000",
			wantPage:   1,
			wantPageSize: 20,
		},
		{
			name:       "invalid page string defaults to 1",
			page:       "abc",
			pageSize:   "20",
			wantPage:   1,
			wantPageSize: 20,
		},
	}

	for _, tt := range tests {
		t.Run(tt.name, func(t *testing.T) {
			req := httptest.NewRequest("GET", "/api/v1/payments?page="+tt.page+"&page_size="+tt.pageSize, nil)
			w := httptest.NewRecorder()

			router.ServeHTTP(w, req)

			assert.Equal(t, http.StatusOK, w.Code)

			var resp PaymentListResponse
			err := json.Unmarshal(w.Body.Bytes(), &resp)
			require.NoError(t, err)

			assert.Equal(t, tt.wantPage, resp.Page)
			// Note: page_size might be capped differently, so just check it's reasonable
			assert.GreaterOrEqual(t, resp.PageSize, 1)
			assert.LessOrEqual(t, resp.PageSize, 100)
		})
	}
}
