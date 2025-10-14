#!/usr/bin/env python3
"""
Multi-Bank Integration Stress Test
Tests DelTran system with 5 banks in different currencies
"""

import asyncio
import aiohttp
import random
import time
import json
from datetime import datetime, timedelta
from typing import List, Dict
import statistics

# Test configuration
GATEWAY_URL = "http://localhost:8080"
NUM_BANKS = 5
CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CHF"]
TEST_DURATION_SECONDS = 300  # 5 minutes
TRANSACTIONS_PER_SECOND = 50

# Bank configurations
BANKS = [
    {
        "id": "BANK001",
        "name": "Global Bank America",
        "swift": "GLBAUS33",
        "currency": "USD",
        "country": "US",
        "balance": 10_000_000.00
    },
    {
        "id": "BANK002",
        "name": "European Finance Group",
        "swift": "EURFDE33",
        "currency": "EUR",
        "country": "DE",
        "balance": 8_000_000.00
    },
    {
        "id": "BANK003",
        "name": "London Sterling Bank",
        "swift": "LSTRGB2L",
        "currency": "GBP",
        "country": "GB",
        "balance": 7_000_000.00
    },
    {
        "id": "BANK004",
        "name": "Tokyo International Bank",
        "swift": "TOINJPJT",
        "currency": "JPY",
        "country": "JP",
        "balance": 1_200_000_000.00
    },
    {
        "id": "BANK005",
        "name": "Swiss Private Banking",
        "swift": "SWPBCHZZ",
        "currency": "CHF",
        "country": "CH",
        "balance": 9_000_000.00
    }
]

# FX rates (against USD)
FX_RATES = {
    "USD": 1.0,
    "EUR": 0.92,
    "GBP": 0.79,
    "JPY": 149.50,
    "CHF": 0.88
}

class StressTestMetrics:
    def __init__(self):
        self.total_requests = 0
        self.successful_requests = 0
        self.failed_requests = 0
        self.response_times = []
        self.errors = []
        self.transactions_by_currency = {c: 0 for c in CURRENCIES}
        self.volume_by_currency = {c: 0.0 for c in CURRENCIES}
        self.start_time = None
        self.end_time = None

    def record_success(self, response_time: float, currency: str, amount: float):
        self.total_requests += 1
        self.successful_requests += 1
        self.response_times.append(response_time)
        self.transactions_by_currency[currency] += 1
        self.volume_by_currency[currency] += amount

    def record_failure(self, error: str):
        self.total_requests += 1
        self.failed_requests += 1
        self.errors.append(error)

    def print_report(self):
        duration = (self.end_time - self.start_time) if self.end_time and self.start_time else 0

        print("\n" + "="*80)
        print("STRESS TEST REPORT - MULTI-BANK INTEGRATION")
        print("="*80)
        print(f"\nTest Duration: {duration:.2f} seconds")
        print(f"Total Requests: {self.total_requests}")
        print(f"Successful: {self.successful_requests} ({self.successful_requests/self.total_requests*100:.2f}%)")
        print(f"Failed: {self.failed_requests} ({self.failed_requests/self.total_requests*100:.2f}%)")

        if self.response_times:
            print(f"\nResponse Times:")
            print(f"  Min: {min(self.response_times)*1000:.2f}ms")
            print(f"  Max: {max(self.response_times)*1000:.2f}ms")
            print(f"  Mean: {statistics.mean(self.response_times)*1000:.2f}ms")
            print(f"  Median: {statistics.median(self.response_times)*1000:.2f}ms")
            print(f"  P95: {statistics.quantiles(self.response_times, n=20)[18]*1000:.2f}ms")
            print(f"  P99: {statistics.quantiles(self.response_times, n=100)[98]*1000:.2f}ms")

        print(f"\nThroughput: {self.successful_requests/duration:.2f} TPS")

        print(f"\nTransactions by Currency:")
        for currency in CURRENCIES:
            count = self.transactions_by_currency[currency]
            volume = self.volume_by_currency[currency]
            print(f"  {currency}: {count} transactions, {volume:,.2f} {currency}")

        if self.errors:
            print(f"\nErrors ({len(self.errors)} total):")
            error_counts = {}
            for error in self.errors[:10]:  # Show first 10
                error_counts[error] = error_counts.get(error, 0) + 1
            for error, count in sorted(error_counts.items(), key=lambda x: x[1], reverse=True):
                print(f"  [{count}x] {error[:100]}")

        print("="*80 + "\n")


metrics = StressTestMetrics()


async def create_payment(session: aiohttp.ClientSession, sender_bank: Dict, receiver_bank: Dict) -> bool:
    """Create a single payment transaction"""

    # Generate realistic amount
    if sender_bank["currency"] == "JPY":
        amount = round(random.uniform(50000, 5000000), 2)
    else:
        amount = round(random.uniform(500, 50000), 2)

    payment_data = {
        "sender_bank_id": sender_bank["id"],
        "receiver_bank_id": receiver_bank["id"],
        "amount": amount,
        "currency": sender_bank["currency"],
        "sender_account": f"ACC{random.randint(100000, 999999)}",
        "receiver_account": f"ACC{random.randint(100000, 999999)}",
        "reference": f"STR-TEST-{int(time.time()*1000)}-{random.randint(1000, 9999)}",
        "sender_name": f"Company {random.choice(['Alpha', 'Beta', 'Gamma', 'Delta', 'Epsilon'])} Ltd",
        "receiver_name": f"Company {random.choice(['Zeta', 'Eta', 'Theta', 'Iota', 'Kappa'])} Inc"
    }

    start = time.time()
    try:
        async with session.post(
            f"{GATEWAY_URL}/api/payments",
            json=payment_data,
            timeout=aiohttp.ClientTimeout(total=10)
        ) as response:
            response_time = time.time() - start

            if response.status == 200 or response.status == 201:
                metrics.record_success(response_time, sender_bank["currency"], amount)
                return True
            else:
                error_text = await response.text()
                metrics.record_failure(f"HTTP {response.status}: {error_text[:100]}")
                return False

    except asyncio.TimeoutError:
        metrics.record_failure("Timeout")
        return False
    except Exception as e:
        metrics.record_failure(f"{type(e).__name__}: {str(e)[:100]}")
        return False


async def register_banks(session: aiohttp.ClientSession):
    """Register all test banks"""
    print("Registering banks...")

    for bank in BANKS:
        bank_data = {
            "bank_id": bank["id"],
            "name": bank["name"],
            "swift_code": bank["swift"],
            "country": bank["country"],
            "primary_currency": bank["currency"]
        }

        try:
            async with session.post(
                f"{GATEWAY_URL}/api/banks/register",
                json=bank_data,
                timeout=aiohttp.ClientTimeout(total=5)
            ) as response:
                if response.status in [200, 201, 409]:  # 409 = already exists
                    print(f"  ✓ {bank['name']} ({bank['id']})")
                else:
                    print(f"  ✗ {bank['name']}: HTTP {response.status}")
        except Exception as e:
            print(f"  ✗ {bank['name']}: {e}")

    print()


async def payment_generator(session: aiohttp.ClientSession, duration: int, rate: int):
    """Generate payments at specified rate"""
    end_time = time.time() + duration
    interval = 1.0 / rate

    while time.time() < end_time:
        # Random sender and receiver (different banks)
        sender = random.choice(BANKS)
        receiver = random.choice([b for b in BANKS if b["id"] != sender["id"]])

        # Create payment without waiting for completion
        asyncio.create_task(create_payment(session, sender, receiver))

        # Wait for next interval
        await asyncio.sleep(interval)


async def batch_status_monitor(session: aiohttp.ClientSession):
    """Monitor batch settlement status"""
    while True:
        try:
            async with session.get(
                f"{GATEWAY_URL}/api/batches",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as response:
                if response.status == 200:
                    data = await response.json()
                    print(f"[Monitor] Active batches: {len(data.get('batches', []))}")
        except Exception:
            pass

        await asyncio.sleep(10)


async def run_stress_test():
    """Main stress test execution"""
    print("\nStarting Multi-Bank Integration Stress Test\n")
    print(f"Configuration:")
    print(f"  Banks: {NUM_BANKS}")
    print(f"  Currencies: {', '.join(CURRENCIES)}")
    print(f"  Target Rate: {TRANSACTIONS_PER_SECOND} TPS")
    print(f"  Duration: {TEST_DURATION_SECONDS} seconds")
    print(f"  Total Expected Transactions: ~{TRANSACTIONS_PER_SECOND * TEST_DURATION_SECONDS}")
    print()

    connector = aiohttp.TCPConnector(limit=200, limit_per_host=100)
    async with aiohttp.ClientSession(connector=connector) as session:
        # Register banks
        await register_banks(session)

        # Start monitoring task
        monitor_task = asyncio.create_task(batch_status_monitor(session))

        # Start stress test
        print(f"Starting payment generation...\n")
        metrics.start_time = time.time()

        # Run payment generator
        await payment_generator(session, TEST_DURATION_SECONDS, TRANSACTIONS_PER_SECOND)

        # Wait a bit for pending requests to complete
        print("\nWaiting for pending requests to complete...")
        await asyncio.sleep(5)

        metrics.end_time = time.time()

        # Cancel monitor
        monitor_task.cancel()

        # Print report
        metrics.print_report()


async def test_component_endpoints(session: aiohttp.ClientSession):
    """Test all component endpoints"""
    print("\n" + "="*80)
    print("COMPONENT ENDPOINT TESTS")
    print("="*80 + "\n")

    endpoints = [
        ("GET", "/health", "Health Check"),
        ("GET", "/api/banks", "List Banks"),
        ("GET", "/api/payments", "List Payments"),
        ("GET", "/api/batches", "List Batches"),
        ("GET", "/api/limits", "Compliance Limits"),
        ("GET", "/api/compliance/status", "Compliance Status"),
        ("GET", "/api/netting/status", "Netting Status"),
        ("GET", "/api/reconciliation/status", "Reconciliation Status"),
    ]

    for method, path, name in endpoints:
        try:
            async with session.request(
                method,
                f"{GATEWAY_URL}{path}",
                timeout=aiohttp.ClientTimeout(total=5)
            ) as response:
                status = "[OK]" if response.status == 200 else "[FAIL]"
                print(f"{status} {name:30} [{method} {path}] - HTTP {response.status}")
        except Exception as e:
            print(f"[FAIL] {name:30} [{method} {path}] - ERROR: {e}")

    print()


async def run_component_tests():
    """Run comprehensive component tests"""
    async with aiohttp.ClientSession() as session:
        await test_component_endpoints(session)


async def main():
    """Main entry point"""
    # First test components
    await run_component_tests()

    # Then run stress test
    await run_stress_test()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
        metrics.end_time = time.time()
        metrics.print_report()
