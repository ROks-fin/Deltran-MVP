package middleware

import (
	"encoding/json"
	"fmt"
	"net/http"
	"strings"

	"github.com/golang-jwt/jwt/v5"
	"github.com/gorilla/mux"
)

// Claims represents JWT token claims
type Claims struct {
	Sub         string   `json:"sub"`
	Role        string   `json:"role"`
	Permissions []string `json:"permissions"`
	jwt.RegisteredClaims
}

// JWTConfig holds JWT configuration
type JWTConfig struct {
	SecretKey []byte
}

// NewJWTConfig creates a new JWT configuration
func NewJWTConfig(secretKey string) *JWTConfig {
	return &JWTConfig{
		SecretKey: []byte(secretKey),
	}
}

// AuthMiddleware validates JWT tokens
func (jc *JWTConfig) AuthMiddleware(next http.Handler) http.Handler {
	return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
		// Skip authentication for health check and public endpoints
		if r.URL.Path == "/health" ||
		   r.URL.Path == "/api/v1/banks" ||
		   r.URL.Path == "/api/v1/corridors" {
			next.ServeHTTP(w, r)
			return
		}

		// Extract token from Authorization header
		authHeader := r.Header.Get("Authorization")
		if authHeader == "" {
			respondWithError(w, http.StatusUnauthorized, "Missing authorization header")
			return
		}

		// Check Bearer token format
		bearerToken := strings.Split(authHeader, " ")
		if len(bearerToken) != 2 || bearerToken[0] != "Bearer" {
			respondWithError(w, http.StatusUnauthorized, "Invalid authorization format")
			return
		}

		tokenString := bearerToken[1]

		// Parse and validate token
		claims := &Claims{}
		token, err := jwt.ParseWithClaims(tokenString, claims, func(token *jwt.Token) (interface{}, error) {
			// Validate signing method
			if _, ok := token.Method.(*jwt.SigningMethodHMAC); !ok {
				return nil, fmt.Errorf("unexpected signing method: %v", token.Header["alg"])
			}
			return jc.SecretKey, nil
		})

		if err != nil {
			respondWithError(w, http.StatusUnauthorized, "Invalid token: "+err.Error())
			return
		}

		if !token.Valid {
			respondWithError(w, http.StatusUnauthorized, "Token is invalid")
			return
		}

		// Add claims to request context
		ctx := r.Context()
		vars := mux.Vars(r)
		if vars == nil {
			vars = make(map[string]string)
		}
		vars["user_id"] = claims.Sub
		vars["user_role"] = claims.Role

		// Store in request header for downstream services
		r.Header.Set("X-User-ID", claims.Sub)
		r.Header.Set("X-User-Role", claims.Role)
		r.Header.Set("X-User-Permissions", strings.Join(claims.Permissions, ","))

		// Continue to next handler
		next.ServeHTTP(w, r.WithContext(ctx))
	})
}

// RequirePermission checks if user has required permission
func RequirePermission(permission string) mux.MiddlewareFunc {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			// Get permissions from header
			permissionsHeader := r.Header.Get("X-User-Permissions")
			if permissionsHeader == "" {
				respondWithError(w, http.StatusForbidden, "No permissions found")
				return
			}

			permissions := strings.Split(permissionsHeader, ",")

			// Check if required permission exists
			hasPermission := false
			for _, p := range permissions {
				if p == permission || p == "admin:all" {
					hasPermission = true
					break
				}
			}

			if !hasPermission {
				respondWithError(w, http.StatusForbidden, "Insufficient permissions")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

// RequireRole checks if user has required role
func RequireRole(allowedRoles ...string) mux.MiddlewareFunc {
	return func(next http.Handler) http.Handler {
		return http.HandlerFunc(func(w http.ResponseWriter, r *http.Request) {
			userRole := r.Header.Get("X-User-Role")
			if userRole == "" {
				respondWithError(w, http.StatusForbidden, "No role found")
				return
			}

			// Check if user role is in allowed roles
			hasRole := false
			for _, role := range allowedRoles {
				if userRole == role {
					hasRole = true
					break
				}
			}

			if !hasRole {
				respondWithError(w, http.StatusForbidden, "Insufficient role")
				return
			}

			next.ServeHTTP(w, r)
		})
	}
}

func respondWithError(w http.ResponseWriter, code int, message string) {
	w.Header().Set("Content-Type", "application/json")
	w.WriteHeader(code)
	json.NewEncoder(w).Encode(map[string]string{
		"error": message,
		"code":  fmt.Sprintf("%d", code),
	})
}
