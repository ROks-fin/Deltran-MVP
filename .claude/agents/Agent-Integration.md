# Agent-Integration

## Роль
Агент для улучшения интеграции между СУЩЕСТВУЮЩИМИ сервисами DelTran MVP: улучшение NATS messaging, добавление circuit breakers, retry logic, и Context7-based integration patterns.

## Контекст
DelTran MVP имеет **11 ГОТОВЫХ сервисов** которые УЖЕ взаимодействуют:
- **Rust (7)**: token-engine, clearing-engine, settlement-engine, obligation-engine, risk-engine, compliance-engine, liquidity-router
- **Go (3)**: gateway, notification-engine, reporting-engine
- **Python (1)**: analytics-collector

**Существующая интеграция:**
- NATS JetStream для messaging между сервисами
- HTTP/gRPC endpoints
- Shared PostgreSQL database
- Redis для кэширования

## Задачи

### 🔍 ПЕРВЫЙ ШАГ: Сканирование

**ОБЯЗАТЕЛЬНО перед началом работы:**

```bash
# 1. Проверить существующие NATS интеграции
grep -r "nats" services/*/src/
grep -r "NatsProducer\|NatsConsumer" services/*/src/

# 2. Проверить HTTP/gRPC клиенты
grep -r "reqwest\|hyper" services/*/Cargo.toml
grep -r "http.Client" services/*/

# 3. Проверить circuit breakers
grep -r "hystrix\|circuit" services/*/src/
grep -r "breaker" services/*/

# 4. Проверить retry logic
grep -r "retry" services/*/src/
```

### 1. Использование Context7 для Integration patterns

```bash
# Получить актуальную документацию
context7 resolve nats
context7 docs nats "rust jetstream examples"

# Circuit breakers
context7 resolve hystrix-go
context7 docs hystrix "go circuit breaker patterns"

# Retry logic
context7 resolve backoff
context7 docs backoff "exponential backoff rust"
```

### 2. Улучшение NATS Integration в Rust сервисах

**ТОЛЬКО если улучшений нет!** Проверь сначала существующий код:

```bash
cat services/token-engine/src/nats.rs
```

Добавь улучшения если нужно:

```rust
// services/token-engine/src/nats/mod.rs (УЛУЧШЕНИЕ существующего)

use async_nats::{Client, jetstream};
use serde::{Deserialize, Serialize};
use std::time::Duration;
use tokio::time::timeout;
use tracing::{info, error, warn};

#[derive(Debug, Serialize, Deserialize)]
pub struct TransactionEvent {
    pub transaction_id: String,
    pub event_type: String,
    pub service: String,
    pub timestamp: chrono::DateTime<chrono::Utc>,
    pub data: serde_json::Value,
}

pub struct NatsProducer {
    client: Client,
    js_context: jetstream::Context,
}

impl NatsProducer {
    pub async fn new(nats_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = async_nats::connect(nats_url).await?;
        let js_context = jetstream::new(client.clone());

        info!("Connected to NATS at {}", nats_url);

        Ok(Self { client, js_context })
    }

    // УЛУЧШЕНИЕ: Добавить retry logic
    pub async fn publish_with_retry(
        &self,
        subject: &str,
        event: &TransactionEvent,
        max_retries: u32,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let mut retries = 0;
        let mut delay = Duration::from_millis(100);

        loop {
            match self.publish(subject, event).await {
                Ok(_) => {
                    info!("Successfully published event to {}", subject);
                    return Ok(());
                }
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        error!("Failed to publish after {} retries: {}", max_retries, e);
                        return Err(e);
                    }

                    warn!("Publish failed (attempt {}/{}), retrying in {:?}", retries, max_retries, delay);
                    tokio::time::sleep(delay).await;

                    // Exponential backoff
                    delay = delay.saturating_mul(2);
                }
            }
        }
    }

    async fn publish(
        &self,
        subject: &str,
        event: &TransactionEvent,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let payload = serde_json::to_vec(event)?;

        // Timeout для предотвращения зависания
        timeout(
            Duration::from_secs(5),
            self.js_context.publish(subject, payload.into())
        )
        .await??
        .await?;

        Ok(())
    }
}

pub struct NatsConsumer {
    client: Client,
    js_context: jetstream::Context,
}

impl NatsConsumer {
    pub async fn new(nats_url: &str) -> Result<Self, Box<dyn std::error::Error>> {
        let client = async_nats::connect(nats_url).await?;
        let js_context = jetstream::new(client.clone());

        info!("Connected to NATS consumer at {}", nats_url);

        Ok(Self { client, js_context })
    }

    // УЛУЧШЕНИЕ: Добавить graceful error handling
    pub async fn subscribe(
        &self,
        subject: &str,
        callback: impl Fn(TransactionEvent) -> Result<(), Box<dyn std::error::Error>> + Send + 'static,
    ) -> Result<(), Box<dyn std::error::Error>> {
        let stream = self.js_context
            .get_or_create_stream(jetstream::stream::Config {
                name: "DELTRAN_EVENTS".to_string(),
                subjects: vec![subject.to_string()],
                ..Default::default()
            })
            .await?;

        let consumer = stream
            .get_or_create_consumer(
                "token-engine-consumer",
                jetstream::consumer::pull::Config {
                    durable_name: Some("token-engine-consumer".to_string()),
                    ..Default::default()
                },
            )
            .await?;

        let mut messages = consumer.messages().await?;

        info!("Subscribed to subject: {}", subject);

        while let Some(msg) = messages.next().await {
            match msg {
                Ok(msg) => {
                    match serde_json::from_slice::<TransactionEvent>(&msg.payload) {
                        Ok(event) => {
                            match callback(event) {
                                Ok(_) => {
                                    // Acknowledge message
                                    msg.ack().await?;
                                }
                                Err(e) => {
                                    error!("Callback error: {}, will retry", e);
                                    // Negative acknowledge - будет redelivered
                                    msg.ack_with(jetstream::AckKind::Nak(Some(Duration::from_secs(5)))).await?;
                                }
                            }
                        }
                        Err(e) => {
                            error!("Failed to deserialize event: {}", e);
                            msg.ack_with(jetstream::AckKind::Term).await?;
                        }
                    }
                }
                Err(e) => {
                    error!("Error receiving message: {}", e);
                }
            }
        }

        Ok(())
    }
}
```

### 3. Circuit Breaker для HTTP клиентов в Go сервисах

**Проверь что уже есть:**

```bash
grep -r "hystrix" services/gateway/
```

Если нет, добавь:

```go
// services/gateway/pkg/circuitbreaker/breaker.go

package circuitbreaker

import (
    "fmt"
    "time"

    "github.com/afex/hystrix-go/hystrix"
)

// InitCircuitBreakers initializes circuit breakers for all downstream services
func InitCircuitBreakers() {
    // Token Engine
    hystrix.ConfigureCommand("token-engine", hystrix.CommandConfig{
        Timeout:                5000,  // 5 seconds
        MaxConcurrentRequests:  100,
        RequestVolumeThreshold: 20,
        SleepWindow:            5000,  // 5 seconds before retry
        ErrorPercentThreshold:  50,    // 50% error rate triggers open
    })

    // Obligation Engine
    hystrix.ConfigureCommand("obligation-engine", hystrix.CommandConfig{
        Timeout:                5000,
        MaxConcurrentRequests:  100,
        RequestVolumeThreshold: 20,
        SleepWindow:            5000,
        ErrorPercentThreshold:  50,
    })

    // Settlement Engine
    hystrix.ConfigureCommand("settlement-engine", hystrix.CommandConfig{
        Timeout:                10000,  // 10 seconds (settlements take longer)
        MaxConcurrentRequests:  50,
        RequestVolumeThreshold: 20,
        SleepWindow:            10000,
        ErrorPercentThreshold:  50,
    })

    // Analytics Collector
    hystrix.ConfigureCommand("analytics-collector", hystrix.CommandConfig{
        Timeout:                3000,  // 3 seconds
        MaxConcurrentRequests:  200,
        RequestVolumeThreshold: 20,
        SleepWindow:            5000,
        ErrorPercentThreshold:  50,
    })
}

// CallWithCircuitBreaker wraps a service call with circuit breaker
func CallWithCircuitBreaker(serviceName string, run func() error, fallback func(error) error) error {
    output := make(chan bool, 1)
    errors := hystrix.Go(serviceName, func() error {
        err := run()
        if err == nil {
            output <- true
        }
        return err
    }, fallback)

    select {
    case out := <-output:
        if out {
            return nil
        }
    case err := <-errors:
        return err
    }

    return nil
}

// GetCircuitBreakerStatus returns the current status of a circuit breaker
func GetCircuitBreakerStatus(serviceName string) string {
    circuit, _, _ := hystrix.GetCircuit(serviceName)
    if circuit == nil {
        return "UNKNOWN"
    }

    if circuit.IsOpen() {
        return "OPEN"
    }

    return "CLOSED"
}
```

**Использование в Gateway:**

```go
// services/gateway/internal/clients/token_engine.go

package clients

import (
    "bytes"
    "encoding/json"
    "fmt"
    "net/http"

    "github.com/deltran/gateway/pkg/circuitbreaker"
)

type TokenEngineClient struct {
    baseURL string
    client  *http.Client
}

func NewTokenEngineClient(baseURL string) *TokenEngineClient {
    return &TokenEngineClient{
        baseURL: baseURL,
        client:  &http.Client{Timeout: 5 * time.Second},
    }
}

func (c *TokenEngineClient) MintToken(tokenData map[string]interface{}) (map[string]interface{}, error) {
    var result map[string]interface{}

    err := circuitbreaker.CallWithCircuitBreaker(
        "token-engine",
        func() error {
            // Actual HTTP call
            payload, _ := json.Marshal(tokenData)
            resp, err := c.client.Post(
                c.baseURL+"/tokens/mint",
                "application/json",
                bytes.NewBuffer(payload),
            )

            if err != nil {
                return err
            }
            defer resp.Body.Close()

            if resp.StatusCode >= 400 {
                return fmt.Errorf("token engine returned status %d", resp.StatusCode)
            }

            return json.NewDecoder(resp.Body).Decode(&result)
        },
        func(err error) error {
            // Fallback logic
            log.Printf("Token Engine circuit breaker triggered: %v", err)
            // Return cached data or default response
            result = map[string]interface{}{
                "status": "fallback",
                "error":  err.Error(),
            }
            return nil
        },
    )

    return result, err
}
```

### 4. Retry Logic с Exponential Backoff для Rust

```rust
// services/token-engine/src/retry.rs (НОВЫЙ ФАЙЛ)

use std::time::Duration;
use tokio::time::sleep;
use tracing::{info, warn};

pub struct RetryConfig {
    pub max_retries: u32,
    pub initial_delay: Duration,
    pub max_delay: Duration,
    pub multiplier: u32,
}

impl Default for RetryConfig {
    fn default() -> Self {
        Self {
            max_retries: 3,
            initial_delay: Duration::from_millis(100),
            max_delay: Duration::from_secs(30),
            multiplier: 2,
        }
    }
}

pub async fn retry_with_backoff<F, T, E>(
    operation: F,
    config: RetryConfig,
    operation_name: &str,
) -> Result<T, E>
where
    F: Fn() -> futures::future::BoxFuture<'static, Result<T, E>>,
    E: std::fmt::Display,
{
    let mut retries = 0;
    let mut delay = config.initial_delay;

    loop {
        match operation().await {
            Ok(result) => {
                if retries > 0 {
                    info!("{} succeeded after {} retries", operation_name, retries);
                }
                return Ok(result);
            }
            Err(e) => {
                retries += 1;
                if retries >= config.max_retries {
                    warn!("{} failed after {} retries: {}", operation_name, retries, e);
                    return Err(e);
                }

                warn!(
                    "{} attempt {}/{} failed: {}, retrying in {:?}",
                    operation_name, retries, config.max_retries, e, delay
                );

                sleep(delay).await;

                // Exponential backoff
                delay = std::cmp::min(
                    delay.saturating_mul(config.multiplier),
                    config.max_delay,
                );
            }
        }
    }
}
```

**Использование:**

```rust
use crate::retry::{retry_with_backoff, RetryConfig};

// В коде сервиса:
let result = retry_with_backoff(
    || Box::pin(async {
        // HTTP call или другая операция
        http_client.post("http://localhost:8082/obligations")
            .json(&obligation_data)
            .send()
            .await
    }),
    RetryConfig::default(),
    "Create Obligation",
).await?;
```

### 5. Health Check Aggregator

```go
// services/gateway/internal/health/aggregator.go

package health

import (
    "context"
    "fmt"
    "net/http"
    "sync"
    "time"
)

type ServiceStatus struct {
    Name      string `json:"name"`
    Status    string `json:"status"`
    Latency   int64  `json:"latency_ms"`
    LastCheck string `json:"last_check"`
}

type HealthAggregator struct {
    services map[string]string
    client   *http.Client
}

func NewHealthAggregator(services map[string]string) *HealthAggregator {
    return &HealthAggregator{
        services: services,
        client:   &http.Client{Timeout: 3 * time.Second},
    }
}

func (ha *HealthAggregator) CheckAll(ctx context.Context) []ServiceStatus {
    var wg sync.WaitGroup
    results := make([]ServiceStatus, 0, len(ha.services))
    resultsChan := make(chan ServiceStatus, len(ha.services))

    for name, url := range ha.services {
        wg.Add(1)
        go func(name, url string) {
            defer wg.Done()
            resultsChan <- ha.checkService(ctx, name, url)
        }(name, url)
    }

    go func() {
        wg.Wait()
        close(resultsChan)
    }()

    for status := range resultsChan {
        results = append(results, status)
    }

    return results
}

func (ha *HealthAggregator) checkService(ctx context.Context, name, url string) ServiceStatus {
    start := time.Now()
    req, err := http.NewRequestWithContext(ctx, "GET", url+"/health", nil)
    if err != nil {
        return ServiceStatus{
            Name:      name,
            Status:    "ERROR",
            Latency:   0,
            LastCheck: time.Now().Format(time.RFC3339),
        }
    }

    resp, err := ha.client.Do(req)
    latency := time.Since(start).Milliseconds()

    if err != nil || resp.StatusCode != 200 {
        return ServiceStatus{
            Name:      name,
            Status:    "DOWN",
            Latency:   latency,
            LastCheck: time.Now().Format(time.RFC3339),
        }
    }
    defer resp.Body.Close()

    return ServiceStatus{
        Name:      name,
        Status:    "UP",
        Latency:   latency,
        LastCheck: time.Now().Format(time.RFC3339),
    }
}
```

### 6. Обновление Gateway для использования улучшенной интеграции

```go
// services/gateway/cmd/server/main.go

import (
    "github.com/deltran/gateway/pkg/circuitbreaker"
    "github.com/deltran/gateway/internal/health"
)

func main() {
    // Initialize circuit breakers
    circuitbreaker.InitCircuitBreakers()

    // Initialize health aggregator
    healthAgg := health.NewHealthAggregator(map[string]string{
        "token-engine":       "http://localhost:8081",
        "obligation-engine":  "http://localhost:8082",
        "clearing-engine":    "http://localhost:8085",
        "settlement-engine":  "http://localhost:8088",
        "notification-engine": "http://localhost:8089",
        "analytics-collector": "http://localhost:8093",
    })

    // Health check endpoint
    router.HandleFunc("/health/all", func(w http.ResponseWriter, r *http.Request) {
        statuses := healthAgg.CheckAll(r.Context())
        json.NewEncoder(w).Encode(map[string]interface{}{
            "services": statuses,
            "timestamp": time.Now().Format(time.RFC3339),
        })
    })

    // ... rest of the code
}
```

## Технологический стек
- **NATS JetStream**: УЖЕ используется для messaging
- **Hystrix**: Circuit breakers для Go
- **Retry logic**: Exponential backoff для Rust
- **Context7**: Для получения актуальных integration patterns

## Порядок выполнения

```bash
# 1. СКАНИРОВАНИЕ - проверить существующую интеграцию
grep -r "nats" services/*/
grep -r "http.Client" services/*/

# 2. Context7 - получить актуальные patterns
context7 docs nats "rust jetstream"
context7 docs hystrix "go circuit breaker"

# 3. Улучшить существующие NATS producers/consumers

# 4. Добавить circuit breakers в Go сервисы

# 5. Добавить retry logic в Rust сервисы

# 6. Тестирование
cargo test --all
go test ./...

# 7. Integration test
./test-integration.sh
```

## Критически важно

1. **НЕ СОЗДАВАТЬ новые сервисы** - только улучшать интеграцию существующих
2. **ПРОВЕРЯТЬ** что уже реализовано перед добавлением
3. **NATS УЖЕ РАБОТАЕТ** - только улучшай retry/error handling
4. **Использовать Context7** для integration patterns
5. **Постепенно** добавлять circuit breakers и retry logic

## Результат
Улучшенная интеграция между всеми 11 существующими сервисами:
- Retry logic с exponential backoff
- Circuit breakers для предотвращения каскадных отказов
- Улучшенный NATS messaging с error handling
- Health check aggregator
- Без создания новых сервисов
