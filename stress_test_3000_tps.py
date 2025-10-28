#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
DelTran HIGH LOAD Stress Test - 3000+ TPS
Extreme load testing with real-time monitoring
"""

import asyncio
import aiohttp
import time
import random
from datetime import datetime
import json
from collections import deque

# Configuration for 3000+ TPS
TARGET_TPS = 3500  # Target transactions per second
TEST_DURATION = 60  # seconds
CONCURRENT_WORKERS = 100  # Number of concurrent workers
BATCH_SIZE = 50  # Transactions per batch

WEB_URL = "http://localhost:3000"
GATEWAY_URL = "http://localhost:8080"

# Test data
CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD", "CNY", "SGD", "HKD"]
STATUSES = ["pending", "processing", "completed", "failed"]
BANKS = [
    "JPMorgan", "BofA", "Wells Fargo", "Citi", "Goldman",
    "Morgan Stanley", "HSBC", "Deutsche", "Barclays", "Credit Suisse"
]

class HighLoadStressTest:
    def __init__(self):
        self.stats = {
            "total_sent": 0,
            "total_success": 0,
            "total_failed": 0,
            "response_times": deque(maxlen=10000),  # Keep last 10k
            "tps_history": [],
            "start_time": None,
            "end_time": None,
            "peak_tps": 0,
            "errors": []
        }
        self.running = True
        self.sessions = []

    def generate_transaction(self) -> dict:
        """Generate lightweight transaction"""
        return {
            "transaction_id": f"TPS-{int(time.time()*1000000)}-{random.randint(1000, 9999)}",
            "amount": str(round(random.uniform(100, 100000), 2)),
            "currency": random.choice(CURRENCIES),
            "sender_bank": random.choice(BANKS),
            "receiver_bank": random.choice(BANKS),
            "status": random.choice(STATUSES),
            "risk_score": random.randint(0, 100),
            "timestamp": time.time()
        }

    async def send_transaction(self, session, transaction: dict) -> bool:
        """Send single transaction"""
        start = time.time()

        try:
            async with session.post(
                f"{GATEWAY_URL}/api/v1/transactions",
                json=transaction,
                timeout=aiohttp.ClientTimeout(total=5)
            ) as response:
                elapsed = time.time() - start
                self.stats["response_times"].append(elapsed)
                self.stats["total_sent"] += 1

                if response.status in [200, 201]:
                    self.stats["total_success"] += 1
                    return True
                else:
                    self.stats["total_failed"] += 1
                    return False

        except asyncio.TimeoutError:
            self.stats["total_sent"] += 1
            self.stats["total_failed"] += 1
            self.stats["errors"].append("Timeout")
            return False
        except Exception as e:
            self.stats["total_sent"] += 1
            self.stats["total_failed"] += 1
            return False

    async def worker(self, worker_id: int):
        """Worker that continuously sends transactions"""
        timeout = aiohttp.ClientTimeout(total=5, connect=2)
        connector = aiohttp.TCPConnector(limit=100, limit_per_host=100)

        async with aiohttp.ClientSession(timeout=timeout, connector=connector) as session:
            self.sessions.append(session)

            while self.running:
                # Send batch of transactions
                tasks = []
                for _ in range(BATCH_SIZE):
                    txn = self.generate_transaction()
                    tasks.append(self.send_transaction(session, txn))

                await asyncio.gather(*tasks, return_exceptions=True)

                # Small delay to control rate
                await asyncio.sleep(0.01)

    async def monitor(self):
        """Monitor and display real-time statistics"""
        last_count = 0
        interval = 1.0  # 1 second intervals

        print("\n" + "="*100)
        print(f"{'Time':<8} {'TPS':<10} {'Total':<12} {'Success':<12} {'Failed':<12} {'Avg RT':<10} {'Peak TPS':<10}")
        print("="*100)

        while self.running:
            await asyncio.sleep(interval)

            # Calculate TPS
            current_count = self.stats["total_sent"]
            current_tps = (current_count - last_count) / interval
            last_count = current_count

            # Update peak TPS
            if current_tps > self.stats["peak_tps"]:
                self.stats["peak_tps"] = current_tps

            self.stats["tps_history"].append({
                "time": time.time(),
                "tps": current_tps
            })

            # Calculate average response time
            if self.stats["response_times"]:
                avg_rt = sum(list(self.stats["response_times"])) / len(self.stats["response_times"])
            else:
                avg_rt = 0

            # Display
            elapsed = int(time.time() - self.stats["start_time"]) if self.stats["start_time"] else 0
            print(f"{elapsed:>7}s {current_tps:>9.0f} {current_count:>11,} "
                  f"{self.stats['total_success']:>11,} {self.stats['total_failed']:>11,} "
                  f"{avg_rt*1000:>8.0f}ms {self.stats['peak_tps']:>9.0f}")

    async def run_high_load_test(self):
        """Execute high load test"""
        print("="*100)
        print("DELTRAN HIGH LOAD STRESS TEST - 3000+ TPS TARGET")
        print("="*100)
        print(f"\nConfiguration:")
        print(f"  Target TPS: {TARGET_TPS}")
        print(f"  Test Duration: {TEST_DURATION}s")
        print(f"  Concurrent Workers: {CONCURRENT_WORKERS}")
        print(f"  Batch Size: {BATCH_SIZE}")
        print(f"  Backend: {GATEWAY_URL}")
        print(f"  Web UI: {WEB_URL}")

        self.stats["start_time"] = time.time()

        # Start workers
        print(f"\nStarting {CONCURRENT_WORKERS} workers...")
        worker_tasks = [asyncio.create_task(self.worker(i)) for i in range(CONCURRENT_WORKERS)]

        # Start monitor
        monitor_task = asyncio.create_task(self.monitor())

        # Run for specified duration
        await asyncio.sleep(TEST_DURATION)

        # Stop workers
        print("\n\nStopping workers...")
        self.running = False

        # Wait for workers to finish
        await asyncio.gather(*worker_tasks, return_exceptions=True)
        monitor_task.cancel()

        self.stats["end_time"] = time.time()

    def print_final_results(self):
        """Print comprehensive final results"""
        duration = self.stats["end_time"] - self.stats["start_time"]

        print("\n" + "="*100)
        print("FINAL RESULTS - HIGH LOAD STRESS TEST")
        print("="*100)

        print(f"\nTest Duration: {duration:.2f}s")

        print(f"\nTransactions:")
        print(f"  Total Sent: {self.stats['total_sent']:,}")
        print(f"  Successful: {self.stats['total_success']:,} ({self.stats['total_success']/self.stats['total_sent']*100:.1f}%)")
        print(f"  Failed: {self.stats['total_failed']:,} ({self.stats['total_failed']/self.stats['total_sent']*100:.1f}%)")

        print(f"\nThroughput:")
        avg_tps = self.stats['total_sent'] / duration
        print(f"  Average TPS: {avg_tps:.2f}")
        print(f"  Peak TPS: {self.stats['peak_tps']:.2f}")
        print(f"  Target TPS: {TARGET_TPS}")

        if avg_tps >= TARGET_TPS:
            print(f"  Status: TARGET ACHIEVED! (+{avg_tps - TARGET_TPS:.0f} TPS)")
        else:
            print(f"  Status: Below target ({TARGET_TPS - avg_tps:.0f} TPS short)")

        if self.stats["response_times"]:
            rt_list = list(self.stats["response_times"])
            print(f"\nResponse Times:")
            print(f"  Average: {sum(rt_list)/len(rt_list)*1000:.2f}ms")
            print(f"  Min: {min(rt_list)*1000:.2f}ms")
            print(f"  Max: {max(rt_list)*1000:.2f}ms")

            # Percentiles
            sorted_rt = sorted(rt_list)
            p50 = sorted_rt[len(sorted_rt)//2] if sorted_rt else 0
            p95 = sorted_rt[int(len(sorted_rt)*0.95)] if sorted_rt else 0
            p99 = sorted_rt[int(len(sorted_rt)*0.99)] if sorted_rt else 0

            print(f"  P50: {p50*1000:.2f}ms")
            print(f"  P95: {p95*1000:.2f}ms")
            print(f"  P99: {p99*1000:.2f}ms")

        if self.stats["errors"]:
            error_count = len(self.stats["errors"])
            print(f"\nErrors: {error_count:,}")
            if error_count > 0:
                unique_errors = set(self.stats["errors"][:100])
                print(f"  Unique error types: {len(unique_errors)}")

        # Save results
        self.save_results()

        print("\n" + "="*100)
        print("MONITORING URLS")
        print("="*100)
        print(f"\nWeb Interface: {WEB_URL}")
        print(f"Dashboard: {WEB_URL}/")
        print(f"Analytics: {WEB_URL}/analytics")
        print(f"Transactions: {WEB_URL}/transactions")
        print("\nReal-time data visible at: http://localhost:3000")
        print("="*100)

    def save_results(self):
        """Save detailed results to JSON"""
        filename = f"high_load_test_3000tps_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        results = {
            "test_config": {
                "target_tps": TARGET_TPS,
                "duration": TEST_DURATION,
                "workers": CONCURRENT_WORKERS,
                "batch_size": BATCH_SIZE
            },
            "results": {
                "total_sent": self.stats["total_sent"],
                "total_success": self.stats["total_success"],
                "total_failed": self.stats["total_failed"],
                "duration": self.stats["end_time"] - self.stats["start_time"],
                "avg_tps": self.stats["total_sent"] / (self.stats["end_time"] - self.stats["start_time"]),
                "peak_tps": self.stats["peak_tps"]
            },
            "tps_history": self.stats["tps_history"][-60:]  # Last 60 seconds
        }

        with open(filename, 'w') as f:
            json.dump(results, f, indent=2)

        print(f"\nResults saved: {filename}")

async def main():
    """Main entry point"""
    test = HighLoadStressTest()

    print("\nDelTran 3000+ TPS Stress Test")
    print("Press Ctrl+C to stop early\n")

    try:
        await test.run_high_load_test()
        test.print_final_results()
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
        test.running = False
        if test.stats["start_time"]:
            test.stats["end_time"] = time.time()
            test.print_final_results()
    except Exception as e:
        print(f"\nError: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    print("\n" + "="*100)
    print("STARTING HIGH LOAD TEST")
    print("="*100)
    asyncio.run(main())
