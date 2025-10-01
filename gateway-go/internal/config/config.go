// Configuration management
package config

import (
	"fmt"
	"os"
	"time"

	"gopkg.in/yaml.v3"
)

// Config represents the gateway configuration
type Config struct {
	Version string       `yaml:"version"`
	Server  ServerConfig `yaml:"server"`
	Ledger  LedgerConfig `yaml:"ledger"`
	Limits  LimitsConfig `yaml:"limits"`
	Banks   []BankConfig `yaml:"banks"`
}

// ServerConfig represents server settings
type ServerConfig struct {
	GRPCAddr       string `yaml:"grpc_addr"`
	HTTPAddr       string `yaml:"http_addr"`
	MaxMessageSize int    `yaml:"max_message_size"`
}

// LedgerConfig represents ledger client settings
type LedgerConfig struct {
	Addr            string        `yaml:"addr"`
	ConnectTimeout  time.Duration `yaml:"connect_timeout"`
	RequestTimeout  time.Duration `yaml:"request_timeout"`
	MaxRetries      int           `yaml:"max_retries"`
	RetryBackoff    time.Duration `yaml:"retry_backoff"`
	EnableBatching  bool          `yaml:"enable_batching"`
	BatchSize       int           `yaml:"batch_size"`
	BatchTimeout    time.Duration `yaml:"batch_timeout"`
}

// LimitsConfig represents rate limiting settings
type LimitsConfig struct {
	MaxPaymentsPerSecond int           `yaml:"max_payments_per_second"`
	MaxPaymentsPerMinute int           `yaml:"max_payments_per_minute"`
	MaxPaymentAmount     string        `yaml:"max_payment_amount"` // Decimal string
	MinPaymentAmount     string        `yaml:"min_payment_amount"` // Decimal string
	WorkerPoolSize       int           `yaml:"worker_pool_size"`
	QueueSize            int           `yaml:"queue_size"`
	RequestTimeout       time.Duration `yaml:"request_timeout"`
}

// BankConfig represents bank connector settings
type BankConfig struct {
	BIC              string   `yaml:"bic"`
	Name             string   `yaml:"name"`
	SupportedCurrencies []string `yaml:"supported_currencies"`
	Endpoint         string   `yaml:"endpoint"`
	ConnectorType    string   `yaml:"connector_type"` // "iso20022", "swift", "api"
	Enabled          bool     `yaml:"enabled"`
}

// Default returns default configuration
func Default() *Config {
	return &Config{
		Version: "1.0.0",
		Server: ServerConfig{
			GRPCAddr:       "0.0.0.0:50052",
			HTTPAddr:       "0.0.0.0:8080",
			MaxMessageSize: 4 * 1024 * 1024, // 4MB
		},
		Ledger: LedgerConfig{
			Addr:            "127.0.0.1:50051",
			ConnectTimeout:  10 * time.Second,
			RequestTimeout:  5 * time.Second,
			MaxRetries:      3,
			RetryBackoff:    100 * time.Millisecond,
			EnableBatching:  true,
			BatchSize:       100,
			BatchTimeout:    10 * time.Millisecond,
		},
		Limits: LimitsConfig{
			MaxPaymentsPerSecond: 1000,
			MaxPaymentsPerMinute: 50000,
			MaxPaymentAmount:     "1000000.00", // $1M
			MinPaymentAmount:     "0.01",       // $0.01
			WorkerPoolSize:       1000,
			QueueSize:            10000,
			RequestTimeout:       30 * time.Second,
		},
		Banks: []BankConfig{
			{
				BIC:              "CHASUS33",
				Name:             "JPMorgan Chase",
				SupportedCurrencies: []string{"USD", "EUR"},
				Endpoint:         "https://chase.example.com/api",
				ConnectorType:    "api",
				Enabled:          true,
			},
		},
	}
}

// Load loads configuration from file or environment
func Load() (*Config, error) {
	// Check for config file path
	configPath := os.Getenv("GATEWAY_CONFIG")
	if configPath == "" {
		configPath = "config.yaml"
	}

	// Try to load from file
	if _, err := os.Stat(configPath); err == nil {
		return loadFromFile(configPath)
	}

	// Fall back to defaults with env overrides
	cfg := Default()
	applyEnvOverrides(cfg)
	return cfg, nil
}

// loadFromFile loads config from YAML file
func loadFromFile(path string) (*Config, error) {
	data, err := os.ReadFile(path)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	cfg := &Config{}
	if err := yaml.Unmarshal(data, cfg); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	applyEnvOverrides(cfg)
	return cfg, nil
}

// applyEnvOverrides applies environment variable overrides
func applyEnvOverrides(cfg *Config) {
	if addr := os.Getenv("GATEWAY_GRPC_ADDR"); addr != "" {
		cfg.Server.GRPCAddr = addr
	}
	if addr := os.Getenv("GATEWAY_HTTP_ADDR"); addr != "" {
		cfg.Server.HTTPAddr = addr
	}
	if addr := os.Getenv("GATEWAY_LEDGER_ADDR"); addr != "" {
		cfg.Ledger.Addr = addr
	}
}

// Validate validates the configuration
func (c *Config) Validate() error {
	if c.Server.GRPCAddr == "" {
		return fmt.Errorf("server.grpc_addr is required")
	}
	if c.Server.HTTPAddr == "" {
		return fmt.Errorf("server.http_addr is required")
	}
	if c.Ledger.Addr == "" {
		return fmt.Errorf("ledger.addr is required")
	}
	if c.Limits.WorkerPoolSize <= 0 {
		return fmt.Errorf("limits.worker_pool_size must be positive")
	}
	return nil
}