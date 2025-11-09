package config

import (
	"fmt"
	"os"
	"time"

	"gopkg.in/yaml.v3"
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
	if configPath == "" {
		configPath = "config.yaml"
	}

	data, err := os.ReadFile(configPath)
	if err != nil {
		return nil, fmt.Errorf("failed to read config file: %w", err)
	}

	var config Config
	if err := yaml.Unmarshal(data, &config); err != nil {
		return nil, fmt.Errorf("failed to parse config: %w", err)
	}

	// Override with environment variables
	config.applyEnvOverrides()

	// Validate configuration
	if err := config.Validate(); err != nil {
		return nil, fmt.Errorf("invalid configuration: %w", err)
	}

	return &config, nil
}

func (c *Config) applyEnvOverrides() {
	if port := os.Getenv("SERVICE_PORT"); port != "" {
		c.Server.Port = port
	}

	if dbHost := os.Getenv("DB_HOST"); dbHost != "" {
		c.Database.Host = dbHost
	}

	if dbPassword := os.Getenv("DB_PASSWORD"); dbPassword != "" {
		c.Database.Password = dbPassword
	}

	if redisAddr := os.Getenv("REDIS_ADDRESS"); redisAddr != "" {
		c.Redis.Address = redisAddr
	}

	if redisPassword := os.Getenv("REDIS_PASSWORD"); redisPassword != "" {
		c.Redis.Password = redisPassword
	}

	if s3Endpoint := os.Getenv("S3_ENDPOINT"); s3Endpoint != "" {
		c.S3.Endpoint = s3Endpoint
	}

	if s3AccessKey := os.Getenv("AWS_ACCESS_KEY_ID"); s3AccessKey != "" {
		c.S3.AccessKey = s3AccessKey
	}

	if s3SecretKey := os.Getenv("AWS_SECRET_ACCESS_KEY"); s3SecretKey != "" {
		c.S3.SecretKey = s3SecretKey
	}

	if natsURL := os.Getenv("NATS_URL"); natsURL != "" {
		c.NATS.URL = natsURL
	}
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

// Default configuration for development
func Default() *Config {
	return &Config{
		Server: ServerConfig{
			Port:           "8087",
			ReadTimeout:    30 * time.Second,
			WriteTimeout:   30 * time.Second,
			MaxRequestSize: 100 * 1024 * 1024, // 100MB
		},
		Database: DatabaseConfig{
			Host:           "localhost",
			Port:           5432,
			Name:           "deltran",
			User:           "deltran",
			Password:       "deltran",
			MaxConnections: 20,
			SSLMode:        "disable",
		},
		Redis: RedisConfig{
			Address:  "localhost:6379",
			Password: "",
			DB:       3,
			CacheTTL: 5 * time.Minute,
		},
		S3: S3Config{
			Endpoint:  "http://localhost:9000",
			Bucket:    "deltran-reports",
			AccessKey: "minioadmin",
			SecretKey: "minioadmin",
			Region:    "us-east-1",
			UseSSL:    false,
		},
		NATS: NATSConfig{
			URL:     "nats://localhost:4222",
			Subject: "reports.requests",
		},
		Scheduler: SchedulerConfig{
			Enabled:       true,
			Timezone:      "UTC",
			MaxConcurrent: 5,
		},
		Reports: ReportsConfig{
			MaxRowsExcel: 1048576,
			MaxRowsCSV:   10000000,
			Timeout:      5 * time.Minute,
			TempDir:      "/tmp/reports",
		},
		Monitoring: MonitoringConfig{
			PrometheusEnabled: true,
			MetricsPort:       "9097",
		},
	}
}
