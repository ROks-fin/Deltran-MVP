package auth

import (
	"database/sql"
	"net/http"

	"github.com/gorilla/mux"
	"github.com/redis/go-redis/v9"
)

// RegisterRoutes registers authentication routes
func RegisterRoutes(router *mux.Router, db *sql.DB, redisClient *redis.Client, jwtSecret string, corsOrigins []string) {
	// Initialize managers
	jwtManager := NewJWTManager(jwtSecret)
	totpManager := NewTOTPManager()
	sessionManager := NewSessionManager(redisClient)
	authHandler := NewAuthHandler(db, jwtManager, totpManager, sessionManager)

	// Create rate limiters
	ipRateLimiter := NewRateLimiter(redisClient, 100, 20)         // 100 req/min, burst 20
	userRateLimiter := NewRateLimiter(redisClient, 500, 50)       // 500 req/min, burst 50
	loginRateLimiter := NewRateLimiter(redisClient, 10, 5)        // 10 login attempts/min, burst 5

	// Auth subrouter
	auth := router.PathPrefix("/api/v1/auth").Subrouter()

	// Apply global middleware
	auth.Use(RequestIDMiddleware())
	auth.Use(SecurityHeadersMiddleware())
	auth.Use(CORSMiddleware(corsOrigins))

	// Public routes (no authentication required)
	public := auth.PathPrefix("").Subrouter()
	public.Use(IPRateLimitMiddleware(ipRateLimiter))

	// Login with stricter rate limiting
	public.Handle("/login", EndpointRateLimitMiddleware(loginRateLimiter, "login")(
		http.HandlerFunc(authHandler.Login),
	)).Methods("POST", "OPTIONS")

	// Refresh token
	public.HandleFunc("/refresh", authHandler.RefreshToken).Methods("POST", "OPTIONS")

	// Protected routes (authentication required)
	protected := auth.PathPrefix("").Subrouter()
	protected.Use(JWTMiddleware(jwtManager))
	protected.Use(UserRateLimitMiddleware(userRateLimiter))

	// Logout
	protected.HandleFunc("/logout", authHandler.Logout).Methods("POST", "OPTIONS")
	protected.HandleFunc("/logout-all", authHandler.LogoutAll).Methods("POST", "OPTIONS")

	// 2FA management
	protected.HandleFunc("/2fa/setup", authHandler.Setup2FA).Methods("POST", "OPTIONS")
	protected.HandleFunc("/2fa/verify", authHandler.Verify2FA).Methods("POST", "OPTIONS")
	protected.HandleFunc("/2fa/disable", authHandler.Disable2FA).Methods("DELETE", "OPTIONS")

	// Session management
	protected.HandleFunc("/sessions", authHandler.GetSessions).Methods("GET", "OPTIONS")
	protected.HandleFunc("/sessions/revoke", authHandler.RevokeSession).Methods("DELETE", "OPTIONS")
}

// RegisterUserManagementRoutes registers user management routes (admin only)
func RegisterUserManagementRoutes(router *mux.Router, db *sql.DB, redisClient *redis.Client, jwtSecret string) {
	jwtManager := NewJWTManager(jwtSecret)
	userRateLimiter := NewRateLimiter(redisClient, 200, 30)

	users := router.PathPrefix("/api/v1/users").Subrouter()

	// Apply authentication and rate limiting
	users.Use(RequestIDMiddleware())
	users.Use(SecurityHeadersMiddleware())
	users.Use(JWTMiddleware(jwtManager))
	users.Use(UserRateLimitMiddleware(userRateLimiter))

	// User CRUD (admin only)
	users.Handle("", RequirePermission(PermUserCreate)(
		http.HandlerFunc(CreateUserHandler(db)),
	)).Methods("POST", "OPTIONS")

	users.Handle("", RequirePermission(PermUserRead)(
		http.HandlerFunc(ListUsersHandler(db)),
	)).Methods("GET", "OPTIONS")

	users.Handle("/{user_id}", RequirePermission(PermUserRead)(
		http.HandlerFunc(GetUserHandler(db)),
	)).Methods("GET", "OPTIONS")

	users.Handle("/{user_id}", RequirePermission(PermUserUpdate)(
		http.HandlerFunc(UpdateUserHandler(db)),
	)).Methods("PUT", "OPTIONS")

	users.Handle("/{user_id}", RequirePermission(PermUserDelete)(
		http.HandlerFunc(DeleteUserHandler(db)),
	)).Methods("DELETE", "OPTIONS")

	// User activation/deactivation (admin only)
	users.Handle("/{user_id}/activate", RequireAdmin()(
		http.HandlerFunc(ActivateUserHandler(db)),
	)).Methods("POST", "OPTIONS")

	users.Handle("/{user_id}/deactivate", RequireAdmin()(
		http.HandlerFunc(DeactivateUserHandler(db)),
	)).Methods("POST", "OPTIONS")

	// Reset user password (admin only)
	users.Handle("/{user_id}/reset-password", RequireAdmin()(
		http.HandlerFunc(ResetPasswordHandler(db)),
	)).Methods("POST", "OPTIONS")

	// Unlock user account (admin only)
	users.Handle("/{user_id}/unlock", RequireAdmin()(
		http.HandlerFunc(UnlockUserHandler(db)),
	)).Methods("POST", "OPTIONS")
}

// Placeholder handlers (to be implemented)

func CreateUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement user creation
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func ListUsersHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement user listing with pagination
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func GetUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement get user by ID
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func UpdateUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement user update
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func DeleteUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement user soft delete
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func ActivateUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement user activation
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func DeactivateUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement user deactivation
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func ResetPasswordHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement password reset
		w.WriteHeader(http.StatusNotImplemented)
	}
}

func UnlockUserHandler(db *sql.DB) http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		// TODO: Implement account unlock
		w.WriteHeader(http.StatusNotImplemented)
	}
}

// HealthCheck returns a simple health check handler
func HealthCheck() http.HandlerFunc {
	return func(w http.ResponseWriter, r *http.Request) {
		w.Header().Set("Content-Type", "application/json")
		w.WriteHeader(http.StatusOK)
		w.Write([]byte(`{"status":"healthy","service":"auth"}`))
	}
}
