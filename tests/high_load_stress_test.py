#!/usr/bin/env python3
"""
High-Load Stress Test - 200 TPS
Tests DelTran system under heavy load
"""

import asyncio
import aiohttp
import random
import time
import json
from datetime import datetime
from typing import List, Dict
import statistics
from collections import defaultdict

# Test configuration
GATEWAY_URL = "http://localhost:8080"
NUM_BANKS = 5
CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CHF"]
TEST_DURATION_SECONDS = 60  # 1 minute
TARGET_TPS = 200  # Target: 200 transactions per second
CONCURRENT_CONNECTIONS = 500  # High concurrency

# Bank configurations
BANKS = [
    {"id": "BANK001", "name": "Global Bank America", "swift": "GLBAUS33", "currency": "USD"},
    {"id": "BANK002", "name": "European Finance Group", "swift": "EURFDE33", "currency": "EUR"},
    {"id": "BANK003", "name": "London Sterling Bank", "swift": "LSTRGB2L", "currency": "GBP"},
    {"id": "BANK004", "name": "Tokyo International Bank", "swift": "TOINJPJT", "currency": "JPY"},
    {"id": "BANK005", "name": "Swiss Private Banking", "swift": "SWPBCHZZ", "currency": "CHF"},
]

# FX rates (against USD)
FX_RATES = {"USD": 1.0, "EUR": 0.92, "GBP": 0.79, "JPY": 149.50, "CHF": 0.88}


class HighLoadMetrics:
    def __init__(self):
        self.total_requests = 0
        self.successful_requests = 0
        self.failed_requests = 0
        self.response_times = []
        self.errors = defaultdict(int)
        self.transactions_by_currency = {c: 0 for c in CURRENCIES}
        self.volume_by_currency = {c: 0.0 for c in CURRENCIES}
        self.start_time = None
        self.end_time = None
        self.status_codes = defaultdict(int)

        # Performance buckets
        self.latency_buckets = {
            "0-10ms": 0,
            "10-50ms": 0,
            "50-100ms": 0,
            "100-500ms": 0,
            "500ms-1s": 0,
            ">1s": 0
        }

    def record_success(self, response_time: float, currency: str, amount: float, status_code: int):
        self.total_requests += 1
        self.successful_requests += 1
        self.response_times.append(response_time)
        self.transactions_by_currency[currency] += 1
        self.volume_by_currency[currency] += amount
        self.status_codes[status_code] += 1

        # Bucket latency
        ms = response_time * 1000
        if ms < 10:
            self.latency_buckets["0-10ms"] += 1
        elif ms < 50:
            self.latency_buckets["10-50ms"] += 1
        elif ms < 100:
            self.latency_buckets["50-100ms"] += 1
        elif ms < 500:
            self.latency_buckets["100-500ms"] += 1
        elif ms < 1000:
            self.latency_buckets["500ms-1s"] += 1
        else:
            self.latency_buckets[">1s"] += 1

    def record_failure(self, error: str, status_code: int = 0):
        self.total_requests += 1
        self.failed_requests += 1
        self.errors[error[:100]] += 1
        if status_code:
            self.status_codes[status_code] += 1

    def print_report(self):
        duration = (self.end_time - self.start_time) if self.end_time and self.start_time else 0
        actual_tps = self.successful_requests / duration if duration > 0 else 0

        print("\n" + "="*80)
        print("HIGH-LOAD STRESS TEST REPORT - 200 TPS TARGET")
        print("="*80)
        print(f"\nTest Configuration:")
        print(f"  Target TPS:           {TARGET_TPS}")
        print(f"  Duration:             {duration:.2f} seconds")
        print(f"  Concurrent Conns:     {CONCURRENT_CONNECTIONS}")
        print(f"  Expected Requests:    {TARGET_TPS * TEST_DURATION_SECONDS}")

        print(f"\nResults:")
        print(f"  Total Requests:       {self.total_requests}")
        print(f"  Successful:           {self.successful_requests} ({self.successful_requests/self.total_requests*100:.2f}%)")
        print(f"  Failed:               {self.failed_requests} ({self.failed_requests/self.total_requests*100:.2f}%)")
        print(f"  Actual TPS:           {actual_tps:.2f}")
        print(f"  Target Achievement:   {actual_tps/TARGET_TPS*100:.1f}%")

        if self.response_times:
            print(f"\nLatency Statistics:")
            print(f"  Min:        {min(self.response_times)*1000:.2f}ms")
            print(f"  Max:        {max(self.response_times)*1000:.2f}ms")
            print(f"  Mean:       {statistics.mean(self.response_times)*1000:.2f}ms")
            print(f"  Median:     {statistics.median(self.response_times)*1000:.2f}ms")

            if len(self.response_times) >= 20:
                p95 = statistics.quantiles(self.response_times, n=20)[18]
                print(f"  P95:        {p95*1000:.2f}ms")
            if len(self.response_times) >= 100:
                p99 = statistics.quantiles(self.response_times, n=100)[98]
                print(f"  P99:        {p99*1000:.2f}ms")

            print(f"\nLatency Distribution:")
            for bucket, count in self.latency_buckets.items():
                if count > 0:
                    pct = count / len(self.response_times) * 100
                    bar = "#" * int(pct / 2)
                    print(f"  {bucket:12} {count:6} ({pct:5.1f}%) {bar}")

        print(f"\nHTTP Status Codes:")
        for code, count in sorted(self.status_codes.items()):
            print(f"  {code}: {count}")

        print(f"\nTransactions by Currency:")
        total_volume_usd = 0
        for currency in CURRENCIES:
            count = self.transactions_by_currency[currency]
            volume = self.volume_by_currency[currency]
            volume_usd = volume / FX_RATES[currency]
            total_volume_usd += volume_usd
            if count > 0:
                print(f"  {currency}: {count:5} txns, {volume:15,.2f} {currency} (${volume_usd:,.2f})")

        print(f"\n  Total Volume (USD):   ${total_volume_usd:,.2f}")

        if self.errors:
            print(f"\nTop Errors ({len(self.errors)} unique):")
            sorted_errors = sorted(self.errors.items(), key=lambda x: x[1], reverse=True)
            for error, count in sorted_errors[:10]:
                print(f"  [{count:4}x] {error}")

        print("\n" + "="*80)

        # Performance assessment
        print("\nPerformance Assessment:")
        if actual_tps >= TARGET_TPS:
            print(f"  [EXCELLENT] Exceeded target! {actual_tps:.1f} TPS >= {TARGET_TPS} TPS")
        elif actual_tps >= TARGET_TPS * 0.9:
            print(f"  [GOOD] Near target: {actual_tps:.1f} TPS (~{actual_tps/TARGET_TPS*100:.0f}% of target)")
        elif actual_tps >= TARGET_TPS * 0.7:
            print(f"  [ACCEPTABLE] {actual_tps:.1f} TPS (~{actual_tps/TARGET_TPS*100:.0f}% of target)")
        else:
            print(f"  [NEEDS IMPROVEMENT] {actual_tps:.1f} TPS (only {actual_tps/TARGET_TPS*100:.0f}% of target)")

        if self.response_times:
            avg_latency = statistics.mean(self.response_times) * 1000
            if avg_latency < 10:
                print(f"  [EXCELLENT] Ultra-low latency: {avg_latency:.1f}ms")
            elif avg_latency < 50:
                print(f"  [GOOD] Low latency: {avg_latency:.1f}ms")
            elif avg_latency < 100:
                print(f"  [ACCEPTABLE] Moderate latency: {avg_latency:.1f}ms")
            else:
                print(f"  [NEEDS IMPROVEMENT] High latency: {avg_latency:.1f}ms")

        success_rate = self.successful_requests / self.total_requests * 100 if self.total_requests > 0 else 0
        if success_rate >= 99:
            print(f"  [EXCELLENT] Success rate: {success_rate:.2f}%")
        elif success_rate >= 95:
            print(f"  [GOOD] Success rate: {success_rate:.2f}%")
        elif success_rate >= 90:
            print(f"  [ACCEPTABLE] Success rate: {success_rate:.2f}%")
        else:
            print(f"  [NEEDS IMPROVEMENT] Success rate: {success_rate:.2f}%")

        print("="*80 + "\n")


metrics = HighLoadMetrics()


async def create_payment(session: aiohttp.ClientSession, sender_bank: Dict, receiver_bank: Dict):
    """Create a single payment transaction"""

    # Generate realistic amount based on currency
    if sender_bank["currency"] == "JPY":
        amount = round(random.uniform(100000, 10000000), 2)
    else:
        amount = round(random.uniform(1000, 100000), 2)

    payment_data = {
        "sender_bank_id": sender_bank["id"],
        "receiver_bank_id": receiver_bank["id"],
        "amount": amount,
        "currency": sender_bank["currency"],
        "sender_account": f"ACC{random.randint(1000000, 9999999)}",
        "receiver_account": f"ACC{random.randint(1000000, 9999999)}",
        "reference": f"LOAD-{int(time.time()*1000000)}-{random.randint(1000, 9999)}",
        "sender_name": f"Sender-{random.randint(1, 1000)}",
        "receiver_name": f"Receiver-{random.randint(1, 1000)}"
    }

    start = time.time()
    try:
        async with session.post(
            f"{GATEWAY_URL}/api/payments",
            json=payment_data,
            timeout=aiohttp.ClientTimeout(total=10)
        ) as response:
            response_time = time.time() - start
            status_code = response.status

            if response.status in [200, 201]:
                metrics.record_success(response_time, sender_bank["currency"], amount, status_code)
                return True
            else:
                error_text = await response.text()
                metrics.record_failure(f"HTTP {response.status}", status_code)
                return False

    except asyncio.TimeoutError:
        metrics.record_failure("Timeout")
        return False
    except aiohttp.ClientError as e:
        metrics.record_failure(f"ClientError: {str(e)[:50]}")
        return False
    except Exception as e:
        metrics.record_failure(f"{type(e).__name__}: {str(e)[:50]}")
        return False


async def register_banks(session: aiohttp.ClientSession):
    """Register all test banks"""
    print("Registering test banks...")

    for bank in BANKS:
        bank_data = {
            "bank_id": bank["id"],
            "name": bank["name"],
            "swift_code": bank["swift"],
            "country": "XX",
            "primary_currency": bank["currency"]
        }

        try:
            async with session.post(
                f"{GATEWAY_URL}/api/banks/register",
                json=bank_data,
                timeout=aiohttp.ClientTimeout(total=5)
            ) as response:
                if response.status in [200, 201, 409]:
                    print(f"  [OK] {bank['name']}")
                else:
                    print(f"  [WARN] {bank['name']}: HTTP {response.status}")
        except Exception as e:
            print(f"  [ERROR] {bank['name']}: {e}")

    print()


async def payment_worker(session: aiohttp.ClientSession, worker_id: int, end_time: float):
    """Worker that generates payments continuously"""
    count = 0
    while time.time() < end_time:
        sender = random.choice(BANKS)
        receiver = random.choice([b for b in BANKS if b["id"] != sender["id"]])

        await create_payment(session, sender, receiver)
        count += 1

        # Small delay to prevent overwhelming
        await asyncio.sleep(0.001)  # 1ms

    return count


async def run_high_load_test():
    """Main high-load stress test execution"""
    print("\n" + "="*80)
    print("HIGH-LOAD STRESS TEST - 200 TPS TARGET")
    print("="*80)
    print(f"\nConfiguration:")
    print(f"  Banks:                {NUM_BANKS}")
    print(f"  Currencies:           {', '.join(CURRENCIES)}")
    print(f"  Target TPS:           {TARGET_TPS}")
    print(f"  Duration:             {TEST_DURATION_SECONDS} seconds")
    print(f"  Concurrent Workers:   {CONCURRENT_CONNECTIONS}")
    print(f"  Expected Requests:    ~{TARGET_TPS * TEST_DURATION_SECONDS}")
    print("="*80 + "\n")

    # High concurrency connector
    connector = aiohttp.TCPConnector(
        limit=CONCURRENT_CONNECTIONS,
        limit_per_host=CONCURRENT_CONNECTIONS,
        ttl_dns_cache=300
    )

    async with aiohttp.ClientSession(connector=connector) as session:
        # Register banks
        await register_banks(session)

        # Warmup
        print("Warming up (5 seconds)...")
        warmup_end = time.time() + 5
        warmup_tasks = [payment_worker(session, i, warmup_end) for i in range(50)]
        await asyncio.gather(*warmup_tasks, return_exceptions=True)
        print("Warmup complete!\n")

        # Start stress test
        print(f"Starting high-load stress test for {TEST_DURATION_SECONDS} seconds...")
        print(f"Workers: {CONCURRENT_CONNECTIONS}")
        print()

        metrics.start_time = time.time()
        end_time = metrics.start_time + TEST_DURATION_SECONDS

        # Create worker tasks
        tasks = [
            payment_worker(session, i, end_time)
            for i in range(CONCURRENT_CONNECTIONS)
        ]

        # Monitor progress
        async def monitor():
            start = metrics.start_time
            while time.time() < end_time:
                await asyncio.sleep(5)
                elapsed = time.time() - start
                current_tps = metrics.successful_requests / elapsed if elapsed > 0 else 0
                print(f"  [{elapsed:.0f}s] TPS: {current_tps:.1f}, Success: {metrics.successful_requests}, Failed: {metrics.failed_requests}")

        # Run workers and monitor
        monitor_task = asyncio.create_task(monitor())
        await asyncio.gather(*tasks, return_exceptions=True)
        monitor_task.cancel()

        # Wait for pending requests
        print("\nWaiting for pending requests to complete...")
        await asyncio.sleep(3)

        metrics.end_time = time.time()

        # Print report
        metrics.print_report()


async def main():
    """Main entry point"""
    try:
        await run_high_load_test()
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
        metrics.end_time = time.time()
        metrics.print_report()


if __name__ == "__main__":
    asyncio.run(main())
