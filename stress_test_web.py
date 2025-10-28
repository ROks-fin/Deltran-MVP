"""
DelTran Web Interface Stress Test
Comprehensive testing with full transaction persistence
"""

import asyncio
import aiohttp
import json
import time
import random
from datetime import datetime, timedelta
from typing import List, Dict, Any
import sys

# Configuration
BASE_URL = "http://localhost:8080"
WEB_URL = "http://localhost:3000"
CONCURRENT_REQUESTS = 100
TOTAL_TRANSACTIONS = 1000
BATCH_SIZE = 50

# Test data generators
CURRENCIES = ["USD", "EUR", "GBP", "JPY", "CHF", "CAD", "AUD"]
STATUSES = ["pending", "completed", "failed"]
BANKS = [
    "JPMorgan Chase", "Bank of America", "Wells Fargo", "Citibank",
    "Goldman Sachs", "Morgan Stanley", "HSBC", "Deutsche Bank"
]

class StressTestRunner:
    def __init__(self):
        self.results = {
            "total_requests": 0,
            "successful_requests": 0,
            "failed_requests": 0,
            "avg_response_time": 0,
            "min_response_time": float('inf'),
            "max_response_time": 0,
            "response_times": [],
            "errors": [],
            "created_transactions": [],
            "start_time": None,
            "end_time": None
        }
        self.session = None

    async def create_session(self):
        """Create aiohttp session"""
        timeout = aiohttp.ClientTimeout(total=30)
        self.session = aiohttp.ClientSession(timeout=timeout)

    async def close_session(self):
        """Close aiohttp session"""
        if self.session:
            await self.session.close()

    def generate_transaction_data(self) -> Dict[str, Any]:
        """Generate realistic transaction data"""
        amount = round(random.uniform(100, 100000), 2)
        currency = random.choice(CURRENCIES)

        return {
            "transaction_id": f"TXN-{int(time.time()*1000)}-{random.randint(1000, 9999)}",
            "amount": amount,
            "currency": currency,
            "sender_bank": random.choice(BANKS),
            "receiver_bank": random.choice(BANKS),
            "status": random.choice(STATUSES),
            "risk_score": random.randint(0, 100),
            "created_at": datetime.now().isoformat(),
            "metadata": {
                "customer_id": f"CUST-{random.randint(10000, 99999)}",
                "reference": f"REF-{random.randint(100000, 999999)}",
                "description": f"Payment for order #{random.randint(1000, 9999)}"
            }
        }

    async def create_transaction(self, transaction_data: Dict[str, Any]) -> Dict[str, Any]:
        """Create a single transaction via API"""
        start_time = time.time()

        try:
            async with self.session.post(
                f"{BASE_URL}/api/v1/transactions",
                json=transaction_data,
                headers={"Content-Type": "application/json"}
            ) as response:
                elapsed = time.time() - start_time

                self.results["total_requests"] += 1
                self.results["response_times"].append(elapsed)

                if response.status in [200, 201]:
                    self.results["successful_requests"] += 1
                    result_data = await response.json()
                    self.results["created_transactions"].append(result_data)
                    return {"success": True, "time": elapsed, "data": result_data}
                else:
                    self.results["failed_requests"] += 1
                    error_text = await response.text()
                    self.results["errors"].append({
                        "status": response.status,
                        "error": error_text,
                        "transaction": transaction_data["transaction_id"]
                    })
                    return {"success": False, "time": elapsed, "error": error_text}

        except Exception as e:
            elapsed = time.time() - start_time
            self.results["total_requests"] += 1
            self.results["failed_requests"] += 1
            self.results["response_times"].append(elapsed)
            self.results["errors"].append({
                "error": str(e),
                "transaction": transaction_data["transaction_id"]
            })
            return {"success": False, "time": elapsed, "error": str(e)}

    async def verify_transaction(self, transaction_id: str) -> bool:
        """Verify transaction was saved correctly"""
        try:
            async with self.session.get(f"{BASE_URL}/api/v1/transactions/{transaction_id}") as response:
                return response.status == 200
        except:
            return False

    async def get_all_transactions(self) -> List[Dict]:
        """Fetch all transactions from API"""
        try:
            async with self.session.get(f"{BASE_URL}/api/v1/transactions/recent") as response:
                if response.status == 200:
                    data = await response.json()
                    return data.get("transactions", [])
        except Exception as e:
            print(f"Error fetching transactions: {e}")
        return []

    async def test_web_endpoints(self) -> Dict[str, Any]:
        """Test web interface endpoints"""
        web_results = {
            "homepage": {"success": False, "time": 0},
            "analytics": {"success": False, "time": 0},
            "transactions": {"success": False, "time": 0},
        }

        # Test homepage
        try:
            start = time.time()
            async with self.session.get(f"{WEB_URL}/") as response:
                web_results["homepage"] = {
                    "success": response.status == 200,
                    "time": time.time() - start
                }
        except Exception as e:
            web_results["homepage"]["error"] = str(e)

        # Test analytics
        try:
            start = time.time()
            async with self.session.get(f"{WEB_URL}/analytics") as response:
                web_results["analytics"] = {
                    "success": response.status == 200,
                    "time": time.time() - start
                }
        except Exception as e:
            web_results["analytics"]["error"] = str(e)

        # Test transactions page
        try:
            start = time.time()
            async with self.session.get(f"{WEB_URL}/transactions") as response:
                web_results["transactions"] = {
                    "success": response.status == 200,
                    "time": time.time() - start
                }
        except Exception as e:
            web_results["transactions"]["error"] = str(e)

        return web_results

    async def run_batch(self, batch_size: int) -> List[Dict]:
        """Run a batch of concurrent requests"""
        transactions = [self.generate_transaction_data() for _ in range(batch_size)]
        tasks = [self.create_transaction(txn) for txn in transactions]
        return await asyncio.gather(*tasks)

    async def run_stress_test(self):
        """Main stress test execution"""
        print("=" * 80)
        print("🚀 DELTRAN WEB INTERFACE STRESS TEST")
        print("=" * 80)
        print(f"\nConfiguration:")
        print(f"  - Target: {BASE_URL}")
        print(f"  - Web UI: {WEB_URL}")
        print(f"  - Total Transactions: {TOTAL_TRANSACTIONS}")
        print(f"  - Concurrent Requests: {CONCURRENT_REQUESTS}")
        print(f"  - Batch Size: {BATCH_SIZE}")
        print()

        await self.create_session()
        self.results["start_time"] = datetime.now()

        try:
            # Phase 1: Warm-up
            print("📊 Phase 1: Warming up...")
            await self.run_batch(10)
            print("✓ Warm-up complete\n")

            # Phase 2: Progressive load
            print("📈 Phase 2: Progressive load test...")
            batches = TOTAL_TRANSACTIONS // BATCH_SIZE

            for i in range(batches):
                print(f"  Batch {i+1}/{batches} - Creating {BATCH_SIZE} transactions...", end=" ")
                batch_start = time.time()

                results = await self.run_batch(BATCH_SIZE)

                batch_time = time.time() - batch_start
                success_count = sum(1 for r in results if r["success"])

                print(f"✓ ({success_count}/{BATCH_SIZE} success, {batch_time:.2f}s)")

                # Small delay between batches
                await asyncio.sleep(0.5)

            # Phase 3: Spike test
            print(f"\n⚡ Phase 3: Spike test ({CONCURRENT_REQUESTS} concurrent)...")
            spike_start = time.time()
            spike_results = await self.run_batch(CONCURRENT_REQUESTS)
            spike_time = time.time() - spike_start
            spike_success = sum(1 for r in spike_results if r["success"])
            print(f"✓ Spike complete ({spike_success}/{CONCURRENT_REQUESTS} success, {spike_time:.2f}s)")

            # Phase 4: Web interface test
            print("\n🌐 Phase 4: Testing web interface...")
            web_results = await self.test_web_endpoints()
            for page, result in web_results.items():
                status = "✓" if result["success"] else "✗"
                print(f"  {status} {page}: {result['time']:.3f}s")

            # Phase 5: Data verification
            print("\n🔍 Phase 5: Verifying data persistence...")
            all_transactions = await self.get_all_transactions()
            print(f"✓ Found {len(all_transactions)} transactions in database")

            self.results["end_time"] = datetime.now()

        except Exception as e:
            print(f"\n❌ Error during stress test: {e}")
            self.results["errors"].append({"fatal": str(e)})
        finally:
            await self.close_session()

    def calculate_statistics(self):
        """Calculate test statistics"""
        if self.results["response_times"]:
            self.results["avg_response_time"] = sum(self.results["response_times"]) / len(self.results["response_times"])
            self.results["min_response_time"] = min(self.results["response_times"])
            self.results["max_response_time"] = max(self.results["response_times"])

    def print_report(self):
        """Print detailed test report"""
        self.calculate_statistics()

        print("\n" + "=" * 80)
        print("📊 STRESS TEST RESULTS")
        print("=" * 80)

        # Time
        duration = (self.results["end_time"] - self.results["start_time"]).total_seconds()
        print(f"\n⏱️  Duration: {duration:.2f}s")

        # Requests
        print(f"\n📨 Requests:")
        print(f"  Total: {self.results['total_requests']}")
        print(f"  Successful: {self.results['successful_requests']} ({self.results['successful_requests']/self.results['total_requests']*100:.1f}%)")
        print(f"  Failed: {self.results['failed_requests']} ({self.results['failed_requests']/self.results['total_requests']*100:.1f}%)")

        # Performance
        print(f"\n⚡ Performance:")
        print(f"  Avg Response Time: {self.results['avg_response_time']*1000:.2f}ms")
        print(f"  Min Response Time: {self.results['min_response_time']*1000:.2f}ms")
        print(f"  Max Response Time: {self.results['max_response_time']*1000:.2f}ms")
        print(f"  Throughput: {self.results['total_requests']/duration:.2f} req/s")

        # Data
        print(f"\n💾 Data Persistence:")
        print(f"  Created Transactions: {len(self.results['created_transactions'])}")

        # Errors
        if self.results["errors"]:
            print(f"\n❌ Errors ({len(self.results['errors'])}):")
            for i, error in enumerate(self.results["errors"][:10], 1):
                print(f"  {i}. {error}")
            if len(self.results["errors"]) > 10:
                print(f"  ... and {len(self.results['errors']) - 10} more")

        # Save results
        self.save_results()

        print("\n" + "=" * 80)
        print("✅ STRESS TEST COMPLETE")
        print("=" * 80)

    def save_results(self):
        """Save results to JSON file"""
        filename = f"stress_test_results_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        # Convert datetime objects to strings
        results_copy = self.results.copy()
        results_copy["start_time"] = results_copy["start_time"].isoformat()
        results_copy["end_time"] = results_copy["end_time"].isoformat()

        with open(filename, 'w') as f:
            json.dump(results_copy, f, indent=2)

        print(f"\n💾 Results saved to: {filename}")

async def main():
    """Main entry point"""
    runner = StressTestRunner()

    try:
        await runner.run_stress_test()
        runner.print_report()
    except KeyboardInterrupt:
        print("\n\n⚠️  Test interrupted by user")
        runner.print_report()
    except Exception as e:
        print(f"\n\n❌ Fatal error: {e}")
        sys.exit(1)

if __name__ == "__main__":
    asyncio.run(main())
