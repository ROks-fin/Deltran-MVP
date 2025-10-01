//! Rate Limiting and DDoS Protection
//!
//! Implements token bucket and sliding window rate limiters for:
//! - Per-IP rate limiting
//! - Per-account rate limiting
//! - Global rate limiting
//! - Adaptive rate limiting based on load

use std::collections::HashMap;
use std::net::IpAddr;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::RwLock;
use tracing::{info, warn};

/// Rate limiter configuration
#[derive(Debug, Clone)]
pub struct RateLimiterConfig {
    /// Maximum requests per window
    pub max_requests: u32,

    /// Time window duration
    pub window_duration: Duration,

    /// Burst size (token bucket capacity)
    pub burst_size: u32,

    /// Token refill rate (tokens per second)
    pub refill_rate: f64,

    /// Enable adaptive rate limiting
    pub adaptive: bool,

    /// Threshold for triggering adaptive limiting (0.0-1.0)
    pub adaptive_threshold: f64,
}

impl Default for RateLimiterConfig {
    fn default() -> Self {
        Self {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            burst_size: 100,
            refill_rate: 16.67, // ~1000 requests per minute
            adaptive: true,
            adaptive_threshold: 0.8,
        }
    }
}

/// Rate limiter result
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RateLimitResult {
    /// Request allowed
    Allowed,

    /// Request denied (rate limit exceeded)
    Denied {
        retry_after: Duration,
    },

    /// Request denied (system overload)
    SystemOverload,
}

/// Token bucket rate limiter
#[derive(Debug)]
struct TokenBucket {
    /// Current token count
    tokens: f64,

    /// Maximum tokens (burst size)
    capacity: f64,

    /// Token refill rate (per second)
    refill_rate: f64,

    /// Last refill timestamp
    last_refill: Instant,
}

impl TokenBucket {
    fn new(capacity: f64, refill_rate: f64) -> Self {
        Self {
            tokens: capacity,
            capacity,
            refill_rate,
            last_refill: Instant::now(),
        }
    }

    fn try_consume(&mut self, tokens: f64) -> bool {
        // Refill tokens based on elapsed time
        let now = Instant::now();
        let elapsed = now.duration_since(self.last_refill).as_secs_f64();
        let new_tokens = elapsed * self.refill_rate;
        self.tokens = (self.tokens + new_tokens).min(self.capacity);
        self.last_refill = now;

        // Try to consume tokens
        if self.tokens >= tokens {
            self.tokens -= tokens;
            true
        } else {
            false
        }
    }

    fn tokens_until_ready(&self, required: f64) -> Duration {
        if self.tokens >= required {
            Duration::ZERO
        } else {
            let deficit = required - self.tokens;
            let seconds = deficit / self.refill_rate;
            Duration::from_secs_f64(seconds)
        }
    }
}

/// Sliding window rate limiter
#[derive(Debug)]
struct SlidingWindow {
    /// Request timestamps
    timestamps: Vec<Instant>,

    /// Maximum requests
    max_requests: usize,

    /// Window duration
    window_duration: Duration,
}

impl SlidingWindow {
    fn new(max_requests: usize, window_duration: Duration) -> Self {
        Self {
            timestamps: Vec::with_capacity(max_requests),
            max_requests,
            window_duration,
        }
    }

    fn try_add(&mut self) -> bool {
        let now = Instant::now();
        let cutoff = now - self.window_duration;

        // Remove old timestamps
        self.timestamps.retain(|&ts| ts > cutoff);

        // Check if we can add new request
        if self.timestamps.len() < self.max_requests {
            self.timestamps.push(now);
            true
        } else {
            false
        }
    }

    fn time_until_ready(&self) -> Duration {
        if self.timestamps.len() < self.max_requests {
            Duration::ZERO
        } else {
            let oldest = self.timestamps[0];
            let cutoff = oldest + self.window_duration;
            cutoff.duration_since(Instant::now())
        }
    }
}

/// Rate limiter entry
#[derive(Debug)]
struct RateLimitEntry {
    token_bucket: TokenBucket,
    sliding_window: SlidingWindow,
    last_access: Instant,
}

/// Multi-tier rate limiter
pub struct RateLimiter {
    /// Configuration
    config: RateLimiterConfig,

    /// Per-IP rate limits
    ip_limits: Arc<RwLock<HashMap<IpAddr, RateLimitEntry>>>,

    /// Per-account rate limits
    account_limits: Arc<RwLock<HashMap<String, RateLimitEntry>>>,

    /// Global rate limiter
    global: Arc<RwLock<RateLimitEntry>>,

    /// System load (0.0-1.0)
    system_load: Arc<RwLock<f64>>,
}

impl RateLimiter {
    /// Create new rate limiter
    pub fn new(config: RateLimiterConfig) -> Self {
        let global_entry = RateLimitEntry {
            token_bucket: TokenBucket::new(
                config.burst_size as f64 * 10.0, // 10x capacity for global
                config.refill_rate * 10.0,
            ),
            sliding_window: SlidingWindow::new(
                config.max_requests as usize * 10,
                config.window_duration,
            ),
            last_access: Instant::now(),
        };

        Self {
            config,
            ip_limits: Arc::new(RwLock::new(HashMap::new())),
            account_limits: Arc::new(RwLock::new(HashMap::new())),
            global: Arc::new(RwLock::new(global_entry)),
            system_load: Arc::new(RwLock::new(0.0)),
        }
    }

    /// Check IP rate limit
    pub async fn check_ip(&self, ip: IpAddr) -> RateLimitResult {
        // Check global limit first
        if let RateLimitResult::Denied { retry_after } = self.check_global().await {
            return RateLimitResult::Denied { retry_after };
        }

        // Check system load
        if self.config.adaptive {
            let load = *self.system_load.read().await;
            if load > self.config.adaptive_threshold {
                warn!("System overloaded: {:.2}%", load * 100.0);
                return RateLimitResult::SystemOverload;
            }
        }

        // Check IP-specific limit
        let mut limits = self.ip_limits.write().await;
        let entry = limits.entry(ip).or_insert_with(|| RateLimitEntry {
            token_bucket: TokenBucket::new(
                self.config.burst_size as f64,
                self.config.refill_rate,
            ),
            sliding_window: SlidingWindow::new(
                self.config.max_requests as usize,
                self.config.window_duration,
            ),
            last_access: Instant::now(),
        });

        entry.last_access = Instant::now();

        // Token bucket check
        if !entry.token_bucket.try_consume(1.0) {
            let retry_after = entry.token_bucket.tokens_until_ready(1.0);
            warn!("IP rate limit exceeded: {}", ip);
            return RateLimitResult::Denied { retry_after };
        }

        // Sliding window check
        if !entry.sliding_window.try_add() {
            let retry_after = entry.sliding_window.time_until_ready();
            warn!("IP rate limit exceeded (window): {}", ip);
            return RateLimitResult::Denied { retry_after };
        }

        RateLimitResult::Allowed
    }

    /// Check account rate limit
    pub async fn check_account(&self, account_id: &str) -> RateLimitResult {
        let mut limits = self.account_limits.write().await;
        let entry = limits
            .entry(account_id.to_string())
            .or_insert_with(|| RateLimitEntry {
                token_bucket: TokenBucket::new(
                    self.config.burst_size as f64 * 2.0, // 2x for authenticated users
                    self.config.refill_rate * 2.0,
                ),
                sliding_window: SlidingWindow::new(
                    self.config.max_requests as usize * 2,
                    self.config.window_duration,
                ),
                last_access: Instant::now(),
            });

        entry.last_access = Instant::now();

        if !entry.token_bucket.try_consume(1.0) {
            let retry_after = entry.token_bucket.tokens_until_ready(1.0);
            warn!("Account rate limit exceeded: {}", account_id);
            return RateLimitResult::Denied { retry_after };
        }

        if !entry.sliding_window.try_add() {
            let retry_after = entry.sliding_window.time_until_ready();
            warn!("Account rate limit exceeded (window): {}", account_id);
            return RateLimitResult::Denied { retry_after };
        }

        RateLimitResult::Allowed
    }

    /// Check global rate limit
    async fn check_global(&self) -> RateLimitResult {
        let mut global = self.global.write().await;

        if !global.token_bucket.try_consume(1.0) {
            let retry_after = global.token_bucket.tokens_until_ready(1.0);
            warn!("Global rate limit exceeded");
            return RateLimitResult::Denied { retry_after };
        }

        if !global.sliding_window.try_add() {
            let retry_after = global.sliding_window.time_until_ready();
            warn!("Global rate limit exceeded (window)");
            return RateLimitResult::Denied { retry_after };
        }

        RateLimitResult::Allowed
    }

    /// Update system load (0.0-1.0)
    pub async fn update_system_load(&self, load: f64) {
        let mut system_load = self.system_load.write().await;
        *system_load = load.clamp(0.0, 1.0);

        if load > self.config.adaptive_threshold {
            info!("System load high: {:.2}%", load * 100.0);
        }
    }

    /// Cleanup old entries
    pub async fn cleanup(&self, max_age: Duration) {
        let cutoff = Instant::now() - max_age;

        // Cleanup IP limits
        let mut ip_limits = self.ip_limits.write().await;
        ip_limits.retain(|_, entry| entry.last_access > cutoff);

        // Cleanup account limits
        let mut account_limits = self.account_limits.write().await;
        account_limits.retain(|_, entry| entry.last_access > cutoff);

        info!(
            "Rate limiter cleanup: {} IPs, {} accounts",
            ip_limits.len(),
            account_limits.len()
        );
    }

    /// Start cleanup task
    pub fn start_cleanup_task(self: Arc<Self>, interval: Duration, max_age: Duration) {
        tokio::spawn(async move {
            let mut interval = tokio::time::interval(interval);
            loop {
                interval.tick().await;
                self.cleanup(max_age).await;
            }
        });
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_token_bucket() {
        let mut bucket = TokenBucket::new(10.0, 1.0); // 10 tokens, refill 1/sec

        // Consume tokens
        assert!(bucket.try_consume(5.0));
        assert_eq!(bucket.tokens, 5.0);

        // Consume more
        assert!(bucket.try_consume(5.0));
        assert_eq!(bucket.tokens, 0.0);

        // Should fail
        assert!(!bucket.try_consume(1.0));

        // Wait and refill
        std::thread::sleep(Duration::from_millis(2000));
        assert!(bucket.try_consume(1.0));
    }

    #[test]
    fn test_sliding_window() {
        let mut window = SlidingWindow::new(5, Duration::from_secs(1));

        // Add requests
        for _ in 0..5 {
            assert!(window.try_add());
        }

        // Should fail
        assert!(!window.try_add());

        // Wait and retry
        std::thread::sleep(Duration::from_millis(1100));
        assert!(window.try_add());
    }

    #[tokio::test]
    async fn test_rate_limiter_ip() {
        let config = RateLimiterConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(1),
            burst_size: 5,
            refill_rate: 1.0,
            adaptive: false,
            adaptive_threshold: 0.8,
        };

        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Should allow burst
        for _ in 0..5 {
            assert_eq!(limiter.check_ip(ip).await, RateLimitResult::Allowed);
        }

        // Should deny
        let result = limiter.check_ip(ip).await;
        assert!(matches!(result, RateLimitResult::Denied { .. }));
    }

    #[tokio::test]
    async fn test_rate_limiter_account() {
        let config = RateLimiterConfig {
            max_requests: 10,
            window_duration: Duration::from_secs(1),
            burst_size: 5,
            refill_rate: 1.0,
            adaptive: false,
            adaptive_threshold: 0.8,
        };

        let limiter = RateLimiter::new(config);
        let account = "test_account";

        // Should allow (2x burst for accounts)
        for _ in 0..10 {
            assert_eq!(
                limiter.check_account(account).await,
                RateLimitResult::Allowed
            );
        }

        // Should deny
        let result = limiter.check_account(account).await;
        assert!(matches!(result, RateLimitResult::Denied { .. }));
    }

    #[tokio::test]
    async fn test_adaptive_rate_limiting() {
        let config = RateLimiterConfig {
            max_requests: 1000,
            window_duration: Duration::from_secs(60),
            burst_size: 100,
            refill_rate: 16.67,
            adaptive: true,
            adaptive_threshold: 0.8,
        };

        let limiter = RateLimiter::new(config);
        let ip: IpAddr = "127.0.0.1".parse().unwrap();

        // Normal load
        limiter.update_system_load(0.5).await;
        assert_eq!(limiter.check_ip(ip).await, RateLimitResult::Allowed);

        // High load
        limiter.update_system_load(0.9).await;
        assert_eq!(limiter.check_ip(ip).await, RateLimitResult::SystemOverload);
    }
}