/**
 * DelTran System Limits and Configuration Constants
 * Based on backend configuration
 */

export const LIMITS = {
  // Payment Limits
  MIN_AMOUNT: 0.01,
  MAX_AMOUNT: 10_000_000,
  DAILY_LIMIT: 50_000_000,

  // Rate Limiting
  RATE_LIMIT_RPS: 1000,
  RATE_LIMIT_BURST: 2000,

  // Settlement Configuration
  SETTLEMENT_WINDOW_HOURS: 6,
  SETTLEMENT_MIN_BATCH: 10,
  SETTLEMENT_MAX_BATCH: 1000,

  // FX Configuration
  FX_UPDATE_INTERVAL: 60, // seconds
  FX_MARKUP_BPS: 25, // 0.25%

  // Risk Thresholds
  RISK_HIGH_THRESHOLD: 0.7,
  RISK_MEDIUM_THRESHOLD: 0.4,

  // Cache Configuration
  CACHE_TTL: 300, // seconds

  // Fee Configuration
  BASE_FEE_PERCENTAGE: 0.0025, // 0.25%
  MIN_FEE: 1.0,
  MAX_FEE: 100.0,
} as const

export const API_CONFIG = {
  BASE_URL: process.env.NEXT_PUBLIC_API_URL || 'http://localhost:8080',
  TENANT_ID: 'deltran-premium',
  TIMEOUT: 30000, // 30 seconds
} as const

export const WEBSOCKET_CONFIG = {
  URL: process.env.NEXT_PUBLIC_WS_URL || 'ws://localhost:8080/ws',
  HEARTBEAT_INTERVAL: 30000, // 30 seconds
  RECONNECT_DELAY_BASE: 1000,
  RECONNECT_MAX_DELAY: 30000,
  MAX_RECONNECT_ATTEMPTS: 10,
} as const

export const PROMETHEUS_ENDPOINTS = {
  GATEWAY: 'http://localhost:9090/metrics',
  LEDGER: 'http://localhost:9091/metrics',
  SETTLEMENT: 'http://localhost:9092/metrics',
  RISK: 'http://localhost:9093/metrics',
} as const

export const CACHE_TIMES = {
  TRANSACTIONS: 30, // seconds
  METRICS: 2,
  RISK_DATA: 10,
  FX_RATES: 60,
  BATCHES: 300,
} as const

export const VALIDATION_PATTERNS = {
  AMOUNT: /^\d+(\.\d{1,2})?$/,
  BIC: /^[A-Z]{6}[A-Z0-9]{2}([A-Z0-9]{3})?$/,
  REFERENCE: /^[a-zA-Z0-9\s\-\/]{1,140}$/,
} as const

export const ALERT_THRESHOLDS = {
  ERROR_RATE: 5, // percent
  QUEUE_DEPTH: 1000,
  SPIKE_PROBABILITY: 0.7,
  RATE_LIMIT_WARNING: 800, // RPS
  LATENCY_P95_WARNING: 100, // ms
  LATENCY_P95_CRITICAL: 200, // ms
} as const
