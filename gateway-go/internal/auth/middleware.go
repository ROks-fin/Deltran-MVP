package auth

import (
	"context"
	"encoding/json"
	"net/http"
	"strings"
)

// contextKey is a custom type for context keys to avoid collisions
type contextKey string

const (
	// ClaimsContextKey is the key for storing claims in context
	ClaimsContextKey contextKey = "claims"
	// UserIDContextKey is the key for storing user ID in context
	UserIDContextKey contextKey = "user_id"
)

// ErrorResponse represents error response
type ErrorResponse struct {
	Error   string `json:"error"`
	Message string `json:"message"`
	Code    int    `json:"code"`
}

// JWTMiddleware creates HTTP middleware for JWT authentication
func JWTMiddleware(jwtManager *JWTManager) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Extract token from Authorization header
			authHeader := r.Header.Get("Authorization")
			if authHeader == "" {
				writeError(w, "missing authorization header", http.StatusUnauthorized)
				return
			}

			tokenString, err := ExtractBearerToken(authHeader)
			if err != nil {
				writeError(w, err.Error(), http.StatusUnauthorized)
				return
			}

			// Validate token
			claims, err := jwtManager.ValidateAccessToken(tokenString)
			if err != nil {
				if err == ErrExpiredToken {
					writeError(w, "token expired", http.StatusUnauthorized)
				} else if err == ErrInvalidSignature {
					writeError(w, "invalid token signature", http.StatusUnauthorized)
				} else {
					writeError(w, "invalid token", http.StatusUnauthorized)
				}
				return
			}

			// Add claims to context
			ctx := context.WithValue(r.Context(), ClaimsContextKey, claims)
			ctx = context.WithValue(ctx, UserIDContextKey, claims.UserID)

			// Call next handler
			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}

// OptionalJWTMiddleware creates middleware that allows requests with or without JWT
func OptionalJWTMiddleware(jwtManager *JWTManager) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			authHeader := r.Header.Get("Authorization")
			if authHeader != "" {
				tokenString, err := ExtractBearerToken(authHeader)
				if err == nil {
					claims, err := jwtManager.ValidateAccessToken(tokenString)
					if err == nil {
						ctx := context.WithValue(r.Context(), ClaimsContextKey, claims)
						ctx = context.WithValue(ctx, UserIDContextKey, claims.UserID)
						r = r.WithContext(ctx)
					}
				}
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequirePermission creates middleware that requires specific permission
func RequirePermission(perm Permission) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
			if !ok {
				writeError(w, "authentication required", http.StatusUnauthorized)
				return
			}

			if !claims.HasPermission(perm) {
				writeError(w, "insufficient permissions", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequireAnyPermission creates middleware that requires any of the specified permissions
func RequireAnyPermission(perms ...Permission) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
			if !ok {
				writeError(w, "authentication required", http.StatusUnauthorized)
				return
			}

			if !claims.HasAnyPermission(perms...) {
				writeError(w, "insufficient permissions", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequireAllPermissions creates middleware that requires all of the specified permissions
func RequireAllPermissions(perms ...Permission) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
			if !ok {
				writeError(w, "authentication required", http.StatusUnauthorized)
				return
			}

			if !claims.HasAllPermissions(perms...) {
				writeError(w, "insufficient permissions", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequireRole creates middleware that requires specific role
func RequireRole(role UserRole) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
			if !ok {
				writeError(w, "authentication required", http.StatusUnauthorized)
				return
			}

			if claims.Role != role {
				writeError(w, "insufficient role", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequireAdmin creates middleware that requires admin role
func RequireAdmin() func(http.Handler) http.Handler {
	return RequireRole(RoleAdmin)
}

// RequireOperator creates middleware that requires operator or admin role
func RequireOperator() func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
			if !ok {
				writeError(w, "authentication required", http.StatusUnauthorized)
				return
			}

			if !claims.IsOperator() {
				writeError(w, "operator role required", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RestrictToBankMiddleware restricts access to resources of user's bank only
func RestrictToBankMiddleware() func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
			if !ok {
				writeError(w, "authentication required", http.StatusUnauthorized)
				return
			}

			// Admins can access all banks
			if claims.IsAdmin() {
				next.ServeHTTP(w, r)
				return
			}

			// Check if user has bank ID
			if claims.BankID == "" {
				writeError(w, "user not associated with any bank", http.StatusForbidden)
				return
			}

			// Extract bank ID from request (query param or path param)
			requestBankID := r.URL.Query().Get("bank_id")
			if requestBankID == "" {
				// Try to extract from path (e.g., /api/banks/{bank_id}/...)
				pathParts := strings.Split(r.URL.Path, "/")
				for i, part := range pathParts {
					if part == "banks" && i+1 < len(pathParts) {
						requestBankID = pathParts[i+1]
						break
					}
				}
			}

			// If bank ID specified in request, verify it matches user's bank
			if requestBankID != "" && requestBankID != claims.BankID {
				writeError(w, "access denied to other bank's resources", http.StatusForbidden)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// GetClaims extracts claims from request context
func GetClaims(r *http.Request) (*Claims, bool) {
	claims, ok := r.Context().Value(ClaimsContextKey).(*Claims)
	return claims, ok
}

// GetUserID extracts user ID from request context
func GetUserID(r *http.Request) (string, bool) {
	userID, ok := r.Context().Value(UserIDContextKey).(string)
	return userID, ok
}

// MustGetClaims extracts claims from request context or panics
func MustGetClaims(r *http.Request) *Claims {
	claims, ok := GetClaims(r)
	if !ok {
		panic("claims not found in context")
	}
	return claims
}

// MustGetUserID extracts user ID from request context or panics
func MustGetUserID(r *http.Request) string {
	userID, ok := GetUserID(r)
	if !ok {
		panic("user ID not found in context")
	}
	return userID
}

// writeError writes JSON error response
func writeError(w http.ResponseWriter, message string, statusCode int) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(statusCode)

	errResp := ErrorResponse{
		Error:   http.StatusText(statusCode),
		Message: message,
		Code:    statusCode,
	}

	json.NewEncoder(w).Encode(errResp)
}

// CORS middleware
func CORSMiddleware(allowedOrigins []string) func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			origin := r.Header.Get("Origin")

			// Check if origin is allowed
			allowed := false
			for _, allowedOrigin := range allowedOrigins {
				if allowedOrigin == "*" || allowedOrigin == origin {
					allowed = true
					break
				}
			}

			if allowed {
				w.Header().Set("Access-Control-Allow-Origin", origin)
				w.Header().Set("Access-Control-Allow-Methods", "GET, POST, PUT, DELETE, OPTIONS, PATCH")
				w.Header().Set("Access-Control-Allow-Headers", "Content-Type, Authorization, X-Requested-With, X-Request-ID")
				w.Header().Set("Access-Control-Allow-Credentials", "true")
				w.Header().Set("Access-Control-Max-Age", "86400") // 24 hours
			}

			// Handle preflight request
			if r.Method == "OPTIONS" {
				w.WriteHeader(http.StatusNoContent)
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// SecurityHeadersMiddleware adds security headers
func SecurityHeadersMiddleware() func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Prevent MIME type sniffing
			w.Header().Set("X-Content-Type-Options", "nosniff")

			// Enable XSS protection
			w.Header().Set("X-XSS-Protection", "1; mode=block")

			// Prevent clickjacking
			w.Header().Set("X-Frame-Options", "DENY")

			// Enforce HTTPS
			w.Header().Set("Strict-Transport-Security", "max-age=31536000; includeSubDomains")

			// Content Security Policy
			w.Header().Set("Content-Security-Policy", "default-src 'self'; script-src 'self'; style-src 'self' 'unsafe-inline'; img-src 'self' data:; font-src 'self'; connect-src 'self'; frame-ancestors 'none'")

			// Referrer Policy
			w.Header().Set("Referrer-Policy", "strict-origin-when-cross-origin")

			// Permissions Policy (formerly Feature Policy)
			w.Header().Set("Permissions-Policy", "geolocation=(), microphone=(), camera=()")

			next.ServeHTTP(w, r)
		})
	}
}

// RequestIDMiddleware adds unique request ID to context
func RequestIDMiddleware() func(http.Handler) http.Handler {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Check if request ID already exists
			requestID := r.Header.Get("X-Request-ID")
			if requestID == "" {
				requestID = generateTokenID()
			}

			// Add to response header
			w.Header().Set("X-Request-ID", requestID)

			// Add to context
			ctx := context.WithValue(r.Context(), "request_id", requestID)

			next.ServeHTTP(w, r.WithContext(ctx))
		})
	}
}
