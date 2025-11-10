package api

import (
	"context"
	"encoding/json"
	"fmt"
	"net/http"
	"strconv"
	"time"

	"github.com/deltran/reporting-engine/internal/scheduler"
	"github.com/deltran/reporting-engine/internal/storage"
	"github.com/deltran/reporting-engine/pkg/types"
	"github.com/gorilla/mux"
	"go.uber.org/zap"
)

type ReportHandler struct {
	storage    *storage.Storage
	generators map[string]scheduler.ReportGenerator
	logger     *zap.Logger
}

func NewReportHandler(
	storage *storage.Storage,
	generators map[string]scheduler.ReportGenerator,
	logger *zap.Logger,
) *ReportHandler {
	return &ReportHandler{
		storage:    storage,
		generators: generators,
		logger:     logger,
	}
}

func (h *ReportHandler) RegisterRoutes(router *mux.Router) {
	// Root-level health endpoint for monitoring
	router.HandleFunc("/health", h.healthCheck).Methods("GET")
	router.HandleFunc("/metrics", h.metricsEndpoint).Methods("GET")

	api := router.PathPrefix("/api/v1").Subrouter()

	// Report generation and management
	api.HandleFunc("/reports/generate", h.generateReport).Methods("POST")
	api.HandleFunc("/reports/{id}", h.getReport).Methods("GET")
	api.HandleFunc("/reports/{id}/download", h.downloadReport).Methods("GET")
	api.HandleFunc("/reports", h.listReports).Methods("GET")
	api.HandleFunc("/reports/{id}", h.deleteReport).Methods("DELETE")

	// Predefined report types
	api.HandleFunc("/reports/aml/daily", h.generateAMLDaily).Methods("POST")
	api.HandleFunc("/reports/settlement/daily", h.generateSettlementDaily).Methods("POST")

	// Metrics
	api.HandleFunc("/metrics/live", h.getLiveMetrics).Methods("GET")

	// Health check
	api.HandleFunc("/health", h.healthCheck).Methods("GET")
}

func (h *ReportHandler) generateReport(w http.ResponseWriter, r *http.Request) {
	var req types.ReportRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		h.respondError(w, http.StatusBadRequest, "Invalid request body", err)
		return
	}

	// Validate request
	if req.Type == "" {
		h.respondError(w, http.StatusBadRequest, "Report type is required", nil)
		return
	}

	if req.Format == "" {
		req.Format = types.FormatExcel
	}

	if req.PeriodStart.IsZero() || req.PeriodEnd.IsZero() {
		h.respondError(w, http.StatusBadRequest, "Period start and end are required", nil)
		return
	}

	// Get generator
	generator, ok := h.generators[req.Type]
	if !ok {
		h.respondError(w, http.StatusBadRequest, fmt.Sprintf("Unknown report type: %s", req.Type), nil)
		return
	}

	// Generate report asynchronously
	go func() {
		ctx := context.Background()
		result, err := generator.Generate(ctx, req)
		if err != nil {
			h.logger.Error("Failed to generate report",
				zap.String("type", req.Type),
				zap.Error(err))
			return
		}

		// Upload to S3
		storagePath, err := h.storage.UploadReport(ctx, result)
		if err != nil {
			h.logger.Error("Failed to upload report", zap.Error(err))
			return
		}

		// Save metadata
		report := &types.Report{
			ID:          result.ReportID,
			Type:        req.Type,
			Name:        fmt.Sprintf("%s Report", req.Type),
			PeriodStart: req.PeriodStart,
			PeriodEnd:   req.PeriodEnd,
			GeneratedAt: result.GeneratedAt,
			GeneratedBy: req.RequestedBy,
			Status:      types.StatusCompleted,
			StoragePath: storagePath,
			FileSize:    result.FileSize,
			Format:      result.Format,
			Metadata:    req.Parameters,
		}

		if err := h.storage.SaveReportMetadata(ctx, report); err != nil {
			h.logger.Error("Failed to save report metadata", zap.Error(err))
		}
	}()

	h.respondJSON(w, http.StatusAccepted, map[string]string{
		"status":  "processing",
		"message": "Report generation started",
	})
}

func (h *ReportHandler) getReport(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	reportID := vars["id"]

	report, err := h.storage.GetReport(r.Context(), reportID)
	if err != nil {
		h.respondError(w, http.StatusNotFound, "Report not found", err)
		return
	}

	// Log access
	h.storage.LogReportAccess(r.Context(), reportID, "user", "view",
		r.RemoteAddr, r.UserAgent())

	h.respondJSON(w, http.StatusOK, report)
}

func (h *ReportHandler) downloadReport(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	reportID := vars["id"]

	report, err := h.storage.GetReport(r.Context(), reportID)
	if err != nil {
		h.respondError(w, http.StatusNotFound, "Report not found", err)
		return
	}

	// Generate presigned URL for S3 download
	url, err := h.storage.GetPresignedURL(r.Context(), report.StoragePath, 15*time.Minute)
	if err != nil {
		h.respondError(w, http.StatusInternalServerError, "Failed to generate download URL", err)
		return
	}

	// Log access
	h.storage.LogReportAccess(r.Context(), reportID, "user", "download",
		r.RemoteAddr, r.UserAgent())

	h.respondJSON(w, http.StatusOK, map[string]string{
		"download_url": url,
		"expires_in":   "15 minutes",
	})
}

func (h *ReportHandler) listReports(w http.ResponseWriter, r *http.Request) {
	reportType := r.URL.Query().Get("type")
	limitStr := r.URL.Query().Get("limit")
	offsetStr := r.URL.Query().Get("offset")

	limit := 50
	offset := 0

	if limitStr != "" {
		if l, err := strconv.Atoi(limitStr); err == nil {
			limit = l
		}
	}

	if offsetStr != "" {
		if o, err := strconv.Atoi(offsetStr); err == nil {
			offset = o
		}
	}

	reports, err := h.storage.ListReports(r.Context(), reportType, limit, offset)
	if err != nil {
		h.respondError(w, http.StatusInternalServerError, "Failed to list reports", err)
		return
	}

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"reports": reports,
		"count":   len(reports),
		"limit":   limit,
		"offset":  offset,
	})
}

func (h *ReportHandler) deleteReport(w http.ResponseWriter, r *http.Request) {
	vars := mux.Vars(r)
	reportID := vars["id"]

	if err := h.storage.DeleteReport(r.Context(), reportID); err != nil {
		h.respondError(w, http.StatusInternalServerError, "Failed to delete report", err)
		return
	}

	h.respondJSON(w, http.StatusOK, map[string]string{
		"status":  "deleted",
		"message": "Report deleted successfully",
	})
}

func (h *ReportHandler) generateAMLDaily(w http.ResponseWriter, r *http.Request) {
	yesterday := time.Now().AddDate(0, 0, -1)
	startOfDay := time.Date(yesterday.Year(), yesterday.Month(), yesterday.Day(), 0, 0, 0, 0, time.UTC)
	endOfDay := startOfDay.Add(24 * time.Hour)

	req := types.ReportRequest{
		Type:        types.ReportTypeAML,
		Format:      types.FormatExcel,
		PeriodStart: startOfDay,
		PeriodEnd:   endOfDay,
		RequestedBy: "api_user",
	}

	h.generateReportSync(w, r, req)
}

func (h *ReportHandler) generateSettlementDaily(w http.ResponseWriter, r *http.Request) {
	yesterday := time.Now().AddDate(0, 0, -1)
	startOfDay := time.Date(yesterday.Year(), yesterday.Month(), yesterday.Day(), 0, 0, 0, 0, time.UTC)
	endOfDay := startOfDay.Add(24 * time.Hour)

	req := types.ReportRequest{
		Type:        types.ReportTypeSettlement,
		Format:      types.FormatCSV,
		PeriodStart: startOfDay,
		PeriodEnd:   endOfDay,
		RequestedBy: "api_user",
	}

	h.generateReportSync(w, r, req)
}

func (h *ReportHandler) generateReportSync(w http.ResponseWriter, r *http.Request, req types.ReportRequest) {
	generator, ok := h.generators[req.Type]
	if !ok {
		h.respondError(w, http.StatusBadRequest, "Unknown report type", nil)
		return
	}

	result, err := generator.Generate(r.Context(), req)
	if err != nil {
		h.respondError(w, http.StatusInternalServerError, "Failed to generate report", err)
		return
	}

	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"report_id": result.ReportID,
		"status":    "completed",
		"size":      result.FileSize,
		"format":    result.Format,
	})
}

func (h *ReportHandler) getLiveMetrics(w http.ResponseWriter, r *http.Request) {
	date := r.URL.Query().Get("date")
	if date == "" {
		date = time.Now().Format("2006-01-02")
	}

	metrics, err := h.storage.GetCachedMetrics(r.Context(), date)
	if err != nil {
		h.respondError(w, http.StatusInternalServerError, "Failed to get metrics", err)
		return
	}

	h.respondJSON(w, http.StatusOK, metrics)
}

func (h *ReportHandler) healthCheck(w http.ResponseWriter, r *http.Request) {
	h.respondJSON(w, http.StatusOK, map[string]interface{}{
		"status":  "healthy",
		"service": "reporting-engine",
		"version": "1.0.0",
		"time":    time.Now().Format(time.RFC3339),
	})
}

func (h *ReportHandler) metricsEndpoint(w http.ResponseWriter, r *http.Request) {
	// Placeholder for Prometheus metrics
	w.Header().Set("Content-Type", "text/plain")
	w.WriteHeader(http.StatusOK)
	fmt.Fprintf(w, "# Prometheus metrics placeholder\n")
}

func (h *ReportHandler) respondJSON(w http.ResponseWriter, status int, data interface{}) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(data)
}

func (h *ReportHandler) respondError(w http.ResponseWriter, status int, message string, err error) {
	h.logger.Error(message, zap.Error(err))
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(status)
	json.NewEncoder(w).Encode(map[string]string{
		"error":   message,
		"details": fmt.Sprintf("%v", err),
	})
}
