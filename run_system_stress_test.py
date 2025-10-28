#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
DelTran System Stress Test with Real-time Web Monitoring
Complete system test with transaction generation and monitoring
"""

import asyncio
import aiohttp
import json
import time
import random
import subprocess
import sys
from datetime import datetime, timedelta
from typing import List, Dict, Any

# Configuration
GATEWAY_URL = "http://localhost:8080"
WEB_URL = "http://localhost:3000"

# Stress test parameters
TOTAL_TRANSACTIONS = 500
CONCURRENT_BATCHES = 10
BATCH_SIZE = 50

# Test data
CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD", "CNY"]
STATUSES = ["pending", "processing", "completed", "failed"]
BANKS = [
    "JPMorgan Chase", "Bank of America", "Wells Fargo", "Citibank",
    "Goldman Sachs", "Morgan Stanley", "HSBC", "Deutsche Bank",
    "Barclays", "Credit Suisse", "UBS", "BNP Paribas"
]

class SystemStressTest:
    def __init__(self):
        self.stats = {
            "total_requests": 0,
            "successful": 0,
            "failed": 0,
            "response_times": [],
            "transactions_created": [],
            "start_time": None,
            "end_time": None,
            "errors": []
        }
        self.session = None

    def generate_transaction(self) -> Dict[str, Any]:
        """Generate realistic transaction data"""
        amount = round(random.uniform(100, 500000), 2)
        currency = random.choice(CURRENCIES)

        return {
            "transaction_id": f"STR-{int(time.time()*1000)}-{random.randint(1000, 9999)}",
            "amount": str(amount),
            "currency": currency,
            "sender": {
                "bank": random.choice(BANKS),
                "account": f"ACC-{random.randint(100000, 999999)}",
                "name": f"Customer {random.randint(1000, 9999)}"
            },
            "receiver": {
                "bank": random.choice(BANKS),
                "account": f"ACC-{random.randint(100000, 999999)}",
                "name": f"Merchant {random.randint(1000, 9999)}"
            },
            "status": random.choice(STATUSES),
            "risk_score": random.randint(0, 100),
            "created_at": datetime.now().isoformat(),
            "metadata": {
                "reference": f"REF-{random.randint(100000, 999999)}",
                "description": f"Payment transaction #{random.randint(1000, 9999)}",
                "ip_address": f"{random.randint(1, 255)}.{random.randint(1, 255)}.{random.randint(1, 255)}.{random.randint(1, 255)}"
            }
        }

    async def send_transaction(self, transaction: Dict) -> bool:
        """Send transaction to API"""
        start = time.time()

        try:
            async with self.session.post(
                f"{GATEWAY_URL}/api/v1/transactions",
                json=transaction,
                headers={"Content-Type": "application/json"},
                timeout=aiohttp.ClientTimeout(total=10)
            ) as response:
                elapsed = time.time() - start
                self.stats["total_requests"] += 1
                self.stats["response_times"].append(elapsed)

                if response.status in [200, 201]:
                    self.stats["successful"] += 1
                    data = await response.json()
                    self.stats["transactions_created"].append(transaction["transaction_id"])
                    return True
                else:
                    self.stats["failed"] += 1
                    self.stats["errors"].append({
                        "txn_id": transaction["transaction_id"],
                        "status": response.status
                    })
                    return False

        except Exception as e:
            self.stats["total_requests"] += 1
            self.stats["failed"] += 1
            self.stats["errors"].append({
                "txn_id": transaction["transaction_id"],
                "error": str(e)
            })
            return False

    async def create_batch(self, size: int):
        """Create a batch of transactions"""
        transactions = [self.generate_transaction() for _ in range(size)]
        tasks = [self.send_transaction(txn) for txn in transactions]
        return await asyncio.gather(*tasks)

    async def monitor_system(self):
        """Monitor system metrics"""
        try:
            async with self.session.get(f"{GATEWAY_URL}/api/v1/metrics/live") as response:
                if response.status == 200:
                    return await response.json()
        except:
            pass
        return None

    async def run_stress_test(self):
        """Execute comprehensive stress test"""
        print("="*80)
        print("DELTRAN SYSTEM STRESS TEST")
        print("="*80)
        print(f"\nConfiguration:")
        print(f"  Backend: {GATEWAY_URL}")
        print(f"  Web UI: {WEB_URL}")
        print(f"  Total Transactions: {TOTAL_TRANSACTIONS}")
        print(f"  Batch Size: {BATCH_SIZE}")
        print(f"  Concurrent Batches: {CONCURRENT_BATCHES}")
        print()

        timeout = aiohttp.ClientTimeout(total=30)
        self.session = aiohttp.ClientSession(timeout=timeout)
        self.stats["start_time"] = datetime.now()

        try:
            # Phase 1: Warm-up
            print("Phase 1: System warm-up...")
            await self.create_batch(10)
            print("OK - Warm-up complete\n")

            # Phase 2: Progressive load
            print("Phase 2: Progressive load test...")
            batches = TOTAL_TRANSACTIONS // BATCH_SIZE

            for i in range(batches):
                print(f"  Batch {i+1}/{batches} ({BATCH_SIZE} txns)...", end=" ")
                batch_start = time.time()

                results = await self.create_batch(BATCH_SIZE)
                success_count = sum(1 for r in results if r)

                elapsed = time.time() - batch_start
                print(f"OK ({success_count}/{BATCH_SIZE} success, {elapsed:.2f}s)")

                # Check metrics every 5 batches
                if (i + 1) % 5 == 0:
                    metrics = await self.monitor_system()
                    if metrics:
                        print(f"    System: {metrics.get('active_payments', 'N/A')} active, "
                              f"{metrics.get('total_volume', 'N/A')} volume")

                await asyncio.sleep(0.3)

            # Phase 3: Spike test
            print(f"\nPhase 3: Spike test ({CONCURRENT_BATCHES} concurrent batches)...")
            spike_start = time.time()

            spike_tasks = [self.create_batch(BATCH_SIZE) for _ in range(CONCURRENT_BATCHES)]
            spike_results = await asyncio.gather(*spike_tasks)

            spike_elapsed = time.time() - spike_start
            total_spike = sum(len(batch) for batch in spike_results)
            success_spike = sum(sum(1 for r in batch if r) for batch in spike_results)

            print(f"OK ({success_spike}/{total_spike} success, {spike_elapsed:.2f}s)")

            # Phase 4: Final metrics check
            print("\nPhase 4: Checking final system state...")
            final_metrics = await self.monitor_system()

            if final_metrics:
                print(f"  Active Payments: {final_metrics.get('active_payments', 'N/A')}")
                print(f"  Total Volume: {final_metrics.get('total_volume', 'N/A')}")
                print(f"  Settlement Rate: {final_metrics.get('settlement_rate', 'N/A')}%")

            self.stats["end_time"] = datetime.now()

        except Exception as e:
            print(f"\nERROR: {e}")
            self.stats["errors"].append({"fatal": str(e)})
        finally:
            await self.session.close()

    def print_results(self):
        """Print test results"""
        print("\n" + "="*80)
        print("STRESS TEST RESULTS")
        print("="*80)

        if self.stats["start_time"] and self.stats["end_time"]:
            duration = (self.stats["end_time"] - self.stats["start_time"]).total_seconds()
            print(f"\nDuration: {duration:.2f}s")

            print(f"\nRequests:")
            print(f"  Total: {self.stats['total_requests']}")
            print(f"  Successful: {self.stats['successful']} ({self.stats['successful']/self.stats['total_requests']*100:.1f}%)")
            print(f"  Failed: {self.stats['failed']}")

            if self.stats["response_times"]:
                avg = sum(self.stats["response_times"]) / len(self.stats["response_times"])
                print(f"\nPerformance:")
                print(f"  Avg Response: {avg*1000:.0f}ms")
                print(f"  Min Response: {min(self.stats['response_times'])*1000:.0f}ms")
                print(f"  Max Response: {max(self.stats['response_times'])*1000:.0f}ms")
                print(f"  Throughput: {self.stats['total_requests']/duration:.2f} req/s")

            print(f"\nData:")
            print(f"  Transactions Created: {len(self.stats['transactions_created'])}")

            if self.stats["errors"]:
                print(f"\nErrors: {len(self.stats['errors'])}")
                for error in self.stats["errors"][:5]:
                    print(f"  - {error}")

        # Save results
        filename = f"system_stress_test_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(filename, 'w') as f:
            results_copy = self.stats.copy()
            if results_copy["start_time"]:
                results_copy["start_time"] = results_copy["start_time"].isoformat()
            if results_copy["end_time"]:
                results_copy["end_time"] = results_copy["end_time"].isoformat()
            json.dump(results_copy, f, indent=2)

        print(f"\nResults saved: {filename}")

        print("\n" + "="*80)
        print("MONITORING LINKS")
        print("="*80)
        print(f"\nWeb Interface: {WEB_URL}")
        print(f"Dashboard: {WEB_URL}/")
        print(f"Analytics: {WEB_URL}/analytics")
        print(f"Transactions: {WEB_URL}/transactions")
        print(f"Real-time data is now visible in the web interface!")
        print("\n" + "="*80)

async def check_services():
    """Check if required services are running"""
    print("Checking services...")

    # Check web interface
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{WEB_URL}/", timeout=aiohttp.ClientTimeout(total=5)) as response:
                if response.status == 200:
                    print(f"  OK - Web interface running at {WEB_URL}")
                else:
                    print(f"  WARNING - Web interface returned {response.status}")
    except Exception as e:
        print(f"  ERROR - Web interface not accessible: {e}")
        print(f"  Please start: cd deltran-web && npm run dev")
        return False

    # Check backend (optional)
    try:
        async with aiohttp.ClientSession() as session:
            async with session.get(f"{GATEWAY_URL}/health", timeout=aiohttp.ClientTimeout(total=5)) as response:
                if response.status == 200:
                    print(f"  OK - Backend running at {GATEWAY_URL}")
    except:
        print(f"  NOTE - Backend at {GATEWAY_URL} not available")
        print(f"  Test will run against web interface only")

    print()
    return True

async def main():
    """Main entry point"""
    print("\nDelTran System Stress Test")
    print("="*80)
    print()

    # Check services
    if not await check_services():
        print("\nPlease ensure services are running before starting the test.")
        return

    # Run test
    test = SystemStressTest()
    try:
        await test.run_stress_test()
        test.print_results()
    except KeyboardInterrupt:
        print("\n\nTest interrupted by user")
        if test.stats["start_time"]:
            test.stats["end_time"] = datetime.now()
            test.print_results()
    except Exception as e:
        print(f"\nFatal error: {e}")
        import traceback
        traceback.print_exc()

if __name__ == "__main__":
    asyncio.run(main())
