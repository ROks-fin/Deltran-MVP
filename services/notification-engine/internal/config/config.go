package config

import (
	"fmt"
	"time"

	"github.com/spf13/viper"
)

type Config struct {
	Server      ServerConfig      `mapstructure:"server"`
	NATS        NATSConfig        `mapstructure:"nats"`
	Database    DatabaseConfig    `mapstructure:"database"`
	Redis       RedisConfig       `mapstructure:"redis"`
	Email       EmailConfig       `mapstructure:"email"`
	SMS         SMSConfig         `mapstructure:"sms"`
	WebSocket   WebSocketConfig   `mapstructure:"websocket"`
	RateLimit   RateLimitConfig   `mapstructure:"rate_limiting"`
	Monitoring  MonitoringConfig  `mapstructure:"monitoring"`
}

type ServerConfig struct {
	HTTPPort     int           `mapstructure:"http_port"`
	WSPort       int           `mapstructure:"ws_port"`
	ReadTimeout  time.Duration `mapstructure:"read_timeout"`
	WriteTimeout time.Duration `mapstructure:"write_timeout"`
	IdleTimeout  time.Duration `mapstructure:"idle_timeout"`
}

type NATSConfig struct {
	URL       string `mapstructure:"url"`
	ClusterID string `mapstructure:"cluster_id"`
	ClientID  string `mapstructure:"client_id"`
}

type DatabaseConfig struct {
	Host           string `mapstructure:"host"`
	Port           int    `mapstructure:"port"`
	Name           string `mapstructure:"name"`
	User           string `mapstructure:"user"`
	Password       string `mapstructure:"password"`
	MaxConnections int    `mapstructure:"max_connections"`
	SSLMode        string `mapstructure:"ssl_mode"`
}

type RedisConfig struct {
	Address  string `mapstructure:"address"`
	Password string `mapstructure:"password"`
	DB       int    `mapstructure:"db"`
}

type EmailConfig struct {
	Provider    string `mapstructure:"provider"`
	APIKey      string `mapstructure:"api_key"`
	FromAddress string `mapstructure:"from_address"`
	FromName    string `mapstructure:"from_name"`
	SMTPHost    string `mapstructure:"smtp_host"`
	SMTPPort    int    `mapstructure:"smtp_port"`
}

type SMSConfig struct {
	Provider    string `mapstructure:"provider"`
	AccountSID  string `mapstructure:"account_sid"`
	AuthToken   string `mapstructure:"auth_token"`
	FromNumber  string `mapstructure:"from_number"`
	MockMode    bool   `mapstructure:"mock_mode"`
}

type WebSocketConfig struct {
	PingInterval   time.Duration `mapstructure:"ping_interval"`
	PongWait       time.Duration `mapstructure:"pong_wait"`
	WriteWait      time.Duration `mapstructure:"write_wait"`
	MaxMessageSize int64         `mapstructure:"max_message_size"`
}

type RateLimitConfig struct {
	EmailPerHour int `mapstructure:"email_per_hour"`
	SMSPerHour   int `mapstructure:"sms_per_hour"`
	PushPerHour  int `mapstructure:"push_per_hour"`
}

type MonitoringConfig struct {
	PrometheusEnabled bool `mapstructure:"prometheus_enabled"`
	MetricsPort       int  `mapstructure:"metrics_port"`
}

// Load loads configuration from file and environment
func Load(configPath string) (*Config, error) {
	viper.SetConfigFile(configPath)
	viper.SetConfigType("yaml")
	viper.AutomaticEnv()

	// Set defaults
	setDefaults()

	if err := viper.ReadInConfig(); err != nil {
		return nil, fmt.Errorf("failed to read config: %w", err)
	}

	var config Config
	if err := viper.Unmarshal(&config); err != nil {
		return nil, fmt.Errorf("failed to unmarshal config: %w", err)
	}

	return &config, nil
}

func setDefaults() {
	viper.SetDefault("server.http_port", 8089)
	viper.SetDefault("server.ws_port", 8090)
	viper.SetDefault("server.read_timeout", "10s")
	viper.SetDefault("server.write_timeout", "10s")
	viper.SetDefault("server.idle_timeout", "120s")

	viper.SetDefault("nats.url", "nats://localhost:4222")
	viper.SetDefault("nats.cluster_id", "deltran-cluster")
	viper.SetDefault("nats.client_id", "notification-engine")

	viper.SetDefault("database.host", "localhost")
	viper.SetDefault("database.port", 5432)
	viper.SetDefault("database.name", "deltran")
	viper.SetDefault("database.user", "deltran")
	viper.SetDefault("database.max_connections", 25)
	viper.SetDefault("database.ssl_mode", "disable")

	viper.SetDefault("redis.address", "localhost:6379")
	viper.SetDefault("redis.db", 2)

	viper.SetDefault("email.provider", "smtp")
	viper.SetDefault("email.from_address", "noreply@deltran.com")
	viper.SetDefault("email.from_name", "DelTran")

	viper.SetDefault("sms.provider", "twilio")
	viper.SetDefault("sms.mock_mode", true)

	viper.SetDefault("websocket.ping_interval", "30s")
	viper.SetDefault("websocket.pong_wait", "60s")
	viper.SetDefault("websocket.write_wait", "10s")
	viper.SetDefault("websocket.max_message_size", 524288) // 512KB

	viper.SetDefault("rate_limiting.email_per_hour", 10)
	viper.SetDefault("rate_limiting.sms_per_hour", 5)
	viper.SetDefault("rate_limiting.push_per_hour", 50)

	viper.SetDefault("monitoring.prometheus_enabled", true)
	viper.SetDefault("monitoring.metrics_port", 9095)
}
