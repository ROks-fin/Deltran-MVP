// fx/config.rs
// FX Orchestration Configuration with Production Defaults

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FXConfig {
    pub market_makers: MarketMakerConfig,
    pub pvp: PvPConfig,
    pub rate_discovery: RateDiscoveryConfig,
    pub currency_pairs: CurrencyPairConfig,
    pub risk: RiskConfig,
    pub performance: PerformanceConfig,
    pub regulatory: RegulatoryConfig,
    pub integration: IntegrationConfig,
}

impl Default for FXConfig {
    fn default() -> Self {
        Self {
            market_makers: MarketMakerConfig::default(),
            pvp: PvPConfig::default(),
            rate_discovery: RateDiscoveryConfig::default(),
            currency_pairs: CurrencyPairConfig::default(),
            risk: RiskConfig::default(),
            performance: PerformanceConfig::default(),
            regulatory: RegulatoryConfig::default(),
            integration: IntegrationConfig::default(),
        }
    }
}

// ==================== Market Maker Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MarketMakerConfig {
    /// Number of market makers for MVP (3-5)
    pub target_mm_count: u32,

    /// Onboarding requirements
    pub require_kyb: bool,
    pub require_lei: bool,
    pub require_license: bool,
    pub require_fsra_whitelist: bool,

    /// Supported protocols
    pub supported_protocols: Vec<ProtocolType>,

    /// Quote selection - weighted scoring
    pub quote_scoring: QuoteScoringWeights,

    /// RFQ timeout (500ms default, 1s for exotics)
    pub rfq_timeout_ms: u64,
    pub rfq_timeout_exotic_ms: u64,

    /// Allow split execution across multiple MMs
    pub allow_split_execution: bool,
    pub max_split_mms: u32,

    /// Rate validation against external references
    pub validate_against_mid_ref: bool,
    pub max_deviation_from_mid_bps: u32, // 50 bps default

    /// Stale quote handling
    pub stale_quote_threshold_ms: u64, // 1000ms standard, 3000ms exotic
    pub stale_quote_threshold_exotic_ms: u64,
    pub stale_quote_action: StaleQuoteAction,

    /// Performance monitoring
    pub health_check_interval_seconds: u64, // 10s
    pub auto_suspension: AutoSuspensionConfig,
    pub enable_leaderboard: bool,
}

impl Default for MarketMakerConfig {
    fn default() -> Self {
        Self {
            target_mm_count: 5,
            require_kyb: true,
            require_lei: true,
            require_license: true,
            require_fsra_whitelist: true,
            supported_protocols: vec![
                ProtocolType::GRPC,
                ProtocolType::REST,
                ProtocolType::WebSocket,
            ],
            quote_scoring: QuoteScoringWeights::default(),
            rfq_timeout_ms: 500,
            rfq_timeout_exotic_ms: 1000,
            allow_split_execution: true,
            max_split_mms: 3,
            validate_against_mid_ref: true,
            max_deviation_from_mid_bps: 50,
            stale_quote_threshold_ms: 1000,
            stale_quote_threshold_exotic_ms: 3000,
            stale_quote_action: StaleQuoteAction::RejectAndRerequest,
            health_check_interval_seconds: 10,
            auto_suspension: AutoSuspensionConfig::default(),
            enable_leaderboard: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ProtocolType {
    GRPC,
    REST,
    WebSocket,
    FIX,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct QuoteScoringWeights {
    /// weight_rate (0.5 default)
    pub w_rate: f32,
    /// weight_liquidity (0.25 default)
    pub w_liquidity: f32,
    /// weight_latency (0.15 default)
    pub w_latency: f32,
    /// weight_fill_rate (0.10 default)
    pub w_fill_rate: f32,
}

impl Default for QuoteScoringWeights {
    fn default() -> Self {
        Self {
            w_rate: 0.5,
            w_liquidity: 0.25,
            w_latency: 0.15,
            w_fill_rate: 0.10,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StaleQuoteAction {
    Reject,
    RejectAndRerequest,
    AcceptWithWarning,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AutoSuspensionConfig {
    pub enabled: bool,
    pub min_uptime_percent: f32, // 99.5%
    pub max_reject_rate_percent: f32, // 10%
    pub max_spread_deviation_percent: f32, // 50% above median
}

impl Default for AutoSuspensionConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            min_uptime_percent: 99.5,
            max_reject_rate_percent: 10.0,
            max_spread_deviation_percent: 50.0,
        }
    }
}

// ==================== PvP Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PvPConfig {
    /// PvP mode (2-phase commit with pre-fund/limits)
    pub default_mode: PvPModeType,

    /// Allow partial settlement for strongly-connected sub-batches
    pub allow_partial_settlement: bool,

    /// PvP timeout (60s default, 30-120s configurable)
    pub timeout_seconds: u64,
    pub min_timeout_seconds: u64,
    pub max_timeout_seconds: u64,

    /// Failure handling
    pub auto_retry: bool,
    pub max_retry_attempts: u32,
    pub retry_backoff_ms: u64,

    /// Settlement accounts
    pub use_external_bank_accounts: bool,
    pub require_escrow_accounts: bool,
    pub require_pre_funding: bool,

    /// Insufficient funds handling
    pub insufficient_funds_action: InsufficientFundsAction,

    /// Settlement finality
    pub require_external_confirmation: bool,
    pub require_crypto_proofs: bool,
    pub require_digital_signatures: bool,

    /// T+0/T+1/T+2
    pub default_settlement_window: SettlementWindow,
}

impl Default for PvPConfig {
    fn default() -> Self {
        Self {
            default_mode: PvPModeType::TwoPhaseCommit,
            allow_partial_settlement: true,
            timeout_seconds: 60,
            min_timeout_seconds: 30,
            max_timeout_seconds: 120,
            auto_retry: true,
            max_retry_attempts: 3,
            retry_backoff_ms: 1000,
            use_external_bank_accounts: true,
            require_escrow_accounts: true,
            require_pre_funding: true,
            insufficient_funds_action: InsufficientFundsAction::RejectUpfront,
            require_external_confirmation: true,
            require_crypto_proofs: true,
            require_digital_signatures: true,
            default_settlement_window: SettlementWindow::T0,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PvPModeType {
    TwoPhaseCommit,
    Simultaneous,
    Sequential,
    Escrow,
    CLS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum InsufficientFundsAction {
    RejectUpfront,
    LockPartial,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementWindow {
    T0,
    T1,
    T2,
}

// ==================== Rate Discovery Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RateDiscoveryConfig {
    /// External reference sources
    pub external_sources: Vec<RateSource>,

    /// Fallback strategy when all MMs offline
    pub fallback_strategy: FallbackStrategy,

    /// Refresh frequency
    pub refresh_interval_seconds: u64, // 60s default
    pub enable_realtime: bool,

    /// Mid-market calculation
    pub calculate_mid_from_multiple_sources: bool,
    pub mid_calculation_method: MidCalculationMethod,

    /// Spread limits
    pub max_spread_standard_bps: u32, // 100 bps
    pub max_spread_exotic_bps: u32,   // 200 bps
    pub require_manual_approval_above_limit: bool,

    /// Pricing transparency
    pub show_markup_breakdown: bool,

    /// Pricing by amount
    pub pricing_model: PricingModel,

    /// Last-look
    pub allow_last_look: bool,
    pub last_look_duration_ms: u64, // 250ms

    /// Market depth
    pub market_depth_levels: u32, // top 5
    pub aggregate_liquidity_across_mms: bool,

    /// Liquidity exhaustion
    pub liquidity_exhaustion_strategy: LiquidityStrategy,

    /// Reservation
    pub enable_short_reservation: bool,
    pub reservation_duration_ms: u64, // 1000ms
}

impl Default for RateDiscoveryConfig {
    fn default() -> Self {
        Self {
            external_sources: vec![
                RateSource::Bloomberg,
                RateSource::Refinitiv,
                RateSource::ECB,
            ],
            fallback_strategy: FallbackStrategy::QueueWithIndicative,
            refresh_interval_seconds: 60,
            enable_realtime: true,
            calculate_mid_from_multiple_sources: true,
            mid_calculation_method: MidCalculationMethod::MedianOfMeans,
            max_spread_standard_bps: 100,
            max_spread_exotic_bps: 200,
            require_manual_approval_above_limit: true,
            show_markup_breakdown: false, // Optional for B2B
            pricing_model: PricingModel::TieredPlusDynamic,
            allow_last_look: true,
            last_look_duration_ms: 250,
            market_depth_levels: 5,
            aggregate_liquidity_across_mms: true,
            liquidity_exhaustion_strategy: LiquidityStrategy::SplitThenQueue,
            enable_short_reservation: true,
            reservation_duration_ms: 1000,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum RateSource {
    Bloomberg,
    Refinitiv,
    ECB,
    XE,
    CFBenchmarks,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum FallbackStrategy {
    QueueWithIndicative,
    RouteToDesignatedLP,
    Reject,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MidCalculationMethod {
    MedianOfMeans,
    WeightedAverage,
    Simple,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum PricingModel {
    Flat,
    Tiered,
    TieredPlusDynamic,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LiquidityStrategy {
    Split,
    Queue,
    SplitThenQueue,
    ReRFQ,
}

// ==================== Currency Pair Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyPairConfig {
    /// Priority pairs for MVP
    pub priority_pairs: Vec<CurrencyPair>,

    /// Support exotic pairs
    pub support_exotic_pairs: bool,

    /// Allow triangulation
    pub allow_triangulation: bool,

    /// Min liquidity per pair (notional/day)
    pub min_liquidity_per_pair_million: u32, // 1-5M

    /// Settlement currencies
    pub settlement_currencies: Vec<String>,

    /// Multi-currency nostro
    pub use_multi_currency_nostro: bool,

    /// Holiday handling
    pub holiday_handling: HolidayHandling,
}

impl Default for CurrencyPairConfig {
    fn default() -> Self {
        Self {
            priority_pairs: vec![
                // G10
                CurrencyPair::new("EUR", "USD"),
                CurrencyPair::new("GBP", "USD"),
                CurrencyPair::new("USD", "JPY"),
                CurrencyPair::new("USD", "CHF"),
                CurrencyPair::new("AUD", "USD"),
                CurrencyPair::new("USD", "CAD"),
                // Priority corridors
                CurrencyPair::new("AED", "USD"),
                CurrencyPair::new("AED", "EUR"),
                CurrencyPair::new("AED", "INR"),
                CurrencyPair::new("USD", "INR"),
                CurrencyPair::new("USD", "ILS"),
                CurrencyPair::new("EUR", "ILS"),
            ],
            support_exotic_pairs: true,
            allow_triangulation: true,
            min_liquidity_per_pair_million: 5,
            settlement_currencies: vec![
                "USD".to_string(),
                "EUR".to_string(),
                "AED".to_string(),
                "GBP".to_string(),
                "ILS".to_string(),
                "INR".to_string(),
            ],
            use_multi_currency_nostro: true,
            holiday_handling: HolidayHandling::PostponeWithAutoTransfer,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CurrencyPair {
    pub base: String,
    pub quote: String,
}

impl CurrencyPair {
    pub fn new(base: &str, quote: &str) -> Self {
        Self {
            base: base.to_string(),
            quote: quote.to_string(),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HolidayHandling {
    Reject,
    PostponeWithAutoTransfer,
    AlternativeRoute,
}

// ==================== Risk Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RiskConfig {
    /// Pre-trade checks
    pub pre_trade_checks: PreTradeChecks,

    /// Per-client limits
    pub enable_per_client_limits: bool,
    pub default_daily_limit_usd: Decimal,
    pub default_monthly_limit_usd: Decimal,
    pub default_per_trade_limit_usd: Decimal,

    /// MM credit risk
    pub mm_credit_handling: MMCreditHandling,

    /// Margin requirements
    pub require_margin_for_high_risk: bool,

    /// Intraday monitoring
    pub intraday_monitoring_realtime: bool,
    pub intraday_reconciliation_interval_hours: u64,

    /// Auto-hedging
    pub enable_auto_hedging: bool,
    pub auto_hedge_threshold_delta: Decimal,

    /// Large orders
    pub large_order_threshold_usd: Decimal,
    pub large_order_handling: LargeOrderHandling,

    /// VaR calculation
    pub enable_var_calculation: bool,

    /// Settlement risk
    pub settlement_fail_handling: SettlementFailHandling,
    pub enable_settlement_guarantee_fund: bool,
    pub herstatt_mitigation: HerstattMitigation,
}

impl Default for RiskConfig {
    fn default() -> Self {
        Self {
            pre_trade_checks: PreTradeChecks::default(),
            enable_per_client_limits: true,
            default_daily_limit_usd: Decimal::new(1_000_000, 0),
            default_monthly_limit_usd: Decimal::new(10_000_000, 0),
            default_per_trade_limit_usd: Decimal::new(100_000, 0),
            mm_credit_handling: MMCreditHandling::PreFundedOrCreditLines,
            require_margin_for_high_risk: true,
            intraday_monitoring_realtime: true,
            intraday_reconciliation_interval_hours: 1,
            enable_auto_hedging: true,
            auto_hedge_threshold_delta: Decimal::new(100_000, 0),
            large_order_threshold_usd: Decimal::new(500_000, 0),
            large_order_handling: LargeOrderHandling::ManualApproveAndAutoSplit,
            enable_var_calculation: true,
            settlement_fail_handling: SettlementFailHandling::RetryEscalateCompensate,
            enable_settlement_guarantee_fund: false, // MVP - future
            herstatt_mitigation: HerstattMitigation::PvPOnly,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PreTradeChecks {
    pub check_credit_limit: bool,
    pub check_exposure_limit: bool,
    pub check_velocity: bool,
    pub check_counterparty_rating: bool,
    pub check_sanctions: bool,
}

impl Default for PreTradeChecks {
    fn default() -> Self {
        Self {
            check_credit_limit: true,
            check_exposure_limit: true,
            check_velocity: true,
            check_counterparty_rating: true,
            check_sanctions: true,
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum MMCreditHandling {
    PreFundedOrCreditLines,
    GuaranteeOnly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum LargeOrderHandling {
    ManualApprove,
    AutoSplit,
    ManualApproveAndAutoSplit,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SettlementFailHandling {
    Retry,
    Escalate,
    Compensate,
    RetryEscalateCompensate,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum HerstattMitigation {
    PvPOnly,
    PvPPlusCutoff,
}

// ==================== Performance Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PerformanceConfig {
    /// Latency targets
    pub target_quote_latency_ms: u64, // < 500ms
    pub target_quote_p99_ms: u64, // <= 1s
    pub target_pvp_latency_ms: u64, // < 5s
    pub target_complex_split_p99_ms: u64, // <= 10s

    /// Throughput
    pub expected_daily_volume: u32, // 1-5k trades
    pub scale_target_daily_volume: u32, // 10k+
    pub peak_tps: u32, // 100 TPS
    pub sustained_tps: u32, // 10-30 TPS

    /// Burst handling
    pub enable_burst_buffers: bool,
    pub enable_prioritization: bool,

    /// Availability
    pub target_sla_percent: f32, // 99.95%
    pub allow_scheduled_downtime: bool,
    pub enable_ha_active_active: bool,
    pub min_validator_nodes: u32, // 3
}

impl Default for PerformanceConfig {
    fn default() -> Self {
        Self {
            target_quote_latency_ms: 500,
            target_quote_p99_ms: 1000,
            target_pvp_latency_ms: 5000,
            target_complex_split_p99_ms: 10000,
            expected_daily_volume: 5000,
            scale_target_daily_volume: 10000,
            peak_tps: 100,
            sustained_tps: 30,
            enable_burst_buffers: true,
            enable_prioritization: true,
            target_sla_percent: 99.95,
            allow_scheduled_downtime: true,
            enable_ha_active_active: true,
            min_validator_nodes: 3,
        }
    }
}

// ==================== Regulatory Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RegulatoryConfig {
    /// Trade reporting
    pub enable_trade_reporting: bool,
    pub reporting_frequency: ReportingFrequency,
    pub reporting_format: Vec<ReportingFormat>,

    /// Audit trail
    pub require_immutable_audit_trail: bool,
    pub enable_merkle_proofs: bool,
    pub enable_regulatory_read_only_api: bool,

    /// Best execution
    pub enable_tca: bool,
    pub enable_post_trade_tca: bool,
    pub require_best_execution_policy: bool,

    /// AML/Sanctions
    pub screen_market_makers: bool,
    pub enhanced_monitoring_high_risk_countries: bool,
    pub enable_edd_for_large_trades: bool,
    pub edd_threshold_usd: Decimal,
}

impl Default for RegulatoryConfig {
    fn default() -> Self {
        Self {
            enable_trade_reporting: true,
            reporting_frequency: ReportingFrequency::TPlusOne,
            reporting_format: vec![
                ReportingFormat::MiFIDLike,
                ReportingFormat::FSRARegLab,
            ],
            require_immutable_audit_trail: true,
            enable_merkle_proofs: true,
            enable_regulatory_read_only_api: true,
            enable_tca: true,
            enable_post_trade_tca: true,
            require_best_execution_policy: true,
            screen_market_makers: true,
            enhanced_monitoring_high_risk_countries: true,
            enable_edd_for_large_trades: true,
            edd_threshold_usd: Decimal::new(100_000, 0),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportingFrequency {
    RealTime,
    TPlusOne,
    Weekly,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum ReportingFormat {
    MiFIDLike,
    FSRARegLab,
    EMIR,
}

// ==================== Integration Configuration ====================

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct IntegrationConfig {
    /// Payment gateway integration
    pub gateway_sync_protocol: SyncProtocol,
    pub gateway_async_protocol: AsyncProtocol,

    /// Ledger integration
    pub ledger_protocol: SyncProtocol,
    pub allow_direct_db_access: bool,

    /// Client streaming
    pub enable_client_streaming: bool,
    pub client_streaming_protocols: Vec<StreamingProtocol>,

    /// Multi-currency netting
    pub netting_strategy: NettingStrategy,

    /// External systems
    pub enable_cls_interface: bool,
    pub enable_swift_mt300_304: bool,
    pub enable_iso20022: bool,
    pub enable_trade_repositories: bool,
    pub trade_repositories: Vec<TradeRepository>,
}

impl Default for IntegrationConfig {
    fn default() -> Self {
        Self {
            gateway_sync_protocol: SyncProtocol::GRPC,
            gateway_async_protocol: AsyncProtocol::NATS,
            ledger_protocol: SyncProtocol::GRPC,
            allow_direct_db_access: false,
            enable_client_streaming: true,
            client_streaming_protocols: vec![
                StreamingProtocol::WebSocket,
                StreamingProtocol::SSE,
            ],
            netting_strategy: NettingStrategy::NetPerCurrency,
            enable_cls_interface: true, // Phase 2+
            enable_swift_mt300_304: true,
            enable_iso20022: true,
            enable_trade_repositories: true,
            trade_repositories: vec![
                TradeRepository::DTCC,
                TradeRepository::REGISTR,
            ],
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum SyncProtocol {
    GRPC,
    REST,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum AsyncProtocol {
    Kafka,
    NATS,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum StreamingProtocol {
    WebSocket,
    SSE,
    GRPC,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum NettingStrategy {
    ConvertToSingleCurrency,
    NetPerCurrency,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TradeRepository {
    DTCC,
    REGISTR,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = FXConfig::default();

        // Market Makers
        assert_eq!(config.market_makers.target_mm_count, 5);
        assert_eq!(config.market_makers.rfq_timeout_ms, 500);
        assert_eq!(config.market_makers.quote_scoring.w_rate, 0.5);

        // PvP
        assert_eq!(config.pvp.timeout_seconds, 60);
        assert!(config.pvp.allow_partial_settlement);

        // Rate Discovery
        assert_eq!(config.rate_discovery.max_spread_standard_bps, 100);
        assert_eq!(config.rate_discovery.last_look_duration_ms, 250);

        // Performance
        assert_eq!(config.performance.target_quote_latency_ms, 500);
        assert_eq!(config.performance.target_sla_percent, 99.95);
    }

    #[test]
    fn test_scoring_weights_sum() {
        let weights = QuoteScoringWeights::default();
        let sum = weights.w_rate + weights.w_liquidity + weights.w_latency + weights.w_fill_rate;
        assert!((sum - 1.0).abs() < 0.001);
    }
}
