# CLEARING ENGINE - ДЕТАЛЬНАЯ СПЕЦИФИКАЦИЯ

## 🎯 КРИТИЧЕСКАЯ ВАЖНОСТЬ
Clearing Engine управляет клиринговыми окнами и оркестрирует процесс неттинга. Это сердце системы отложенных расчетов, обеспечивающее 70-90% экономию на движении средств.

## 📊 АРХИТЕКТУРА

```
Каждые 6 часов (00:00, 06:00, 12:00, 18:00 UTC):
1. Закрытие текущего окна
2. Сбор всех обязательств
3. Расчет неттинга
4. Генерация settlement инструкций
5. Открытие нового окна
```

## 🛠️ ТЕХНОЛОГИИ
- **Язык:** Rust 1.75
- **Framework:** Actix-web 4.4
- **Scheduler:** tokio-cron-scheduler
- **База данных:** PostgreSQL 16
- **Message Queue:** NATS JetStream
- **gRPC:** tonic для внутренней коммуникации

## 📁 СТРУКТУРА ПРОЕКТА

```
services/clearing-engine/
├── src/
│   ├── main.rs                 # Entry point с scheduler
│   ├── lib.rs                 # Library exports
│   ├── models.rs              # Data structures
│   ├── errors.rs              # Error handling
│   ├── config.rs              # Configuration
│   ├── database.rs            # DB operations
│   ├── window/
│   │   ├── mod.rs            # Window management
│   │   ├── lifecycle.rs      # Window lifecycle (open/close)
│   │   ├── validator.rs      # Window validation
│   │   └── cutoff.rs         # Cutoff time management
│   ├── orchestration/
│   │   ├── mod.rs            # Orchestration logic
│   │   ├── collector.rs      # Obligation collection
│   │   ├── coordinator.rs    # Service coordination
│   │   └── finalizer.rs      # Settlement finalization
│   ├── grpc/
│   │   ├── mod.rs            # gRPC server
│   │   ├── clearing.proto    # Proto definitions
│   │   └── server.rs         # gRPC handlers
│   ├── monitoring/
│   │   ├── mod.rs            # Monitoring
│   │   ├── metrics.rs        # Prometheus metrics
│   │   └── health.rs         # Health checks
│   ├── atomics/              # Атомарные операции
│   │   ├── mod.rs
│   │   ├── window_lock.rs    # Блокировка окна
│   │   ├── state_machine.rs  # State transitions
│   │   └── rollback.rs       # Rollback logic
│   └── handlers.rs           # REST API handlers
├── proto/
│   └── clearing.proto        # gRPC definitions
├── Cargo.toml
├── Dockerfile
└── config.toml
```

## 📊 МОДЕЛИ ДАННЫХ

```rust
use chrono::{DateTime, Utc};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::sync::Arc;
use tokio::sync::RwLock;

// ===== CLEARING WINDOW =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ClearingWindow {
    pub id: i64,                          // Window ID (epoch timestamp)
    pub start_time: DateTime<Utc>,
    pub end_time: DateTime<Utc>,
    pub status: WindowStatus,
    pub region: ClearingRegion,
    pub total_obligations: u32,
    pub total_volume: Decimal,
    pub netted_volume: Decimal,
    pub saved_amount: Decimal,
    pub netting_efficiency: f64,
    pub settlement_instructions: Vec<Uuid>,
    pub metadata: WindowMetadata,
    pub created_at: DateTime<Utc>,
    pub closed_at: Option<DateTime<Utc>>,
    pub processed_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub enum WindowStatus {
    Scheduled,      // Будущее окно
    Open,          // Принимает транзакции
    Closing,       // В процессе закрытия
    Closed,        // Закрыто для новых транзакций
    Processing,    // Идет неттинг
    Settling,      // Идет settlement
    Completed,     // Завершено
    Failed,        // Ошибка
    Rolled_Back,   // Откачено
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ClearingRegion {
    Global,        // Все регионы
    ADGM,         // ОАЭ и GCC
    Europe,       // EU/UK
    Americas,     // USA/LATAM
    AsiaPacific,  // APAC
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct WindowMetadata {
    pub cutoff_time: DateTime<Utc>,
    pub grace_period_seconds: i64,
    pub max_obligations: u32,
    pub min_netting_efficiency: f64,
    pub auto_settle: bool,
    pub emergency_mode: bool,
}

// ===== ORCHESTRATION =====
#[derive(Debug, Clone)]
pub struct ClearingOrchestrator {
    pub window_manager: Arc<RwLock<WindowManager>>,
    pub obligation_client: ObligationEngineClient,
    pub settlement_client: SettlementEngineClient,
    pub notification_client: NotificationEngineClient,
    pub risk_client: RiskEngineClient,
}

// ===== CLEARING RESULT =====
#[derive(Debug, Serialize, Deserialize)]
pub struct ClearingResult {
    pub window_id: i64,
    pub status: ClearingStatus,
    pub obligations_processed: u32,
    pub net_positions: Vec<NetPosition>,
    pub settlement_instructions: Vec<SettlementInstruction>,
    pub total_saved: Decimal,
    pub efficiency: f64,
    pub errors: Vec<ClearingError>,
    pub processing_time_ms: u64,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ClearingStatus {
    Success,
    PartialSuccess,
    Failed,
}

// ===== NET POSITION =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NetPosition {
    pub bank_pair: BankPair,
    pub currency: String,
    pub gross_debit: Decimal,
    pub gross_credit: Decimal,
    pub net_amount: Decimal,
    pub direction: PositionDirection,
    pub obligations_count: u32,
    pub netting_ratio: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct BankPair {
    pub bank_a: Uuid,
    pub bank_b: Uuid,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PositionDirection {
    Debit,   // Bank A owes Bank B
    Credit,  // Bank B owes Bank A
    Neutral, // Net zero
}

// ===== SETTLEMENT INSTRUCTION =====
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SettlementInstruction {
    pub id: Uuid,
    pub window_id: i64,
    pub from_bank: Uuid,
    pub to_bank: Uuid,
    pub amount: Decimal,
    pub currency: String,
    pub instruction_type: InstructionType,
    pub priority: u8,
    pub deadline: DateTime<Utc>,
    pub metadata: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InstructionType {
    NetSettlement,      // После неттинга
    GrossSettlement,    // Без неттинга
    EmergencySettlement,// Экстренный
}

// ===== ATOMIC OPERATIONS =====
#[derive(Debug)]
pub struct AtomicWindowOperation {
    pub operation_id: Uuid,
    pub window_id: i64,
    pub operation_type: AtomicOperationType,
    pub state: Arc<RwLock<AtomicState>>,
    pub rollback_handler: Option<Box<dyn Fn() -> Result<(), Error> + Send + Sync>>,
}

#[derive(Debug, Clone)]
pub enum AtomicOperationType {
    WindowClose,
    ObligationCollection,
    NettingCalculation,
    InstructionGeneration,
    SettlementInitiation,
    WindowOpen,
}

#[derive(Debug, Clone)]
pub enum AtomicState {
    Pending,
    InProgress,
    Committed,
    RolledBack,
    Failed(String),
}
```

## 🔧 CORE АЛГОРИТМЫ

### 1. Window Lifecycle Management

```rust
// window/lifecycle.rs

impl WindowManager {
    /// Атомарное закрытие окна
    pub async fn close_window(&self, window_id: i64) -> Result<(), ClearingError> {
        // Начинаем транзакцию
        let mut tx = self.db.begin().await?;

        // 1. Проверка что окно существует и открыто
        let window = self.get_window(window_id).await?;
        if window.status != WindowStatus::Open {
            return Err(ClearingError::InvalidWindowState);
        }

        // 2. Установка grace period (30 секунд для последних транзакций)
        let cutoff_time = Utc::now();
        let grace_end = cutoff_time + chrono::Duration::seconds(30);

        // 3. Атомарное изменение статуса
        sqlx::query!(
            "UPDATE clearing_windows
             SET status = 'Closing',
                 cutoff_time = $1,
                 updated_at = NOW()
             WHERE id = $2 AND status = 'Open'",
            cutoff_time,
            window_id
        )
        .execute(&mut tx)
        .await?;

        // 4. Отправка уведомлений всем банкам
        self.notification_client
            .broadcast(WindowClosingEvent {
                window_id,
                cutoff_time,
                grace_period_ends: grace_end,
            })
            .await?;

        // 5. Ожидание grace period
        tokio::time::sleep(Duration::from_secs(30)).await;

        // 6. Финальное закрытие
        sqlx::query!(
            "UPDATE clearing_windows
             SET status = 'Closed',
                 closed_at = NOW()
             WHERE id = $1",
            window_id
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await?;

        // 7. Запуск асинхронной обработки
        tokio::spawn(async move {
            if let Err(e) = self.process_window(window_id).await {
                error!("Failed to process window {}: {}", window_id, e);
                self.handle_processing_failure(window_id).await;
            }
        });

        Ok(())
    }

    /// Открытие нового окна
    pub async fn open_new_window(&self, region: ClearingRegion) -> Result<ClearingWindow, ClearingError> {
        let mut tx = self.db.begin().await?;

        // Проверка что нет других открытых окон
        let open_windows = sqlx::query_scalar!(
            "SELECT COUNT(*) FROM clearing_windows
             WHERE status IN ('Open', 'Closing') AND region = $1",
            region as i32
        )
        .fetch_one(&mut tx)
        .await?;

        if open_windows.unwrap_or(0) > 0 {
            return Err(ClearingError::WindowAlreadyOpen);
        }

        let now = Utc::now();
        let window_id = now.timestamp();
        let end_time = now + chrono::Duration::hours(6);

        let window = ClearingWindow {
            id: window_id,
            start_time: now,
            end_time,
            status: WindowStatus::Open,
            region,
            total_obligations: 0,
            total_volume: Decimal::ZERO,
            netted_volume: Decimal::ZERO,
            saved_amount: Decimal::ZERO,
            netting_efficiency: 0.0,
            settlement_instructions: vec![],
            metadata: WindowMetadata {
                cutoff_time: end_time - chrono::Duration::minutes(5),
                grace_period_seconds: 30,
                max_obligations: 10000,
                min_netting_efficiency: 0.5,
                auto_settle: true,
                emergency_mode: false,
            },
            created_at: now,
            closed_at: None,
            processed_at: None,
        };

        // Сохранение в БД
        sqlx::query!(
            "INSERT INTO clearing_windows
             (id, start_time, end_time, status, region, metadata, created_at)
             VALUES ($1, $2, $3, $4, $5, $6, $7)",
            window.id,
            window.start_time,
            window.end_time,
            window.status as i32,
            window.region as i32,
            serde_json::to_value(&window.metadata)?,
            window.created_at
        )
        .execute(&mut tx)
        .await?;

        tx.commit().await?;

        Ok(window)
    }
}
```

### 2. Orchestration Process

```rust
// orchestration/coordinator.rs

impl ClearingOrchestrator {
    /// Главный процесс обработки окна
    pub async fn process_clearing_window(
        &self,
        window_id: i64
    ) -> Result<ClearingResult, ClearingError> {
        let start_time = Instant::now();
        let mut errors = Vec::new();

        // 1. COLLECT - Сбор обязательств
        let obligations = match self.collect_obligations(window_id).await {
            Ok(obs) => obs,
            Err(e) => {
                errors.push(e);
                return Ok(ClearingResult {
                    window_id,
                    status: ClearingStatus::Failed,
                    obligations_processed: 0,
                    net_positions: vec![],
                    settlement_instructions: vec![],
                    total_saved: Decimal::ZERO,
                    efficiency: 0.0,
                    errors,
                    processing_time_ms: start_time.elapsed().as_millis() as u64,
                });
            }
        };

        info!("Collected {} obligations for window {}", obligations.len(), window_id);

        // 2. VALIDATE - Валидация данных
        if let Err(e) = self.validate_obligations(&obligations).await {
            errors.push(e);
        }

        // 3. NETTING - Расчет неттинга через Obligation Engine
        let netting_result = match self.obligation_client
            .calculate_netting(window_id)
            .await
        {
            Ok(result) => result,
            Err(e) => {
                errors.push(ClearingError::NettingFailed(e.to_string()));
                // Fallback to gross settlement
                return self.fallback_gross_settlement(window_id, obligations).await;
            }
        };

        // 4. OPTIMIZE - Оптимизация settlement путей
        let optimized_positions = self.optimize_settlement_paths(
            netting_result.net_positions
        ).await?;

        // 5. GENERATE - Генерация инструкций
        let mut instructions = Vec::new();
        for position in &optimized_positions {
            if position.net_amount > Decimal::ZERO {
                let instruction = self.generate_settlement_instruction(position, window_id)?;
                instructions.push(instruction);
            }
        }

        // 6. RISK CHECK - Проверка рисков
        for instruction in &instructions {
            match self.risk_client.evaluate_settlement(&instruction).await {
                Ok(risk_result) if risk_result.approved => {},
                Ok(risk_result) => {
                    warn!("Settlement rejected by risk: {:?}", risk_result.reason);
                    errors.push(ClearingError::RiskCheckFailed(instruction.id));
                },
                Err(e) => {
                    errors.push(ClearingError::RiskCheckError(e.to_string()));
                }
            }
        }

        // 7. INITIATE SETTLEMENT - Запуск settlement
        let mut settlement_ids = Vec::new();
        for instruction in instructions {
            match self.settlement_client.execute(instruction.clone()).await {
                Ok(settlement_id) => {
                    settlement_ids.push(settlement_id);
                },
                Err(e) => {
                    error!("Settlement failed for instruction {}: {}", instruction.id, e);
                    errors.push(ClearingError::SettlementFailed(instruction.id));
                }
            }
        }

        // 8. UPDATE WINDOW - Обновление статуса окна
        self.update_window_status(
            window_id,
            WindowStatus::Completed,
            &netting_result,
            &settlement_ids
        ).await?;

        // 9. NOTIFICATIONS - Уведомления
        self.notification_client
            .notify_clearing_completed(window_id, &netting_result)
            .await?;

        let processing_time = start_time.elapsed().as_millis() as u64;

        Ok(ClearingResult {
            window_id,
            status: if errors.is_empty() {
                ClearingStatus::Success
            } else {
                ClearingStatus::PartialSuccess
            },
            obligations_processed: obligations.len() as u32,
            net_positions: optimized_positions,
            settlement_instructions: settlement_ids,
            total_saved: netting_result.total_saved,
            efficiency: netting_result.efficiency,
            errors,
            processing_time_ms: processing_time,
        })
    }

    /// Валидация обязательств перед неттингом
    async fn validate_obligations(&self, obligations: &[Obligation]) -> Result<(), ClearingError> {
        for obligation in obligations {
            // Проверка балансов
            let debtor_balance = self.get_bank_balance(obligation.bank_debtor_id).await?;
            if debtor_balance < obligation.amount_sent {
                return Err(ClearingError::InsufficientBalance {
                    bank_id: obligation.bank_debtor_id,
                    required: obligation.amount_sent,
                    available: debtor_balance,
                });
            }

            // Проверка лимитов
            let limit = self.get_bank_limit(obligation.bank_debtor_id).await?;
            if obligation.amount_sent > limit {
                return Err(ClearingError::LimitExceeded {
                    bank_id: obligation.bank_debtor_id,
                    amount: obligation.amount_sent,
                    limit,
                });
            }
        }
        Ok(())
    }
}
```

### 3. Settlement Path Optimization

```rust
// orchestration/coordinator.rs

impl ClearingOrchestrator {
    /// Оптимизация маршрутов settlement для минимизации транзакций
    async fn optimize_settlement_paths(
        &self,
        positions: Vec<NetPosition>
    ) -> Result<Vec<NetPosition>, ClearingError> {
        // Построение графа задолженностей
        let mut graph = DiGraph::<Uuid, Decimal>::new();
        let mut node_map = HashMap::new();

        for position in &positions {
            let bank_a = *node_map.entry(position.bank_pair.bank_a)
                .or_insert_with(|| graph.add_node(position.bank_pair.bank_a));
            let bank_b = *node_map.entry(position.bank_pair.bank_b)
                .or_insert_with(|| graph.add_node(position.bank_pair.bank_b));

            match position.direction {
                PositionDirection::Debit => {
                    graph.add_edge(bank_a, bank_b, position.net_amount);
                },
                PositionDirection::Credit => {
                    graph.add_edge(bank_b, bank_a, position.net_amount);
                },
                _ => {}
            }
        }

        // Поиск циклов для дополнительного неттинга
        let cycles = self.find_cycles(&graph);
        for cycle in cycles {
            let min_amount = self.find_min_amount_in_cycle(&graph, &cycle);
            self.reduce_cycle(&mut graph, &cycle, min_amount);
        }

        // Конвертация обратно в позиции
        let optimized = self.graph_to_positions(graph, node_map);
        Ok(optimized)
    }
}
```

## 🔌 gRPC API

```proto
// proto/clearing.proto

syntax = "proto3";
package clearing;

service ClearingEngine {
    // Window management
    rpc GetCurrentWindow(GetCurrentWindowRequest) returns (Window);
    rpc GetWindowStatus(GetWindowStatusRequest) returns (WindowStatus);
    rpc ForceCloseWindow(ForceCloseWindowRequest) returns (WindowCloseResult);

    // Streaming
    rpc StreamWindowUpdates(StreamWindowRequest) returns (stream WindowUpdate);
    rpc StreamSettlementStatus(StreamSettlementRequest) returns (stream SettlementStatus);

    // Manual intervention
    rpc TriggerEmergencyClearing(EmergencyRequest) returns (ClearingResult);
    rpc RollbackWindow(RollbackRequest) returns (RollbackResult);
}

message Window {
    int64 id = 1;
    string status = 2;
    string start_time = 3;
    string end_time = 4;
    int32 obligations_count = 5;
    string total_volume = 6;
}

message WindowUpdate {
    int64 window_id = 1;
    string event_type = 2;
    string timestamp = 3;
    string details = 4;
}
```

## 🔌 REST API ENDPOINTS

```yaml
# Window Management
GET    /api/v1/clearing/windows                 # Список всех окон
GET    /api/v1/clearing/windows/current         # Текущее окно
GET    /api/v1/clearing/windows/{id}           # Детали окна
GET    /api/v1/clearing/windows/{id}/status    # Статус окна

# Processing
POST   /api/v1/clearing/windows/{id}/process   # Запустить обработку
GET    /api/v1/clearing/windows/{id}/result    # Результат обработки

# Settlement Instructions
GET    /api/v1/clearing/instructions           # Все инструкции
GET    /api/v1/clearing/instructions/{id}      # Детали инструкции

# Manual Operations
POST   /api/v1/clearing/force-close           # Принудительное закрытие
POST   /api/v1/clearing/emergency             # Экстренный клиринг
POST   /api/v1/clearing/rollback/{id}         # Откат окна

# Monitoring
GET    /api/v1/clearing/metrics               # Метрики
GET    /api/v1/clearing/health                # Health check
```

## 📊 БАЗА ДАННЫХ

```sql
-- Clearing windows
CREATE TABLE clearing_windows (
    id BIGINT PRIMARY KEY, -- timestamp as ID
    start_time TIMESTAMPTZ NOT NULL,
    end_time TIMESTAMPTZ NOT NULL,
    status VARCHAR(20) NOT NULL,
    region VARCHAR(20) NOT NULL,
    total_obligations INT DEFAULT 0,
    total_volume DECIMAL(20,2) DEFAULT 0,
    netted_volume DECIMAL(20,2) DEFAULT 0,
    saved_amount DECIMAL(20,2) DEFAULT 0,
    netting_efficiency DECIMAL(5,2) DEFAULT 0,
    settlement_instructions UUID[],
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    closed_at TIMESTAMPTZ,
    processed_at TIMESTAMPTZ,

    INDEX idx_clearing_status (status),
    INDEX idx_clearing_region (region),
    INDEX idx_clearing_created (created_at DESC)
);

-- Window events for audit
CREATE TABLE clearing_window_events (
    id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL REFERENCES clearing_windows(id),
    event_type VARCHAR(50) NOT NULL,
    event_data JSONB NOT NULL,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),

    INDEX idx_window_events (window_id, created_at)
);

-- Atomic operations log
CREATE TABLE clearing_atomic_operations (
    operation_id UUID PRIMARY KEY DEFAULT gen_random_uuid(),
    window_id BIGINT NOT NULL,
    operation_type VARCHAR(50) NOT NULL,
    state VARCHAR(20) NOT NULL,
    started_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    completed_at TIMESTAMPTZ,
    error_message TEXT,
    rollback_data JSONB,

    INDEX idx_atomic_window (window_id),
    INDEX idx_atomic_state (state)
);
```

## ⚙️ CONFIGURATION

```toml
[server]
host = "0.0.0.0"
http_port = 8085
grpc_port = 50085

[database]
url = "${DATABASE_URL}"
max_connections = 20
min_connections = 5

[nats]
url = "${NATS_URL}"
stream = "clearing-events"
durable = "clearing-engine"

[clearing]
# Window configuration
window_duration_hours = 6
grace_period_seconds = 30
max_obligations_per_window = 10000

# Regions and schedules (UTC)
[clearing.regions.global]
schedule = ["00:00", "06:00", "12:00", "18:00"]
enabled = true

[clearing.regions.adgm]
schedule = ["00:00", "06:00", "12:00", "18:00"]
utc_offset = 4
enabled = true

[clearing.regions.europe]
schedule = ["08:00", "14:00", "20:00"]
utc_offset = 1
enabled = true

# Settlement
[settlement]
auto_settle = true
min_netting_efficiency = 0.5
emergency_threshold = 0.3

# Monitoring
[monitoring]
metrics_enabled = true
health_check_interval = 30

# Clients
[clients]
obligation_engine = "http://obligation-engine:8082"
settlement_engine = "http://settlement-engine:8087"
risk_engine = "http://risk-engine:8084"
notification_engine = "http://notification-engine:8089"
```

## 🚀 SCHEDULER SETUP

```rust
// main.rs

use tokio_cron_scheduler::{JobScheduler, Job};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    let config = Config::from_env()?;
    let orchestrator = ClearingOrchestrator::new(config.clone()).await?;

    // Инициализация scheduler
    let scheduler = JobScheduler::new().await?;

    // Добавление заданий для каждого региона
    for (region, region_config) in config.clearing.regions {
        if region_config.enabled {
            for schedule_time in region_config.schedule {
                let cron_expression = format!("0 {} * * * *", schedule_time);
                let region_clone = region.clone();
                let orchestrator_clone = orchestrator.clone();

                scheduler.add(
                    Job::new(cron_expression.as_str(), move |_uuid, _l| {
                        let region = region_clone.clone();
                        let orch = orchestrator_clone.clone();

                        tokio::spawn(async move {
                            info!("Starting clearing window for region: {}", region);

                            // Закрытие текущего окна
                            if let Some(current) = orch.get_current_window(region).await? {
                                orch.close_and_process_window(current.id).await?;
                            }

                            // Открытие нового окна
                            orch.open_new_window(region).await?;
                        });
                    })?
                ).await?;
            }
        }
    }

    scheduler.start().await?;

    // Запуск HTTP и gRPC серверов
    tokio::try_join!(
        start_http_server(config.server.http_port, orchestrator.clone()),
        start_grpc_server(config.server.grpc_port, orchestrator.clone())
    )?;

    Ok(())
}
```

## 📊 PROMETHEUS METRICS

```rust
lazy_static! {
    static ref CLEARING_WINDOWS_TOTAL: Counter = register_counter!(
        "deltran_clearing_windows_total",
        "Total number of clearing windows"
    ).unwrap();

    static ref CLEARING_WINDOW_DURATION: Histogram = register_histogram!(
        "deltran_clearing_window_duration_seconds",
        "Duration of clearing window processing"
    ).unwrap();

    static ref NETTING_EFFICIENCY: Gauge = register_gauge!(
        "deltran_netting_efficiency_percent",
        "Current netting efficiency"
    ).unwrap();

    static ref SETTLEMENT_INSTRUCTIONS_TOTAL: Counter = register_counter!(
        "deltran_settlement_instructions_total",
        "Total settlement instructions generated"
    ).unwrap();

    static ref CLEARING_ERRORS: Counter = register_counter!(
        "deltran_clearing_errors_total",
        "Total clearing errors"
    ).unwrap();

    static ref ATOMIC_OPERATIONS: Counter = register_counter_vec!(
        "deltran_atomic_operations_total",
        "Atomic operations by type",
        &["operation_type", "status"]
    ).unwrap();
}
```

## 🔒 БЕЗОПАСНОСТЬ И АТОМАРНОСТЬ

### Финансовый контроль каждой операции:

```rust
impl AtomicWindowOperation {
    /// Выполнение с автоматическим откатом при ошибке
    pub async fn execute(&mut self) -> Result<(), Error> {
        // 1. Начало операции
        self.state.write().await.clone_from(&AtomicState::InProgress);

        // 2. Сохранение точки отката
        let rollback_point = self.create_rollback_point().await?;

        // 3. Выполнение операции
        match self.perform_operation().await {
            Ok(()) => {
                // 4. Коммит
                self.state.write().await.clone_from(&AtomicState::Committed);
                Ok(())
            },
            Err(e) => {
                // 5. Автоматический откат
                error!("Operation failed, rolling back: {}", e);
                self.rollback(rollback_point).await?;
                self.state.write().await.clone_from(&AtomicState::RolledBack);
                Err(e)
            }
        }
    }
}
```

## 🎯 КРИТЕРИИ УСПЕХА

- Clearing window обрабатывается < 5 минут
- Netting efficiency > 70%
- Zero loss tolerance (нулевая потеря средств)
- Автоматический откат при любой ошибке
- 100% аудит всех операций
- Поддержка ручного вмешательства