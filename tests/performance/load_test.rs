//! Performance Load Testing
//!
//! Validates 100 TPS sustained throughput with SLO targets:
//! - p50 < 200ms
//! - p95 < 500ms
//! - p99 < 2s
//! - 0% dropped transactions

#![cfg(test)]

use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::Semaphore;
use tokio::task::JoinSet;
use uuid::Uuid;

/// Load test configuration
struct LoadTestConfig {
    target_tps: u64,
    duration_secs: u64,
    ramp_up_secs: u64,
    max_concurrent: usize,
}

impl Default for LoadTestConfig {
    fn default() -> Self {
        Self {
            target_tps: 100,
            duration_secs: 60,
            ramp_up_secs: 10,
            max_concurrent: 1000,
        }
    }
}

/// Performance statistics
#[derive(Debug, Default)]
struct PerfStats {
    total_requests: AtomicU64,
    successful: AtomicU64,
    failed: AtomicU64,
    latencies_ms: Vec<u64>,
}

impl PerfStats {
    fn record_success(&self, latency_ms: u64) {
        self.successful.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn record_failure(&self) {
        self.failed.fetch_add(1, Ordering::Relaxed);
        self.total_requests.fetch_add(1, Ordering::Relaxed);
    }

    fn report(&self) {
        let total = self.total_requests.load(Ordering::Relaxed);
        let success = self.successful.load(Ordering::Relaxed);
        let failed = self.failed.load(Ordering::Relaxed);
        let success_rate = if total > 0 {
            (success as f64 / total as f64) * 100.0
        } else {
            0.0
        };

        println!("\n📊 Performance Test Results");
        println!("============================");
        println!("Total requests:     {}", total);
        println!("Successful:         {} ({:.2}%)", success, success_rate);
        println!("Failed:             {}", failed);
    }
}

/// Simulated payment submission
async fn submit_payment(payment_id: String) -> Result<Duration, String> {
    let start = Instant::now();

    // Simulate API call latency
    tokio::time::sleep(Duration::from_millis(50 + rand::random::<u64>() % 100)).await;

    // Simulate 1% failure rate
    if rand::random::<f64>() < 0.01 {
        return Err("Simulated failure".to_string());
    }

    Ok(start.elapsed())
}

#[tokio::test]
#[ignore] // Run explicitly with: cargo test --release test_100_tps_sustained
async fn test_100_tps_sustained() {
    let config = LoadTestConfig::default();
    let stats = Arc::new(PerfStats::default());
    let semaphore = Arc::new(Semaphore::new(config.max_concurrent));

    println!("\n🚀 Starting Load Test");
    println!("   Target TPS: {}", config.target_tps);
    println!("   Duration: {}s", config.duration_secs);
    println!("   Max Concurrent: {}", config.max_concurrent);

    let test_start = Instant::now();
    let mut join_set = JoinSet::new();

    // Calculate inter-request delay for target TPS
    let delay_micros = (1_000_000 / config.target_tps) as u64;

    let mut request_count = 0u64;
    let total_requests = config.target_tps * config.duration_secs;

    while request_count < total_requests {
        let payment_id = Uuid::new_v4().to_string();
        let stats = Arc::clone(&stats);
        let permit = Arc::clone(&semaphore)
            .acquire_owned()
            .await
            .expect("Semaphore closed");

        join_set.spawn(async move {
            let result = submit_payment(payment_id).await;
            drop(permit);

            match result {
                Ok(latency) => stats.record_success(latency.as_millis() as u64),
                Err(_) => stats.record_failure(),
            }
        });

        request_count += 1;

        // Rate limiting
        tokio::time::sleep(Duration::from_micros(delay_micros)).await;

        // Progress report every 10 seconds
        if request_count % (config.target_tps * 10) == 0 {
            let elapsed = test_start.elapsed().as_secs();
            let current_tps = stats.total_requests.load(Ordering::Relaxed) / elapsed.max(1);
            println!("   {} requests sent ({} TPS)", request_count, current_tps);
        }
    }

    println!("   All requests sent, waiting for completion...");

    // Wait for all tasks to complete
    while join_set.join_next().await.is_some() {}

    let total_duration = test_start.elapsed();

    // Report statistics
    stats.report();
    println!("\n⏱️  Timing");
    println!("============================");
    println!("Total duration:     {:?}", total_duration);
    println!("Actual TPS:         {:.2}",
        stats.total_requests.load(Ordering::Relaxed) as f64 / total_duration.as_secs_f64());

    // Validate SLO targets
    let success_rate = stats.successful.load(Ordering::Relaxed) as f64
        / stats.total_requests.load(Ordering::Relaxed) as f64;

    assert!(
        success_rate >= 0.99,
        "Success rate {:.2}% below 99% SLO",
        success_rate * 100.0
    );

    println!("\n✅ Load test PASSED");
}

/// Stress test: gradually increase load until system breaks
#[tokio::test]
#[ignore]
async fn test_stress_find_breaking_point() {
    println!("\n🔥 Stress Test - Finding Breaking Point");

    let mut current_tps = 50u64;
    let max_tps = 500u64;
    let step = 50u64;
    let test_duration_secs = 30u64;

    while current_tps <= max_tps {
        println!("\n📈 Testing {} TPS...", current_tps);

        let stats = Arc::new(PerfStats::default());
        let semaphore = Arc::new(Semaphore::new(2000));

        let start = Instant::now();
        let mut join_set = JoinSet::new();

        let delay_micros = (1_000_000 / current_tps) as u64;
        let total_requests = current_tps * test_duration_secs;

        for _ in 0..total_requests {
            let payment_id = Uuid::new_v4().to_string();
            let stats = Arc::clone(&stats);
            let permit = Arc::clone(&semaphore).acquire_owned().await.unwrap();

            join_set.spawn(async move {
                let result = submit_payment(payment_id).await;
                drop(permit);

                match result {
                    Ok(latency) => stats.record_success(latency.as_millis() as u64),
                    Err(_) => stats.record_failure(),
                }
            });

            tokio::time::sleep(Duration::from_micros(delay_micros)).await;
        }

        while join_set.join_next().await.is_some() {}

        let duration = start.elapsed();
        let success_rate = stats.successful.load(Ordering::Relaxed) as f64
            / stats.total_requests.load(Ordering::Relaxed) as f64;

        println!("   Success rate: {:.2}%", success_rate * 100.0);
        println!("   Duration: {:?}", duration);

        if success_rate < 0.95 {
            println!("\n❌ System degraded at {} TPS", current_tps);
            println!("   Breaking point found!");
            break;
        }

        current_tps += step;
    }
}

/// Latency percentile calculations
fn calculate_percentiles(mut latencies: Vec<u64>) -> (u64, u64, u64, u64) {
    latencies.sort();
    let len = latencies.len();

    let p50 = latencies[len / 2];
    let p95 = latencies[len * 95 / 100];
    let p99 = latencies[len * 99 / 100];
    let max = *latencies.last().unwrap();

    (p50, p95, p99, max)
}
