package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"net/http/httptest"
	"testing"
)

// mockHandler for testing middleware
func mockHandler(statusCode int, message string) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		w.WriteHeader(statusCode)
		w.Write([]byte(message))
	})
}

func TestJWTMiddleware_ValidToken(t *testing.T) {
	jwtManager := NewJWTManager("test-secret-key")

	user := &User{
		ID:       "user123",
		Email:    "test@example.com",
		Username: "testuser",
		Role:     RoleOperator,
	}

	tokenPair, err := jwtManager.GenerateTokenPair(user)
	if err != nil {
		t.Fatalf("Failed to generate token: %v", err)
	}

	token := tokenPair.AccessToken

	middleware := JWTMiddleware(jwtManager)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("Authorization", "Bearer "+token)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}

	if rr.Body.String() != "success" {
		t.Errorf("Expected 'success', got %s", rr.Body.String())
	}
}

func TestJWTMiddleware_MissingAuthHeader(t *testing.T) {
	jwtManager := NewJWTManager("test-secret-key")
	middleware := JWTMiddleware(jwtManager)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusUnauthorized {
		t.Errorf("Expected status 401, got %d", rr.Code)
	}

	var errorResp ErrorResponse
	if err := json.NewDecoder(rr.Body).Decode(&errorResp); err != nil {
		t.Fatalf("Failed to decode error response: %v", err)
	}

	if errorResp.Message != "missing authorization header" {
		t.Errorf("Expected 'missing authorization header', got %s", errorResp.Message)
	}
}

func TestJWTMiddleware_InvalidToken(t *testing.T) {
	jwtManager := NewJWTManager("test-secret-key")
	middleware := JWTMiddleware(jwtManager)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("Authorization", "Bearer invalid.token.here")
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusUnauthorized {
		t.Errorf("Expected status 401, got %d", rr.Code)
	}
}

func TestJWTMiddleware_ExpiredToken(t *testing.T) {
	// This test is skipped because we cannot easily create expired tokens with the current API
	// The real expiration is tested in jwt_test.go
	t.Skip("Expiration testing is covered in jwt_test.go")
}

func TestOptionalJWTMiddleware_WithValidToken(t *testing.T) {
	jwtManager := NewJWTManager("test-secret-key")

	user := &User{
		ID:       "user123",
		Email:    "test@example.com",
		Username: "testuser",
		Role:     RoleOperator,
	}

	tokenPair, err := jwtManager.GenerateTokenPair(user)
	if err != nil {
		t.Fatalf("Failed to generate token: %v", err)
	}

	middleware := OptionalJWTMiddleware(jwtManager)
	handler := middleware(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		claims, ok := GetClaims(r)
		if !ok || claims.UserID != "user123" {
			t.Error("Claims should be in context")
		}
		w.WriteHeader(http.StatusOK)
	}))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("Authorization", "Bearer "+tokenPair.AccessToken)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestOptionalJWTMiddleware_WithoutToken(t *testing.T) {
	jwtManager := NewJWTManager("test-secret-key")

	middleware := OptionalJWTMiddleware(jwtManager)
	handler := middleware(http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		_, ok := GetClaims(r)
		if ok {
			t.Error("Claims should NOT be in context")
		}
		w.WriteHeader(http.StatusOK)
	}))

	req := httptest.NewRequest("GET", "/test", nil)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequirePermission_WithPermission(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		Permissions: []Permission{
			PermPaymentCreate,
			PermPaymentRead,
		},
	}

	middleware := RequirePermission(PermPaymentCreate)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequirePermission_WithoutPermission(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		Permissions: []Permission{
			PermPaymentRead,
		},
	}

	middleware := RequirePermission(PermPaymentCreate)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403, got %d", rr.Code)
	}
}

func TestRequirePermission_NoClaims(t *testing.T) {
	middleware := RequirePermission(PermPaymentCreate)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusUnauthorized {
		t.Errorf("Expected status 401, got %d", rr.Code)
	}
}

func TestRequireAnyPermission_HasOne(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		Permissions: []Permission{
			PermPaymentRead,
		},
	}

	middleware := RequireAnyPermission(PermPaymentCreate, PermPaymentRead)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequireAnyPermission_HasNone(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		Permissions: []Permission{
			PermBatchRead,
		},
	}

	middleware := RequireAnyPermission(PermPaymentCreate, PermPaymentRead)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403, got %d", rr.Code)
	}
}

func TestRequireAllPermissions_HasAll(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		Permissions: []Permission{
			PermPaymentCreate,
			PermPaymentRead,
			PermPaymentUpdate,
		},
	}

	middleware := RequireAllPermissions(PermPaymentCreate, PermPaymentRead)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequireAllPermissions_MissingOne(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		Permissions: []Permission{
			PermPaymentRead,
		},
	}

	middleware := RequireAllPermissions(PermPaymentCreate, PermPaymentRead)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403, got %d", rr.Code)
	}
}

func TestRequireRole_CorrectRole(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
	}

	middleware := RequireRole(RoleOperator)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequireRole_WrongRole(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleViewer,
	}

	middleware := RequireRole(RoleAdmin)
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403, got %d", rr.Code)
	}
}

func TestRequireAdmin_IsAdmin(t *testing.T) {
	claims := &Claims{
		UserID: "admin123",
		Role:   RoleAdmin,
	}

	middleware := RequireAdmin()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequireAdmin_NotAdmin(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleViewer,
	}

	middleware := RequireAdmin()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403, got %d", rr.Code)
	}
}

func TestRequireOperator_IsOperator(t *testing.T) {
	claims := &Claims{
		UserID: "op123",
		Role:   RoleOperator,
	}

	middleware := RequireOperator()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d", rr.Code)
	}
}

func TestRequireOperator_IsAdmin(t *testing.T) {
	claims := &Claims{
		UserID: "admin123",
		Role:   RoleAdmin,
	}

	middleware := RequireOperator()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200, got %d (admin should pass operator check)", rr.Code)
	}
}

func TestRequireOperator_NotOperator(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleViewer,
	}

	middleware := RequireOperator()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403, got %d", rr.Code)
	}
}

func TestRestrictToBankMiddleware_AdminAccess(t *testing.T) {
	claims := &Claims{
		UserID: "admin123",
		Role:   RoleAdmin,
		BankID: "BANK001",
	}

	middleware := RestrictToBankMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/api/banks/BANK002/payments", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	// Admin should access any bank
	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200 for admin, got %d", rr.Code)
	}
}

func TestRestrictToBankMiddleware_SameBank(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		BankID: "BANK001",
	}

	middleware := RestrictToBankMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/api/banks/BANK001/payments", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusOK {
		t.Errorf("Expected status 200 for same bank, got %d", rr.Code)
	}
}

func TestRestrictToBankMiddleware_DifferentBank(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		BankID: "BANK001",
	}

	middleware := RestrictToBankMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/api/banks/BANK002/payments", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403 for different bank, got %d", rr.Code)
	}
}

func TestRestrictToBankMiddleware_NoBankID(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Role:   RoleOperator,
		BankID: "",
	}

	middleware := RestrictToBankMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/api/payments", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusForbidden {
		t.Errorf("Expected status 403 for user without bank, got %d", rr.Code)
	}
}

func TestGetClaims(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
		Email:  "test@example.com",
	}

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)

	gotClaims, ok := GetClaims(req)
	if !ok {
		t.Fatal("GetClaims should return true")
	}

	if gotClaims.UserID != "user123" {
		t.Errorf("Expected user ID 'user123', got %s", gotClaims.UserID)
	}
}

func TestGetClaims_NotPresent(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)

	_, ok := GetClaims(req)
	if ok {
		t.Error("GetClaims should return false when claims not present")
	}
}

func TestGetUserID(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), UserIDContextKey, "user123")
	req = req.WithContext(ctx)

	userID, ok := GetUserID(req)
	if !ok {
		t.Fatal("GetUserID should return true")
	}

	if userID != "user123" {
		t.Errorf("Expected 'user123', got %s", userID)
	}
}

func TestMustGetClaims_Present(t *testing.T) {
	claims := &Claims{
		UserID: "user123",
	}

	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), ClaimsContextKey, claims)
	req = req.WithContext(ctx)

	gotClaims := MustGetClaims(req)
	if gotClaims.UserID != "user123" {
		t.Errorf("Expected user ID 'user123', got %s", gotClaims.UserID)
	}
}

func TestMustGetClaims_NotPresent(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)

	defer func() {
		if r := recover(); r == nil {
			t.Error("MustGetClaims should panic when claims not present")
		}
	}()

	MustGetClaims(req)
}

func TestMustGetUserID_Present(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)
	ctx := context.WithValue(req.Context(), UserIDContextKey, "user123")
	req = req.WithContext(ctx)

	userID := MustGetUserID(req)
	if userID != "user123" {
		t.Errorf("Expected 'user123', got %s", userID)
	}
}

func TestMustGetUserID_NotPresent(t *testing.T) {
	req := httptest.NewRequest("GET", "/test", nil)

	defer func() {
		if r := recover(); r == nil {
			t.Error("MustGetUserID should panic when user ID not present")
		}
	}()

	MustGetUserID(req)
}

func TestCORSMiddleware_AllowedOrigin(t *testing.T) {
	middleware := CORSMiddleware([]string{"https://example.com"})
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("Origin", "https://example.com")
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Header().Get("Access-Control-Allow-Origin") != "https://example.com" {
		t.Error("CORS header should be set for allowed origin")
	}
}

func TestCORSMiddleware_Wildcard(t *testing.T) {
	middleware := CORSMiddleware([]string{"*"})
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("Origin", "https://any-origin.com")
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Header().Get("Access-Control-Allow-Origin") != "https://any-origin.com" {
		t.Error("CORS header should be set for wildcard")
	}
}

func TestCORSMiddleware_DisallowedOrigin(t *testing.T) {
	middleware := CORSMiddleware([]string{"https://example.com"})
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("Origin", "https://evil.com")
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Header().Get("Access-Control-Allow-Origin") != "" {
		t.Error("CORS header should NOT be set for disallowed origin")
	}
}

func TestCORSMiddleware_PreflightRequest(t *testing.T) {
	middleware := CORSMiddleware([]string{"https://example.com"})
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("OPTIONS", "/test", nil)
	req.Header.Set("Origin", "https://example.com")
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Code != http.StatusNoContent {
		t.Errorf("Expected status 204 for OPTIONS, got %d", rr.Code)
	}
}

func TestSecurityHeadersMiddleware(t *testing.T) {
	middleware := SecurityHeadersMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	expectedHeaders := map[string]string{
		"X-Content-Type-Options":           "nosniff",
		"X-XSS-Protection":                 "1; mode=block",
		"X-Frame-Options":                  "DENY",
		"Strict-Transport-Security":        "max-age=31536000; includeSubDomains",
		"Referrer-Policy":                  "strict-origin-when-cross-origin",
	}

	for header, expectedValue := range expectedHeaders {
		if rr.Header().Get(header) != expectedValue {
			t.Errorf("Expected %s: %s, got %s", header, expectedValue, rr.Header().Get(header))
		}
	}
}

func TestRequestIDMiddleware_ExistingID(t *testing.T) {
	middleware := RequestIDMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	req.Header.Set("X-Request-ID", "existing-id-123")
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	if rr.Header().Get("X-Request-ID") != "existing-id-123" {
		t.Error("Should preserve existing request ID")
	}
}

func TestRequestIDMiddleware_GeneratesID(t *testing.T) {
	middleware := RequestIDMiddleware()
	handler := middleware(mockHandler(http.StatusOK, "success"))

	req := httptest.NewRequest("GET", "/test", nil)
	rr := httptest.NewRecorder()

	handler.ServeHTTP(rr, req)

	requestID := rr.Header().Get("X-Request-ID")
	if requestID == "" {
		t.Error("Should generate request ID when not provided")
	}
}

func TestWriteError(t *testing.T) {
	rr := httptest.NewRecorder()
	writeError(rr, "test error message", http.StatusBadRequest)

	if rr.Code != http.StatusBadRequest {
		t.Errorf("Expected status 400, got %d", rr.Code)
	}

	var errorResp ErrorResponse
	if err := json.NewDecoder(rr.Body).Decode(&errorResp); err != nil {
		t.Fatalf("Failed to decode error response: %v", err)
	}

	if errorResp.Message != "test error message" {
		t.Errorf("Expected 'test error message', got %s", errorResp.Message)
	}

	if errorResp.Code != http.StatusBadRequest {
		t.Errorf("Expected code 400, got %d", errorResp.Code)
	}
}
