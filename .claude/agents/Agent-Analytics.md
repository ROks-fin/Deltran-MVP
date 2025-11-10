# Agent-Analytics

## Роль
Агент для улучшения мониторинга СУЩЕСТВУЮЩИХ сервисов DelTran MVP: добавление Prometheus metrics в Rust/Go сервисы, улучшение Analytics Collector, создание Grafana dashboards для реальных метрик.

## Контекст
DelTran MVP имеет **11 ГОТОВЫХ сервисов**:
- **Rust (7)**: token-engine, clearing-engine, settlement-engine, obligation-engine, risk-engine, compliance-engine, liquidity-router
- **Go (3)**: gateway, notification-engine, reporting-engine
- **Python (1)**: analytics-collector (УЖЕ СОЗДАН на порту 8093)

**Analytics Collector УЖЕ работает** с:
- PostgreSQL для хранения метрик
- Endpoints для dashboard metrics
- Transaction analytics

## Задачи

### 🔍 ПЕРВЫЙ ШАГ: Сканирование

**ОБЯЗАТЕЛЬНО перед началом работы:**

```bash
# 1. Проверить существующие metrics endpoints
curl http://localhost:8080/metrics  # Gateway
curl http://localhost:8081/metrics  # Token Engine
curl http://localhost:8093/metrics/dashboard  # Analytics Collector

# 2. Проверить что уже есть для метрик
grep -r "metrics" services/*/src/
grep -r "prometheus" services/*/Cargo.toml
grep -r "prometheus" services/*/go.mod

# 3. Проверить Analytics Collector
cat services/analytics-collector/main.py
ls services/analytics-collector/

# 4. Проверить существующие dashboards
ls monitoring/grafana/
```

### 1. Использование Context7 для Prometheus patterns

```bash
# Получить актуальную документацию
context7 resolve prometheus
context7 docs prometheus "rust actix-web integration"
context7 docs prometheus "go metrics middleware"

# Grafana dashboards
context7 resolve grafana
context7 docs grafana "prometheus dashboard json"
```

### 2. Добавление Prometheus Metrics в Rust сервисы

**ТОЛЬКО если /metrics endpoint еще НЕТ!** Проверь сначала:

```bash
curl http://localhost:8081/metrics
```

Если нет, добавь:

```rust
// services/token-engine/src/metrics.rs (НОВЫЙ ФАЙЛ)

use prometheus::{
    IntCounter, IntCounterVec, IntGauge, Histogram, HistogramOpts,
    HistogramVec, Opts, Registry, TextEncoder, Encoder,
};
use lazy_static::lazy_static;
use std::sync::Arc;

lazy_static! {
    // HTTP metrics
    pub static ref HTTP_REQUESTS_TOTAL: IntCounterVec = IntCounterVec::new(
        Opts::new("http_requests_total", "Total HTTP requests"),
        &["method", "path", "status"]
    ).unwrap();

    pub static ref HTTP_REQUEST_DURATION: HistogramVec = HistogramVec::new(
        HistogramOpts::new("http_request_duration_seconds", "HTTP request duration"),
        &["method", "path"]
    ).unwrap();

    // Business metrics
    pub static ref TOKENS_MINTED: IntCounter = IntCounter::new(
        "tokens_minted_total", "Total tokens minted"
    ).unwrap();

    pub static ref TOKENS_BURNED: IntCounter = IntCounter::new(
        "tokens_burned_total", "Total tokens burned"
    ).unwrap();

    pub static ref ACTIVE_TOKENS: IntGauge = IntGauge::new(
        "active_tokens", "Number of active tokens"
    ).unwrap();

    pub static ref TOKEN_VALUE: Histogram = Histogram::with_opts(
        HistogramOpts::new("token_value", "Token values distribution")
            .buckets(vec![100.0, 1000.0, 10000.0, 100000.0, 1000000.0])
    ).unwrap();
}

pub fn register_metrics(registry: &Registry) {
    registry.register(Box::new(HTTP_REQUESTS_TOTAL.clone())).unwrap();
    registry.register(Box::new(HTTP_REQUEST_DURATION.clone())).unwrap();
    registry.register(Box::new(TOKENS_MINTED.clone())).unwrap();
    registry.register(Box::new(TOKENS_BURNED.clone())).unwrap();
    registry.register(Box::new(ACTIVE_TOKENS.clone())).unwrap();
    registry.register(Box::new(TOKEN_VALUE.clone())).unwrap();
}

pub fn metrics_handler() -> String {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();
    let mut buffer = vec![];
    encoder.encode(&metric_families, &mut buffer).unwrap();
    String::from_utf8(buffer).unwrap()
}
```

**Интеграция в main.rs:**

```rust
// services/token-engine/src/main.rs

mod metrics;
use metrics::*;

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Register Prometheus metrics
    let registry = Registry::new();
    register_metrics(&registry);

    HttpServer::new(move || {
        App::new()
            .route("/metrics", web::get().to(|| async {
                HttpResponse::Ok()
                    .content_type("text/plain")
                    .body(metrics_handler())
            }))
            // ... остальные routes
    })
    .bind("127.0.0.1:8081")?
    .run()
    .await
}
```

**Обновить Cargo.toml:**

```toml
[dependencies]
prometheus = "0.13"
lazy_static = "1.4"
```

### 3. Добавление Prometheus Metrics в Go сервисы

**Проверь сначала:**

```bash
grep -r "prometheus" services/notification-engine/
```

Если нет, добавь:

```go
// services/notification-engine/internal/metrics/metrics.go

package metrics

import (
    "github.com/prometheus/client_golang/prometheus"
    "github.com/prometheus/client_golang/prometheus/promauto"
)

var (
    // HTTP metrics
    HTTPRequestsTotal = promauto.NewCounterVec(
        prometheus.CounterOpts{
            Name: "http_requests_total",
            Help: "Total HTTP requests",
        },
        []string{"method", "path", "status"},
    )

    HTTPRequestDuration = promauto.NewHistogramVec(
        prometheus.HistogramOpts{
            Name:    "http_request_duration_seconds",
            Help:    "HTTP request duration",
            Buckets: prometheus.DefBuckets,
        },
        []string{"method", "path"},
    )

    // Business metrics
    NotificationsSent = promauto.NewCounterVec(
        prometheus.CounterOpts{
            Name: "notifications_sent_total",
            Help: "Total notifications sent",
        },
        []string{"channel", "status"},
    )

    ActiveWebSockets = promauto.NewGauge(
        prometheus.GaugeOpts{
            Name: "active_websockets",
            Help: "Number of active WebSocket connections",
        },
    )

    QueueDepth = promauto.NewGauge(
        prometheus.GaugeOpts{
            Name: "notification_queue_depth",
            Help: "Current notification queue depth",
        },
    )
)
```

**Интеграция в main.go:**

```go
// services/notification-engine/cmd/server/main.go

import (
    "github.com/prometheus/client_golang/prometheus/promhttp"
    "github.com/deltran/notification-engine/internal/metrics"
)

func main() {
    // ... existing code

    router := mux.NewRouter()

    // Metrics endpoint
    router.Handle("/metrics", promhttp.Handler())

    // ... остальные routes
}
```

### 4. Улучшение Analytics Collector

**Analytics Collector УЖЕ СУЩЕСТВУЕТ!** Только улучши его:

```python
# services/analytics-collector/main.py

# ДОБАВЬ новые endpoints если их нет:

@app.get("/metrics/prometheus")
async def prometheus_metrics():
    """Export metrics in Prometheus format for scraping"""

    query = """
        SELECT
            COUNT(*) as total_transactions,
            COUNT(CASE WHEN status = 'completed' THEN 1 END) as completed,
            COUNT(CASE WHEN status = 'failed' THEN 1 END) as failed,
            AVG(total_latency) as avg_latency,
            MAX(total_latency) as max_latency
        FROM transaction_analytics
        WHERE timestamp > NOW() - INTERVAL '5 minutes'
    """

    async with app.state.db_pool.acquire() as conn:
        row = await conn.fetchrow(query)

        # Format as Prometheus metrics
        metrics = f"""
# HELP deltran_transactions_total Total transactions
# TYPE deltran_transactions_total counter
deltran_transactions_total{{status="completed"}} {row['completed']}
deltran_transactions_total{{status="failed"}} {row['failed']}

# HELP deltran_transaction_latency_seconds Transaction latency
# TYPE deltran_transaction_latency_seconds gauge
deltran_transaction_latency_avg_seconds {row['avg_latency'] / 1000}
deltran_transaction_latency_max_seconds {row['max_latency'] / 1000}
"""

        return Response(content=metrics, media_type="text/plain")

@app.get("/metrics/service/{service_name}")
async def get_service_metrics(service_name: str):
    """Get metrics for specific service"""

    query = """
        SELECT
            service,
            COUNT(*) as request_count,
            AVG(duration_ms) as avg_duration,
            PERCENTILE_CONT(0.95) WITHIN GROUP (ORDER BY duration_ms) as p95_duration
        FROM transaction_events
        WHERE service = $1 AND timestamp > NOW() - INTERVAL '5 minutes'
        GROUP BY service
    """

    async with app.state.db_pool.acquire() as conn:
        row = await conn.fetchrow(query, service_name)
        return dict(row) if row else {}
```

### 5. Grafana Dashboard для СУЩЕСТВУЮЩИХ сервисов

```json
// monitoring/grafana/dashboards/deltran-overview.json

{
  "dashboard": {
    "title": "DelTran MVP - Overview",
    "panels": [
      {
        "id": 1,
        "title": "Transaction Throughput (TPS)",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total{path=~\"/api/v1/transfer\"}[1m])",
            "legendFormat": "{{service}}"
          }
        ]
      },
      {
        "id": 2,
        "title": "Service Health Status",
        "type": "stat",
        "targets": [
          {
            "expr": "up{job=~\"gateway|token-engine|obligation-engine|clearing-engine|settlement-engine|notification-engine|reporting-engine\"}",
            "legendFormat": "{{job}}"
          }
        ]
      },
      {
        "id": 3,
        "title": "Request Latency (P95)",
        "type": "graph",
        "targets": [
          {
            "expr": "histogram_quantile(0.95, rate(http_request_duration_seconds_bucket[5m]))",
            "legendFormat": "{{service}} - {{path}}"
          }
        ]
      },
      {
        "id": 4,
        "title": "Error Rate",
        "type": "graph",
        "targets": [
          {
            "expr": "rate(http_requests_total{status=~\"5..\"}[5m])",
            "legendFormat": "{{service}}"
          }
        ]
      },
      {
        "id": 5,
        "title": "Active Tokens",
        "type": "gauge",
        "targets": [
          {
            "expr": "active_tokens"
          }
        ]
      },
      {
        "id": 6,
        "title": "WebSocket Connections",
        "type": "graph",
        "targets": [
          {
            "expr": "active_websockets"
          }
        ]
      },
      {
        "id": 7,
        "title": "Transaction Success Rate",
        "type": "gauge",
        "targets": [
          {
            "expr": "rate(deltran_transactions_total{status=\"completed\"}[5m]) / rate(deltran_transactions_total[5m]) * 100"
          }
        ]
      }
    ]
  }
}
```

### 6. Prometheus Configuration для ВСЕХ сервисов

```yaml
# monitoring/prometheus/prometheus.yml

global:
  scrape_interval: 15s
  evaluation_interval: 15s

scrape_configs:
  # Gateway (Port 8080)
  - job_name: 'gateway'
    static_configs:
      - targets: ['localhost:8080']
        labels:
          service: 'gateway'

  # Token Engine (Port 8081)
  - job_name: 'token-engine'
    static_configs:
      - targets: ['localhost:8081']
        labels:
          service: 'token-engine'

  # Obligation Engine (Port 8082)
  - job_name: 'obligation-engine'
    static_configs:
      - targets: ['localhost:8082']
        labels:
          service: 'obligation-engine'

  # Liquidity Router (Port 8083)
  - job_name: 'liquidity-router'
    static_configs:
      - targets: ['localhost:8083']
        labels:
          service: 'liquidity-router'

  # Risk Engine (Port 8084)
  - job_name: 'risk-engine'
    static_configs:
      - targets: ['localhost:8084']
        labels:
          service: 'risk-engine'

  # Clearing Engine (Port 8085)
  - job_name: 'clearing-engine'
    static_configs:
      - targets: ['localhost:8085']
        labels:
          service: 'clearing-engine'

  # Compliance Engine (Port 8086)
  - job_name: 'compliance-engine'
    static_configs:
      - targets: ['localhost:8086']
        labels:
          service: 'compliance-engine'

  # Reporting Engine (Port 8087)
  - job_name: 'reporting-engine'
    static_configs:
      - targets: ['localhost:8087']
        labels:
          service: 'reporting-engine'

  # Settlement Engine (Port 8088)
  - job_name: 'settlement-engine'
    static_configs:
      - targets: ['localhost:8088']
        labels:
          service: 'settlement-engine'

  # Notification Engine (Port 8089)
  - job_name: 'notification-engine'
    static_configs:
      - targets: ['localhost:8089']
        labels:
          service: 'notification-engine'

  # Analytics Collector (Port 8093)
  - job_name: 'analytics-collector'
    static_configs:
      - targets: ['localhost:8093']
        labels:
          service: 'analytics-collector'
```

### 7. Docker Compose для Monitoring Stack

```yaml
# monitoring/docker-compose.yml

version: '3.8'

services:
  prometheus:
    image: prom/prometheus:latest
    container_name: deltran-prometheus
    volumes:
      - ./prometheus/prometheus.yml:/etc/prometheus/prometheus.yml
      - prometheus-data:/prometheus
    command:
      - '--config.file=/etc/prometheus/prometheus.yml'
      - '--storage.tsdb.path=/prometheus'
    ports:
      - "9090:9090"
    networks:
      - deltran-monitoring
    extra_hosts:
      - "host.docker.internal:host-gateway"

  grafana:
    image: grafana/grafana:latest
    container_name: deltran-grafana
    volumes:
      - grafana-data:/var/lib/grafana
      - ./grafana/dashboards:/etc/grafana/provisioning/dashboards
      - ./grafana/datasources:/etc/grafana/provisioning/datasources
    environment:
      - GF_SECURITY_ADMIN_USER=admin
      - GF_SECURITY_ADMIN_PASSWORD=admin
      - GF_USERS_ALLOW_SIGN_UP=false
    ports:
      - "3000:3000"
    networks:
      - deltran-monitoring
    depends_on:
      - prometheus

volumes:
  prometheus-data:
  grafana-data:

networks:
  deltran-monitoring:
    driver: bridge
```

### 8. Grafana Datasource Configuration

```yaml
# monitoring/grafana/datasources/prometheus.yml

apiVersion: 1

datasources:
  - name: Prometheus
    type: prometheus
    access: proxy
    url: http://prometheus:9090
    isDefault: true
    editable: true
```

## Технологический стек
- **Prometheus**: Для сбора метрик из Rust/Go сервисов
- **Grafana**: Для визуализации
- **Analytics Collector**: УЖЕ СУЩЕСТВУЕТ (Python FastAPI)
- **Context7**: Для получения актуальных patterns

## Порядок выполнения

```bash
# 1. СКАНИРОВАНИЕ - проверить что уже есть
grep -r "metrics" services/*/src/
curl http://localhost:8093/metrics/dashboard

# 2. Context7 - получить актуальные patterns
context7 docs prometheus "rust integration"
context7 docs grafana "dashboard json"

# 3. Добавить /metrics endpoints в сервисы где их НЕТ

# 4. Запустить monitoring stack
cd monitoring
docker-compose up -d

# 5. Проверить что Prometheus scraping работает
curl http://localhost:9090/targets

# 6. Открыть Grafana
# http://localhost:3000 (admin/admin)

# 7. Импортировать dashboard
# Import -> Upload deltran-overview.json
```

## Критически важно

1. **Analytics Collector УЖЕ СУЩЕСТВУЕТ** - только улучшай его
2. **НЕ создавать новые сервисы мониторинга**
3. **ПРОВЕРЯТЬ** какие /metrics endpoints уже есть
4. **Использовать Context7** для Prometheus/Grafana patterns
5. **Добавлять metrics ТОЛЬКО там где их нет**

## Результат
Полный monitoring stack для всех 11 существующих сервисов:
- Prometheus metrics endpoints в каждом сервисе
- Улучшенный Analytics Collector
- Grafana dashboards для реальных метрик
- Docker Compose для запуска
- Без дублирования существующих функций
