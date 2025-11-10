package config

import (
	"fmt"
	"os"
	"time"
)

// Config holds all configuration for the gateway
type Config struct {
	Server    ServerConfig
	Services  ServiceEndpoints
	Auth      AuthConfig
	RateLimit RateLimitConfig
	CircuitBreaker CircuitBreakerConfig
}

// ServerConfig holds server configuration
type ServerConfig struct {
	Port            string
	ReadTimeout     time.Duration
	WriteTimeout    time.Duration
	IdleTimeout     time.Duration
	ShutdownTimeout time.Duration
}

// ServiceEndpoints holds URLs for all backend services
type ServiceEndpoints struct {
	TokenEngine      string
	ObligationEngine string
	LiquidityRouter  string
	RiskEngine       string
	ComplianceEngine string
	ClearingEngine   string
	SettlementEngine string
	NotificationEngine string
	ReportingEngine  string
}

// AuthConfig holds authentication configuration
type AuthConfig struct {
	JWTSecret     string
	JWTExpiration time.Duration
	Issuer        string
}

// RateLimitConfig holds rate limiting configuration
type RateLimitConfig struct {
	RequestsPerMinute int
	BurstSize         int
}

// CircuitBreakerConfig holds circuit breaker configuration
type CircuitBreakerConfig struct {
	Timeout               time.Duration
	MaxConcurrentRequests int
	ErrorThreshold        int
	SleepWindow           time.Duration
}

// Load loads configuration from environment variables
func Load() *Config {
	return &Config{
		Server: ServerConfig{
			Port:            getEnv("GATEWAY_PORT", "8080"),
			ReadTimeout:     getDurationEnv("GATEWAY_READ_TIMEOUT", 15*time.Second),
			WriteTimeout:    getDurationEnv("GATEWAY_WRITE_TIMEOUT", 15*time.Second),
			IdleTimeout:     getDurationEnv("GATEWAY_IDLE_TIMEOUT", 60*time.Second),
			ShutdownTimeout: getDurationEnv("GATEWAY_SHUTDOWN_TIMEOUT", 30*time.Second),
		},
		Services: ServiceEndpoints{
			TokenEngine:      getEnv("TOKEN_ENGINE_URL", "http://token-engine:8081"),
			ObligationEngine: getEnv("OBLIGATION_ENGINE_URL", "http://obligation-engine:8082"),
			LiquidityRouter:  getEnv("LIQUIDITY_ROUTER_URL", "http://liquidity-router:8083"),
			RiskEngine:       getEnv("RISK_ENGINE_URL", "http://risk-engine:8084"),
			ComplianceEngine: getEnv("COMPLIANCE_ENGINE_URL", "http://compliance-engine:8086"),
			ClearingEngine:   getEnv("CLEARING_ENGINE_URL", "http://clearing-engine:8085"),
			SettlementEngine: getEnv("SETTLEMENT_ENGINE_URL", "http://settlement-engine:8088"),
			NotificationEngine: getEnv("NOTIFICATION_ENGINE_URL", "http://notification-engine:8089"),
			ReportingEngine:  getEnv("REPORTING_ENGINE_URL", "http://reporting-engine:8087"),
		},
		Auth: AuthConfig{
			JWTSecret:     getEnv("JWT_SECRET", "deltran-secret-key-change-in-production"),
			JWTExpiration: getDurationEnv("JWT_EXPIRATION", 24*time.Hour),
			Issuer:        getEnv("JWT_ISSUER", "deltran-gateway"),
		},
		RateLimit: RateLimitConfig{
			RequestsPerMinute: getIntEnv("RATE_LIMIT_RPM", 100),
			BurstSize:         getIntEnv("RATE_LIMIT_BURST", 20),
		},
		CircuitBreaker: CircuitBreakerConfig{
			Timeout:               getDurationEnv("CB_TIMEOUT", 5*time.Second),
			MaxConcurrentRequests: getIntEnv("CB_MAX_CONCURRENT", 100),
			ErrorThreshold:        getIntEnv("CB_ERROR_THRESHOLD", 50),
			SleepWindow:           getDurationEnv("CB_SLEEP_WINDOW", 10*time.Second),
		},
	}
}

// Helper functions
func getEnv(key, defaultValue string) string {
	if value := os.Getenv(key); value != "" {
		return value
	}
	return defaultValue
}

func getIntEnv(key string, defaultValue int) int {
	if value := os.Getenv(key); value != "" {
		var intValue int
		if _, err := fmt.Sscanf(value, "%d", &intValue); err == nil {
			return intValue
		}
	}
	return defaultValue
}

func getDurationEnv(key string, defaultValue time.Duration) time.Duration {
	if value := os.Getenv(key); value != "" {
		if duration, err := time.ParseDuration(value); err == nil {
			return duration
		}
	}
	return defaultValue
}
