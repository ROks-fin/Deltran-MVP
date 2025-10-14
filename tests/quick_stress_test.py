#!/usr/bin/env python3
"""
Quick Stress Test - Demo version (30 seconds)
"""

import asyncio
import aiohttp
import random
import time
import statistics

GATEWAY_URL = "http://localhost:8080"
TEST_DURATION_SECONDS = 30
TRANSACTIONS_PER_SECOND = 20

BANKS = [
    {"id": "BANK001", "name": "Global Bank America", "currency": "USD"},
    {"id": "BANK002", "name": "European Finance Group", "currency": "EUR"},
    {"id": "BANK003", "name": "London Sterling Bank", "currency": "GBP"},
    {"id": "BANK004", "name": "Tokyo International Bank", "currency": "JPY"},
    {"id": "BANK005", "name": "Swiss Private Banking", "currency": "CHF"},
]

class Metrics:
    def __init__(self):
        self.total = 0
        self.success = 0
        self.failed = 0
        self.response_times = []
        self.start_time = None
        self.end_time = None

    def record_success(self, response_time: float):
        self.total += 1
        self.success += 1
        self.response_times.append(response_time)

    def record_failure(self):
        self.total += 1
        self.failed += 1

    def print_report(self):
        duration = (self.end_time - self.start_time) if self.end_time and self.start_time else 0

        print("\n" + "="*70)
        print("QUICK STRESS TEST REPORT")
        print("="*70)
        print(f"Duration: {duration:.2f} seconds")
        print(f"Total Requests: {self.total}")
        print(f"Success: {self.success} ({self.success/self.total*100:.1f}%)")
        print(f"Failed: {self.failed} ({self.failed/self.total*100:.1f}%)")

        if self.response_times:
            print(f"\nResponse Times:")
            print(f"  Min: {min(self.response_times)*1000:.1f}ms")
            print(f"  Max: {max(self.response_times)*1000:.1f}ms")
            print(f"  Mean: {statistics.mean(self.response_times)*1000:.1f}ms")
            print(f"  Median: {statistics.median(self.response_times)*1000:.1f}ms")

        print(f"\nThroughput: {self.success/duration:.1f} TPS")
        print("="*70 + "\n")


metrics = Metrics()


async def create_payment(session: aiohttp.ClientSession, sender: dict, receiver: dict):
    amount = round(random.uniform(1000, 100000), 2)

    payment_data = {
        "sender_bank_id": sender["id"],
        "receiver_bank_id": receiver["id"],
        "amount": amount,
        "currency": sender["currency"],
        "sender_account": f"ACC{random.randint(100000, 999999)}",
        "receiver_account": f"ACC{random.randint(100000, 999999)}",
        "reference": f"TEST-{int(time.time()*1000)}",
        "sender_name": "Test Sender",
        "receiver_name": "Test Receiver"
    }

    start = time.time()
    try:
        async with session.post(
            f"{GATEWAY_URL}/api/payments",
            json=payment_data,
            timeout=aiohttp.ClientTimeout(total=5)
        ) as response:
            response_time = time.time() - start
            if response.status in [200, 201]:
                metrics.record_success(response_time)
            else:
                metrics.record_failure()
    except Exception:
        metrics.record_failure()


async def payment_generator(session: aiohttp.ClientSession, duration: int, rate: int):
    end_time = time.time() + duration
    interval = 1.0 / rate

    while time.time() < end_time:
        sender = random.choice(BANKS)
        receiver = random.choice([b for b in BANKS if b["id"] != sender["id"]])
        asyncio.create_task(create_payment(session, sender, receiver))
        await asyncio.sleep(interval)


async def register_banks(session: aiohttp.ClientSession):
    print("Registering 5 banks...")
    for bank in BANKS:
        try:
            async with session.post(
                f"{GATEWAY_URL}/api/banks/register",
                json={
                    "bank_id": bank["id"],
                    "name": bank["name"],
                    "swift_code": bank["id"] + "XX",
                    "country": "XX",
                    "primary_currency": bank["currency"]
                },
                timeout=aiohttp.ClientTimeout(total=3)
            ) as response:
                if response.status in [200, 201, 409]:
                    print(f"  [OK] {bank['name']}")
        except Exception as e:
            print(f"  [FAIL] {bank['name']}: {e}")
    print()


async def main():
    print("\n" + "="*70)
    print("MULTI-BANK STRESS TEST - 30 Second Demo")
    print("="*70)
    print(f"Banks: 5 ({', '.join([b['currency'] for b in BANKS])})")
    print(f"Target Rate: {TRANSACTIONS_PER_SECOND} TPS")
    print(f"Duration: {TEST_DURATION_SECONDS} seconds")
    print(f"Expected Transactions: ~{TRANSACTIONS_PER_SECOND * TEST_DURATION_SECONDS}")
    print("="*70 + "\n")

    connector = aiohttp.TCPConnector(limit=100)
    async with aiohttp.ClientSession(connector=connector) as session:
        await register_banks(session)

        print("Starting payment generation...\n")
        metrics.start_time = time.time()
        await payment_generator(session, TEST_DURATION_SECONDS, TRANSACTIONS_PER_SECOND)

        print("Waiting for pending requests...")
        await asyncio.sleep(3)
        metrics.end_time = time.time()

        metrics.print_report()


if __name__ == "__main__":
    try:
        asyncio.run(main())
    except KeyboardInterrupt:
        print("\nTest interrupted")
        metrics.end_time = time.time()
        metrics.print_report()
