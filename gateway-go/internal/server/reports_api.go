package server

import (
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
	"strings"
	"time"

	"github.com/google/uuid"
)

// ========== REGULATORY REPORTS API ==========

// ReportGenerateRequest creates a new regulatory report
type ReportGenerateRequest struct {
	ReportType  string    `json:"report_type"`  // AML_ANNUAL, PRU_MONTHLY, CTR, etc.
	Format      string    `json:"format"`       // excel, pdf, csv, json, xml
	StartDate   time.Time `json:"start_date"`
	EndDate     time.Time `json:"end_date"`
	Currency    string    `json:"currency,omitempty"`
	Country     string    `json:"country,omitempty"`
	GeneratedBy string    `json:"generated_by"`
}

// ReportMetadataResponse returns report metadata
type ReportMetadataResponse struct {
	ReportID    string    `json:"report_id"`
	ReportType  string    `json:"report_type"`
	Format      string    `json:"format"`
	Status      string    `json:"status"`
	GeneratedAt time.Time `json:"generated_at"`
	PeriodStart time.Time `json:"period_start"`
	PeriodEnd   time.Time `json:"period_end"`
	FileSize    int64     `json:"file_size"`
	DownloadURL string    `json:"download_url"`
	GeneratedBy string    `json:"generated_by"`
	ApprovedBy  *string   `json:"approved_by,omitempty"`
	SubmittedAt *time.Time `json:"submitted_at,omitempty"`
}

// ReportListResponse returns list of available reports
type ReportListResponse struct {
	Reports    []ReportMetadataResponse `json:"reports"`
	TotalCount int                      `json:"total_count"`
	Page       int                      `json:"page"`
	PageSize   int                      `json:"page_size"`
}

// ReportStatsResponse returns report statistics
type ReportStatsResponse struct {
	TotalReports    int            `json:"total_reports"`
	ByType          map[string]int `json:"by_type"`
	ByStatus        map[string]int `json:"by_status"`
	TotalSizeBytes  int64          `json:"total_size_bytes"`
	OldestReport    *time.Time     `json:"oldest_report,omitempty"`
	LatestReport    *time.Time     `json:"latest_report,omitempty"`
}

// HandleReportGenerate generates a new regulatory report
func (s *Server) HandleReportGenerate(w http.ResponseWriter, r *http.Request) {
	var req ReportGenerateRequest
	if err := json.NewDecoder(r.Body).Decode(&req); err != nil {
		http.Error(w, err.Error(), http.StatusBadRequest)
		return
	}

	// Validate request
	if req.ReportType == "" {
		http.Error(w, "report_type is required", http.StatusBadRequest)
		return
	}
	if req.Format == "" {
		req.Format = "excel" // default
	}
	if req.GeneratedBy == "" {
		req.GeneratedBy = "system"
	}

	// Generate report ID
	reportID := uuid.New().String()
	timestamp := time.Now()

	// Generate mock report data
	reportData := s.generateMockReportData(&req)

	// Create report file
	reportsDir := "./data/reports"
	os.MkdirAll(reportsDir, 0755)

	fileExt := s.getFileExtension(req.Format)
	filename := fmt.Sprintf("%s_%s_%s_%s.%s",
		req.ReportType,
		req.StartDate.Format("20060102"),
		req.EndDate.Format("20060102"),
		reportID[:8],
		fileExt)

	filePath := filepath.Join(reportsDir, filename)

	// Write report content
	if err := os.WriteFile(filePath, []byte(reportData), 0644); err != nil {
		http.Error(w, fmt.Sprintf("Failed to write report: %v", err), http.StatusInternalServerError)
		return
	}

	// Get file size
	fileInfo, _ := os.Stat(filePath)
	var fileSize int64
	if fileInfo != nil {
		fileSize = fileInfo.Size()
	}

	// Generate download URL
	downloadURL := fmt.Sprintf("/api/v1/compliance/reports/%s/download", reportID)

	// Store report metadata (in production, use database)
	s.storeReportMetadata(reportID, filename, &req, fileSize)

	response := ReportMetadataResponse{
		ReportID:    reportID,
		ReportType:  req.ReportType,
		Format:      req.Format,
		Status:      "READY",
		GeneratedAt: timestamp,
		PeriodStart: req.StartDate,
		PeriodEnd:   req.EndDate,
		FileSize:    fileSize,
		DownloadURL: downloadURL,
		GeneratedBy: req.GeneratedBy,
		ApprovedBy:  nil,
		SubmittedAt: nil,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleReportDownload downloads a generated report
func (s *Server) HandleReportDownload(w http.ResponseWriter, r *http.Request) {
	// Extract report_id from URL path
	pathParts := strings.Split(strings.TrimPrefix(r.URL.Path, "/"), "/")
	var reportID string
	for i, part := range pathParts {
		if part == "reports" && i+1 < len(pathParts) {
			reportID = pathParts[i+1]
			break
		}
	}

	if reportID == "" || reportID == "stats" {
		http.Error(w, "report_id is required", http.StatusBadRequest)
		return
	}

	// Get report metadata from storage (in production, use database)
	metadata, filename, err := s.getReportMetadata(reportID)
	if err != nil {
		http.Error(w, "Report not found", http.StatusNotFound)
		return
	}

	// Open report file
	filePath := filepath.Join("./data/reports", filename)
	file, err := os.Open(filePath)
	if err != nil {
		http.Error(w, "Report file not found", http.StatusNotFound)
		return
	}
	defer file.Close()

	// Get file info
	fileInfo, err := file.Stat()
	if err != nil {
		http.Error(w, "Failed to read file info", http.StatusInternalServerError)
		return
	}

	// Set response headers
	w.Header().Set("Content-Type", s.getMimeType(metadata.Format))
	w.Header().Set("Content-Disposition", fmt.Sprintf("attachment; filename=\"%s\"", filename))
	w.Header().Set("Content-Length", fmt.Sprintf("%d", fileInfo.Size()))
	w.Header().Set("Access-Control-Allow-Origin", "*")
	w.Header().Set("Access-Control-Expose-Headers", "Content-Disposition")

	// Stream file to response
	io.Copy(w, file)
}

// HandleReportList lists all generated reports
func (s *Server) HandleReportList(w http.ResponseWriter, r *http.Request) {
	// Parse query parameters
	reportType := r.URL.Query().Get("type")
	status := r.URL.Query().Get("status")

	// Read reports directory
	reportsDir := "./data/reports"
	entries, err := os.ReadDir(reportsDir)
	if err != nil {
		// Directory doesn't exist yet
		response := ReportListResponse{
			Reports:    []ReportMetadataResponse{},
			TotalCount: 0,
			Page:       1,
			PageSize:   50,
		}
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Access-Control-Allow-Origin", "*")
		json.NewEncoder(w).Encode(response)
		return
	}

	var reports []ReportMetadataResponse

	// Iterate through report files
	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		fileInfo, err := entry.Info()
		if err != nil {
			continue
		}

		// Parse filename to extract metadata
		metadata := s.parseReportFilename(entry.Name(), fileInfo)

		// Apply filters
		if reportType != "" && metadata.ReportType != reportType {
			continue
		}
		if status != "" && metadata.Status != status {
			continue
		}

		reports = append(reports, metadata)
	}

	response := ReportListResponse{
		Reports:    reports,
		TotalCount: len(reports),
		Page:       1,
		PageSize:   50,
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(response)
}

// HandleReportStats returns report statistics
func (s *Server) HandleReportStats(w http.ResponseWriter, r *http.Request) {
	reportsDir := "./data/reports"
	entries, err := os.ReadDir(reportsDir)
	if err != nil {
		// Directory doesn't exist yet
		response := ReportStatsResponse{
			TotalReports:   0,
			ByType:         make(map[string]int),
			ByStatus:       make(map[string]int),
			TotalSizeBytes: 0,
		}
		w.Header().Set("Content-Type", "application/json")
		w.Header().Set("Access-Control-Allow-Origin", "*")
		json.NewEncoder(w).Encode(response)
		return
	}

	stats := ReportStatsResponse{
		ByType:   make(map[string]int),
		ByStatus: make(map[string]int),
	}

	var oldestTime, latestTime time.Time

	for _, entry := range entries {
		if entry.IsDir() {
			continue
		}

		fileInfo, err := entry.Info()
		if err != nil {
			continue
		}

		stats.TotalReports++
		stats.TotalSizeBytes += fileInfo.Size()

		// Track oldest and latest
		modTime := fileInfo.ModTime()
		if oldestTime.IsZero() || modTime.Before(oldestTime) {
			oldestTime = modTime
		}
		if latestTime.IsZero() || modTime.After(latestTime) {
			latestTime = modTime
		}

		// Parse filename for type
		metadata := s.parseReportFilename(entry.Name(), fileInfo)
		stats.ByType[metadata.ReportType]++
		stats.ByStatus[metadata.Status]++
	}

	if !oldestTime.IsZero() {
		stats.OldestReport = &oldestTime
	}
	if !latestTime.IsZero() {
		stats.LatestReport = &latestTime
	}

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(stats)
}

// HandleReportDelete deletes a report
func (s *Server) HandleReportDelete(w http.ResponseWriter, r *http.Request) {
	// Extract report_id from URL path
	pathParts := strings.Split(strings.TrimPrefix(r.URL.Path, "/"), "/")
	var reportID string
	for i, part := range pathParts {
		if part == "reports" && i+1 < len(pathParts) {
			reportID = pathParts[i+1]
			break
		}
	}

	if reportID == "" || reportID == "stats" {
		http.Error(w, "report_id is required", http.StatusBadRequest)
		return
	}

	// Get report metadata
	_, filename, err := s.getReportMetadata(reportID)
	if err != nil {
		http.Error(w, "Report not found", http.StatusNotFound)
		return
	}

	// Delete report file
	filePath := filepath.Join("./data/reports", filename)
	if err := os.Remove(filePath); err != nil {
		http.Error(w, "Failed to delete report", http.StatusInternalServerError)
		return
	}

	// Remove from metadata storage
	s.deleteReportMetadata(reportID)

	w.Header().Set("Content-Type", "application/json")
	w.Header().Set("Access-Control-Allow-Origin", "*")
	json.NewEncoder(w).Encode(map[string]string{
		"status":  "success",
		"message": "Report deleted successfully",
	})
}

// Helper functions

func (s *Server) generateMockReportData(req *ReportGenerateRequest) string {
	data := fmt.Sprintf("=== %s REPORT ===\n\n", req.ReportType)
	data += fmt.Sprintf("Generated: %s\n", time.Now().Format("2006-01-02 15:04:05 UTC"))
	data += fmt.Sprintf("Period: %s to %s\n\n", req.StartDate.Format("2006-01-02"), req.EndDate.Format("2006-01-02"))

	data += "SUMMARY\n"
	data += "Total Transactions: 1,831,185\n"
	data += "Total Volume: $2,671,198,391,358.93\n"
	data += "Success Rate: 97.49%\n\n"

	data += "COMPLIANCE CHECKS\n"
	data += "Total Checks: 1,831,185\n"
	data += "Passed: 1,785,305 (97.5%)\n"
	data += "Flagged: 45,880 (2.5%)\n"
	data += "Sanctions Hits: 0\n"
	data += "PEP Matches: 0\n"
	data += "AML Alerts: 0\n\n"

	data += "RISK METRICS\n"
	data += "Average Risk Score: 15.5\n"
	data += "High Risk Transactions: 18,312 (1.0%)\n"
	data += "Fraud Alerts: 0\n"
	data += "Velocity Violations: 0\n\n"

	data += "Generated by DelTran Regulatory Reporting System\n"
	data += "For Big 4 Audit Compliance\n"

	return data
}

func (s *Server) getFileExtension(format string) string {
	switch format {
	case "excel":
		return "xlsx"
	case "pdf":
		return "pdf"
	case "csv":
		return "csv"
	case "json":
		return "json"
	case "xml":
		return "xml"
	default:
		return "txt"
	}
}

func (s *Server) getMimeType(format string) string {
	switch format {
	case "excel":
		return "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
	case "pdf":
		return "application/pdf"
	case "csv":
		return "text/csv"
	case "json":
		return "application/json"
	case "xml":
		return "application/xml"
	default:
		return "text/plain"
	}
}

func (s *Server) parseReportFilename(filename string, fileInfo os.FileInfo) ReportMetadataResponse {
	// Extract report ID from filename (last 8 chars before extension)
	reportID := uuid.New().String()

	// Extract report type from filename prefix
	reportType := "UNKNOWN"
	if len(filename) > 10 {
		reportType = filename[:10]
	}

	return ReportMetadataResponse{
		ReportID:    reportID,
		ReportType:  reportType,
		Format:      "excel",
		Status:      "READY",
		GeneratedAt: fileInfo.ModTime(),
		PeriodStart: time.Now().AddDate(0, -1, 0),
		PeriodEnd:   time.Now(),
		FileSize:    fileInfo.Size(),
		DownloadURL: fmt.Sprintf("/api/v1/compliance/reports/%s/download", reportID),
		GeneratedBy: "system",
	}
}

// In-memory metadata storage (in production, use database)
var reportMetadataStore = make(map[string]reportMetadataEntry)

type reportMetadataEntry struct {
	Metadata ReportMetadataResponse
	Filename string
}

func (s *Server) storeReportMetadata(reportID, filename string, req *ReportGenerateRequest, fileSize int64) {
	reportMetadataStore[reportID] = reportMetadataEntry{
		Metadata: ReportMetadataResponse{
			ReportID:    reportID,
			ReportType:  req.ReportType,
			Format:      req.Format,
			Status:      "READY",
			GeneratedAt: time.Now(),
			PeriodStart: req.StartDate,
			PeriodEnd:   req.EndDate,
			FileSize:    fileSize,
			DownloadURL: fmt.Sprintf("/api/v1/compliance/reports/%s/download", reportID),
			GeneratedBy: req.GeneratedBy,
		},
		Filename: filename,
	}
}

func (s *Server) getReportMetadata(reportID string) (ReportMetadataResponse, string, error) {
	entry, exists := reportMetadataStore[reportID]
	if !exists {
		return ReportMetadataResponse{}, "", fmt.Errorf("report not found")
	}
	return entry.Metadata, entry.Filename, nil
}

func (s *Server) deleteReportMetadata(reportID string) {
	delete(reportMetadataStore, reportID)
}
