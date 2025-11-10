package config

import (
	"fmt"
	"time"

	"github.com/spf13/viper"
)

type Config struct {
	Server     ServerConfig     `yaml:"server"`
	Database   DatabaseConfig   `yaml:"database"`
	Redis      RedisConfig      `yaml:"redis"`
	S3         S3Config         `yaml:"s3"`
	NATS       NATSConfig       `yaml:"nats"`
	Scheduler  SchedulerConfig  `yaml:"scheduler"`
	Reports    ReportsConfig    `yaml:"reports"`
	Monitoring MonitoringConfig `yaml:"monitoring"`
}

type ServerConfig struct {
	Port           string        `yaml:"port"`
	ReadTimeout    time.Duration `yaml:"read_timeout"`
	WriteTimeout   time.Duration `yaml:"write_timeout"`
	MaxRequestSize int64         `yaml:"max_request_size"`
}

type DatabaseConfig struct {
	Host           string `yaml:"host"`
	Port           int    `yaml:"port"`
	Name           string `yaml:"name"`
	User           string `yaml:"user"`
	Password       string `yaml:"password"`
	MaxConnections int    `yaml:"max_connections"`
	SSLMode        string `yaml:"ssl_mode"`
}

type RedisConfig struct {
	Address  string        `yaml:"address"`
	Password string        `yaml:"password"`
	DB       int           `yaml:"db"`
	CacheTTL time.Duration `yaml:"cache_ttl"`
}

type S3Config struct {
	Endpoint  string `yaml:"endpoint"`
	Bucket    string `yaml:"bucket"`
	AccessKey string `yaml:"access_key"`
	SecretKey string `yaml:"secret_key"`
	Region    string `yaml:"region"`
	UseSSL    bool   `yaml:"use_ssl"`
}

type NATSConfig struct {
	URL     string `yaml:"url"`
	Subject string `yaml:"subject"`
}

type SchedulerConfig struct {
	Enabled       bool   `yaml:"enabled"`
	Timezone      string `yaml:"timezone"`
	MaxConcurrent int    `yaml:"max_concurrent"`
}

type ReportsConfig struct {
	MaxRowsExcel int           `yaml:"max_rows_excel"`
	MaxRowsCSV   int           `yaml:"max_rows_csv"`
	Timeout      time.Duration `yaml:"timeout"`
	TempDir      string        `yaml:"temp_dir"`
}

type MonitoringConfig struct {
	PrometheusEnabled bool   `yaml:"prometheus_enabled"`
	MetricsPort       string `yaml:"metrics_port"`
}

func Load(configPath string) (*Config, error) {
	// Set defaults first
	setDefaults()

	viper.AutomaticEnv()

	// Try to load config file if provided, but don't fail if it doesn't exist
	if configPath != "" {
		viper.SetConfigFile(configPath)
		viper.SetConfigType("yaml")

		if err := viper.ReadInConfig(); err != nil {
			// Config file not required - just use defaults and env vars
			fmt.Printf("Config file not found, using defaults and environment variables: %v\n", err)
		}
	}

	var config Config
	if err := viper.Unmarshal(&config); err != nil {
		return nil, fmt.Errorf("failed to unmarshal config: %w", err)
	}

	// Validate configuration
	if err := config.Validate(); err != nil {
		return nil, fmt.Errorf("invalid configuration: %w", err)
	}

	return &config, nil
}

func setDefaults() {
	// Server defaults
	viper.SetDefault("server.port", "8088")
	viper.SetDefault("server.read_timeout", "30s")
	viper.SetDefault("server.write_timeout", "30s")
	viper.SetDefault("server.max_request_size", 104857600) // 100MB

	// Database defaults with Docker service names
	viper.SetDefault("database.host", "postgres")
	viper.SetDefault("database.port", 5432)
	viper.SetDefault("database.name", "deltran")
	viper.SetDefault("database.user", "deltran")
	viper.SetDefault("database.password", "deltran_secure_pass_2024")
	viper.SetDefault("database.max_connections", 20)
	viper.SetDefault("database.ssl_mode", "disable")

	// Redis defaults with Docker service names
	viper.SetDefault("redis.address", "redis:6379")
	viper.SetDefault("redis.password", "")
	viper.SetDefault("redis.db", 3)
	viper.SetDefault("redis.cache_ttl", "5m")

	// S3/MinIO defaults with Docker service names
	viper.SetDefault("s3.endpoint", "http://minio:9000")
	viper.SetDefault("s3.bucket", "deltran-reports")
	viper.SetDefault("s3.access_key", "minioadmin")
	viper.SetDefault("s3.secret_key", "minioadmin")
	viper.SetDefault("s3.region", "us-east-1")
	viper.SetDefault("s3.use_ssl", false)

	// NATS defaults with Docker service names
	viper.SetDefault("nats.url", "nats://nats:4222")
	viper.SetDefault("nats.subject", "reports.requests")

	// Scheduler defaults
	viper.SetDefault("scheduler.enabled", true)
	viper.SetDefault("scheduler.timezone", "UTC")
	viper.SetDefault("scheduler.max_concurrent", 5)

	// Reports defaults
	viper.SetDefault("reports.max_rows_excel", 1048576)
	viper.SetDefault("reports.max_rows_csv", 10000000)
	viper.SetDefault("reports.timeout", "5m")
	viper.SetDefault("reports.temp_dir", "/tmp/reports")

	// Monitoring defaults
	viper.SetDefault("monitoring.prometheus_enabled", true)
	viper.SetDefault("monitoring.metrics_port", "9097")
}

func (c *Config) Validate() error {
	if c.Server.Port == "" {
		return fmt.Errorf("server port is required")
	}

	if c.Database.Host == "" {
		return fmt.Errorf("database host is required")
	}

	if c.Database.Name == "" {
		return fmt.Errorf("database name is required")
	}

	if c.S3.Bucket == "" {
		return fmt.Errorf("S3 bucket is required")
	}

	return nil
}

func (c *Config) DatabaseDSN() string {
	sslMode := c.Database.SSLMode
	if sslMode == "" {
		sslMode = "disable"
	}

	return fmt.Sprintf(
		"host=%s port=%d user=%s password=%s dbname=%s sslmode=%s",
		c.Database.Host,
		c.Database.Port,
		c.Database.User,
		c.Database.Password,
		c.Database.Name,
		sslMode,
	)
}

