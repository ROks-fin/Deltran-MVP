"""
Bank Grade Stress Test - Ultra High Performance
Target: 10,000+ TPS over 10 minutes
Simulates real banking workload with multiple scenarios
"""

import asyncio
import aiohttp
import time
import random
import json
from datetime import datetime, timedelta
from collections import defaultdict
import statistics

# Test configuration
GATEWAY_URL = "http://localhost:8080"
DURATION_SECONDS = 600  # 10 minutes
TARGET_TPS = 10000
CONCURRENT_WORKERS = 2000
BATCH_SIZE = 100

# Banking test scenarios
BANKS = [
    {"id": "BANK001", "name": "JPMorgan Chase", "swift": "CHASUS33", "currency": "USD", "weight": 25},
    {"id": "BANK002", "name": "Bank of America", "swift": "BOFAUS3N", "currency": "USD", "weight": 20},
    {"id": "BANK003", "name": "Citibank", "swift": "CITIUS33", "currency": "USD", "weight": 15},
    {"id": "BANK004", "name": "Wells Fargo", "swift": "WFBIUS6S", "currency": "USD", "weight": 12},
    {"id": "BANK005", "name": "HSBC USA", "swift": "MRMDUS33", "currency": "USD", "weight": 10},
    {"id": "BANK006", "name": "Deutsche Bank", "swift": "DEUTDEFF", "currency": "EUR", "weight": 8},
    {"id": "BANK007", "name": "Barclays", "swift": "BARCGB22", "currency": "GBP", "weight": 5},
    {"id": "BANK008", "name": "UBS", "swift": "UBSWCHZH", "currency": "CHF", "weight": 3},
    {"id": "BANK009", "name": "Mizuho Bank", "swift": "MHCBJPJT", "currency": "JPY", "weight": 2},
]

# Transaction amount distribution (realistic banking)
AMOUNT_RANGES = [
    (100, 1000, 40),           # Small retail: $100-1K (40%)
    (1000, 10000, 30),         # Medium retail: $1K-10K (30%)
    (10000, 100000, 20),       # Corporate: $10K-100K (20%)
    (100000, 1000000, 8),      # Large corporate: $100K-1M (8%)
    (1000000, 10000000, 2),    # Wire transfers: $1M-10M (2%)
]

class BankGradeMetrics:
    def __init__(self):
        self.start_time = time.time()
        self.total_requests = 0
        self.successful_requests = 0
        self.failed_requests = 0
        self.response_times = []
        self.tps_samples = []

        # Latency buckets
        self.latency_buckets = {
            "0-10ms": 0,
            "10-25ms": 0,
            "25-50ms": 0,
            "50-100ms": 0,
            "100-250ms": 0,
            "250-500ms": 0,
            "500ms-1s": 0,
            ">1s": 0
        }

        # Per-bank metrics
        self.bank_metrics = defaultdict(lambda: {
            "count": 0,
            "success": 0,
            "failed": 0,
            "volume": 0
        })

        # Per-scenario metrics
        self.scenario_metrics = defaultdict(lambda: {
            "count": 0,
            "success": 0
        })

        # Error tracking
        self.errors = defaultdict(int)

        # Time series data (per second)
        self.time_series = []
        self.lock = asyncio.Lock()

    async def record_request(self, success, response_time_ms, bank_id, amount, scenario, error=None):
        async with self.lock:
            self.total_requests += 1
            if success:
                self.successful_requests += 1
            else:
                self.failed_requests += 1
                if error:
                    self.errors[error] += 1

            self.response_times.append(response_time_ms)

            # Latency bucket
            if response_time_ms < 10:
                self.latency_buckets["0-10ms"] += 1
            elif response_time_ms < 25:
                self.latency_buckets["10-25ms"] += 1
            elif response_time_ms < 50:
                self.latency_buckets["25-50ms"] += 1
            elif response_time_ms < 100:
                self.latency_buckets["50-100ms"] += 1
            elif response_time_ms < 250:
                self.latency_buckets["100-250ms"] += 1
            elif response_time_ms < 500:
                self.latency_buckets["250-500ms"] += 1
            elif response_time_ms < 1000:
                self.latency_buckets["500ms-1s"] += 1
            else:
                self.latency_buckets[">1s"] += 1

            # Bank metrics
            self.bank_metrics[bank_id]["count"] += 1
            if success:
                self.bank_metrics[bank_id]["success"] += 1
                self.bank_metrics[bank_id]["volume"] += amount
            else:
                self.bank_metrics[bank_id]["failed"] += 1

            # Scenario metrics
            self.scenario_metrics[scenario]["count"] += 1
            if success:
                self.scenario_metrics[scenario]["success"] += 1

    def get_current_tps(self):
        elapsed = time.time() - self.start_time
        if elapsed > 0:
            return self.total_requests / elapsed
        return 0

    def get_statistics(self):
        elapsed = time.time() - self.start_time

        if not self.response_times:
            return None

        sorted_times = sorted(self.response_times)

        return {
            "duration_seconds": elapsed,
            "total_requests": self.total_requests,
            "successful": self.successful_requests,
            "failed": self.failed_requests,
            "success_rate": (self.successful_requests / self.total_requests * 100) if self.total_requests > 0 else 0,
            "tps_actual": self.total_requests / elapsed if elapsed > 0 else 0,
            "latency_mean_ms": statistics.mean(self.response_times),
            "latency_median_ms": statistics.median(self.response_times),
            "latency_p95_ms": sorted_times[int(len(sorted_times) * 0.95)],
            "latency_p99_ms": sorted_times[int(len(sorted_times) * 0.99)],
            "latency_min_ms": min(self.response_times),
            "latency_max_ms": max(self.response_times),
            "latency_buckets": self.latency_buckets,
            "bank_metrics": dict(self.bank_metrics),
            "scenario_metrics": dict(self.scenario_metrics),
            "errors": dict(self.errors)
        }


def select_bank_weighted():
    """Select bank based on weight distribution"""
    total_weight = sum(bank["weight"] for bank in BANKS)
    r = random.uniform(0, total_weight)
    cumulative = 0
    for bank in BANKS:
        cumulative += bank["weight"]
        if r <= cumulative:
            return bank
    return BANKS[0]


def generate_amount():
    """Generate realistic transaction amount"""
    r = random.random() * 100
    cumulative = 0
    for min_amt, max_amt, weight in AMOUNT_RANGES:
        cumulative += weight
        if r <= cumulative:
            return round(random.uniform(min_amt, max_amt), 2)
    return random.uniform(100, 1000)


def generate_payment(scenario="standard"):
    """Generate realistic payment data"""
    sender_bank = select_bank_weighted()
    receiver_bank = select_bank_weighted()

    # Ensure different banks
    while receiver_bank["id"] == sender_bank["id"]:
        receiver_bank = select_bank_weighted()

    amount = generate_amount()

    # Apply scenario-specific modifications
    if scenario == "high_value":
        amount = random.uniform(1000000, 10000000)
    elif scenario == "micro":
        amount = random.uniform(1, 100)
    elif scenario == "cross_border":
        # Ensure different currencies
        while receiver_bank["currency"] == sender_bank["currency"]:
            receiver_bank = select_bank_weighted()

    return {
        "sender_bank_id": sender_bank["id"],
        "receiver_bank_id": receiver_bank["id"],
        "amount": amount,
        "currency": sender_bank["currency"],
        "sender_account": f"ACC{random.randint(1000000, 9999999)}",
        "receiver_account": f"ACC{random.randint(1000000, 9999999)}",
        "reference": f"STRESS-{int(time.time()*1000000)}-{random.randint(1000, 9999)}",
        "sender_name": f"Entity-{random.randint(1, 10000)}",
        "receiver_name": f"Beneficiary-{random.randint(1, 10000)}",
    }, sender_bank, amount, scenario


async def send_payment(session, payment):
    """Send single payment request"""
    try:
        start_time = time.time()
        async with session.post(
            f"{GATEWAY_URL}/api/payments",
            json=payment[0],
            timeout=aiohttp.ClientTimeout(total=30)
        ) as response:
            response_time_ms = (time.time() - start_time) * 1000
            success = response.status in [200, 201]
            return success, response_time_ms, payment[1]["id"], payment[2], payment[3], None
    except Exception as e:
        response_time_ms = (time.time() - start_time) * 1000
        return False, response_time_ms, payment[1]["id"], payment[2], payment[3], str(type(e).__name__)


async def worker(worker_id, metrics, stop_event):
    """Worker coroutine - sends continuous requests"""
    connector = aiohttp.TCPConnector(limit=100, ttl_dns_cache=300)
    async with aiohttp.ClientSession(connector=connector) as session:
        scenarios = ["standard", "high_value", "micro", "cross_border"]

        while not stop_event.is_set():
            # Select scenario
            scenario = random.choice(scenarios)

            # Generate and send payment
            payment = generate_payment(scenario)
            success, response_time, bank_id, amount, scenario, error = await send_payment(session, payment)

            # Record metrics
            await metrics.record_request(success, response_time, bank_id, amount, scenario, error)

            # Small delay to control rate
            await asyncio.sleep(0.0001)


async def progress_reporter(metrics, stop_event):
    """Report progress every 10 seconds"""
    last_count = 0

    while not stop_event.is_set():
        await asyncio.sleep(10)

        elapsed = time.time() - metrics.start_time
        current_count = metrics.total_requests
        interval_count = current_count - last_count
        interval_tps = interval_count / 10

        print(f"\n[{elapsed:.0f}s] Progress Report:")
        print(f"  Total Requests: {current_count:,}")
        print(f"  Success Rate: {metrics.successful_requests/current_count*100:.2f}%")
        print(f"  Current TPS: {interval_tps:.1f}")
        print(f"  Avg TPS: {metrics.get_current_tps():.1f}")

        if metrics.response_times:
            recent_times = metrics.response_times[-1000:]
            print(f"  Recent P95 Latency: {sorted(recent_times)[int(len(recent_times)*0.95)]:.1f}ms")

        last_count = current_count


async def run_stress_test():
    """Run bank grade stress test"""
    print("=" * 80)
    print("BANK GRADE STRESS TEST - ULTRA HIGH PERFORMANCE")
    print("=" * 80)
    print(f"Target: {TARGET_TPS:,} TPS")
    print(f"Duration: {DURATION_SECONDS} seconds (10 minutes)")
    print(f"Concurrent Workers: {CONCURRENT_WORKERS:,}")
    print(f"Banks: {len(BANKS)}")
    print(f"Start Time: {datetime.now().strftime('%Y-%m-%d %H:%M:%S')}")
    print("=" * 80)

    metrics = BankGradeMetrics()
    stop_event = asyncio.Event()

    # Create workers
    workers = [
        asyncio.create_task(worker(i, metrics, stop_event))
        for i in range(CONCURRENT_WORKERS)
    ]

    # Create progress reporter
    reporter = asyncio.create_task(progress_reporter(metrics, stop_event))

    # Run for specified duration
    await asyncio.sleep(DURATION_SECONDS)

    # Stop all workers
    stop_event.set()

    # Wait for workers to finish
    await asyncio.gather(*workers, reporter, return_exceptions=True)

    # Generate final report
    print("\n" + "=" * 80)
    print("FINAL RESULTS")
    print("=" * 80)

    stats = metrics.get_statistics()

    print(f"\n[OVERVIEW]")
    print(f"Duration: {stats['duration_seconds']:.1f} seconds")
    print(f"Total Requests: {stats['total_requests']:,}")
    print(f"Successful: {stats['successful']:,}")
    print(f"Failed: {stats['failed']:,}")
    print(f"Success Rate: {stats['success_rate']:.2f}%")
    print(f"Actual TPS: {stats['tps_actual']:.1f}")
    print(f"Target TPS: {TARGET_TPS:,}")
    print(f"Achievement: {stats['tps_actual']/TARGET_TPS*100:.1f}%")

    print(f"\n[LATENCY]")
    print(f"Mean: {stats['latency_mean_ms']:.2f}ms")
    print(f"Median: {stats['latency_median_ms']:.2f}ms")
    print(f"P95: {stats['latency_p95_ms']:.2f}ms")
    print(f"P99: {stats['latency_p99_ms']:.2f}ms")
    print(f"Min: {stats['latency_min_ms']:.2f}ms")
    print(f"Max: {stats['latency_max_ms']:.2f}ms")

    print(f"\n[LATENCY DISTRIBUTION]")
    total = sum(stats['latency_buckets'].values())
    for bucket, count in stats['latency_buckets'].items():
        pct = (count / total * 100) if total > 0 else 0
        bar = "#" * int(pct / 2)
        print(f"  {bucket:12s}: {count:8,} ({pct:5.1f}%) {bar}")

    print(f"\n[BANK PERFORMANCE]")
    print(f"{'Bank ID':<12} {'Requests':>10} {'Success':>8} {'Failed':>8} {'Volume':>15}")
    print("-" * 60)
    for bank_id, data in sorted(stats['bank_metrics'].items()):
        success_rate = (data['success'] / data['count'] * 100) if data['count'] > 0 else 0
        print(f"{bank_id:<12} {data['count']:>10,} {data['success']:>8,} {data['failed']:>8,} ${data['volume']:>14,.2f}")

    print(f"\n[SCENARIO PERFORMANCE]")
    print(f"{'Scenario':<15} {'Requests':>10} {'Success Rate':>12}")
    print("-" * 40)
    for scenario, data in sorted(stats['scenario_metrics'].items()):
        success_rate = (data['success'] / data['count'] * 100) if data['count'] > 0 else 0
        print(f"{scenario:<15} {data['count']:>10,} {success_rate:>11.1f}%")

    if stats['errors']:
        print(f"\n[ERRORS]")
        for error, count in sorted(stats['errors'].items(), key=lambda x: x[1], reverse=True):
            print(f"  {error}: {count:,}")

    # Calculate total volume
    total_volume = sum(data['volume'] for data in stats['bank_metrics'].values())
    print(f"\n[VOLUME]")
    print(f"Total Transaction Volume: ${total_volume:,.2f}")
    print(f"Average Transaction: ${total_volume/stats['total_requests']:,.2f}")

    # Save detailed report
    report_file = f"bank_grade_stress_test_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
    with open(report_file, 'w') as f:
        json.dump(stats, f, indent=2)

    print(f"\n[REPORT SAVED]")
    print(f"Detailed report: {report_file}")
    print("=" * 80)


if __name__ == "__main__":
    asyncio.run(run_stress_test())
