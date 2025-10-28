"""
DelTran Frontend Stress Test
Tests web interface performance and UI responsiveness
"""

import asyncio
import aiohttp
import time
from datetime import datetime
import json
import sys

# Configuration
WEB_URL = "http://localhost:3000"
CONCURRENT_REQUESTS = 50
TEST_PAGES = [
    "/",
    "/analytics",
    "/transactions",
    "/payments",
    "/compliance",
    "/audit",
    "/banks",
    "/reports",
    "/users",
    "/network",
    "/database",
    "/settings"
]

class FrontendStressTest:
    def __init__(self):
        self.results = {
            "total_requests": 0,
            "successful_requests": 0,
            "failed_requests": 0,
            "page_results": {},
            "response_times": [],
            "errors": [],
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

    async def test_page(self, page: str, repeat: int = 1) -> dict:
        """Test a single page"""
        url = f"{WEB_URL}{page}"
        page_times = []

        for i in range(repeat):
            start_time = time.time()

            try:
                async with self.session.get(url, allow_redirects=True) as response:
                    elapsed = time.time() - start_time
                    content = await response.text()

                    self.results["total_requests"] += 1
                    page_times.append(elapsed)

                    if response.status == 200:
                        self.results["successful_requests"] += 1
                        self.results["response_times"].append(elapsed)
                    else:
                        self.results["failed_requests"] += 1
                        self.results["errors"].append({
                            "page": page,
                            "status": response.status,
                            "attempt": i + 1
                        })

            except Exception as e:
                elapsed = time.time() - start_time
                self.results["total_requests"] += 1
                self.results["failed_requests"] += 1
                page_times.append(elapsed)
                self.results["errors"].append({
                    "page": page,
                    "error": str(e),
                    "attempt": i + 1
                })

        return {
            "page": page,
            "times": page_times,
            "avg_time": sum(page_times) / len(page_times) if page_times else 0,
            "min_time": min(page_times) if page_times else 0,
            "max_time": max(page_times) if page_times else 0
        }

    async def test_concurrent_load(self, page: str, concurrent: int) -> dict:
        """Test concurrent requests to a page"""
        print(f"  Testing {concurrent} concurrent requests to {page}...")

        start_time = time.time()
        tasks = [self.test_page(page) for _ in range(concurrent)]
        results = await asyncio.gather(*tasks)
        total_time = time.time() - start_time

        times = []
        for result in results:
            times.extend(result["times"])

        return {
            "page": page,
            "concurrent": concurrent,
            "total_time": total_time,
            "avg_time": sum(times) / len(times) if times else 0,
            "throughput": len(times) / total_time if total_time > 0 else 0
        }

    async def run_stress_test(self):
        """Main stress test execution"""
        print("=" * 80)
        print("🎨 DELTRAN FRONTEND STRESS TEST")
        print("=" * 80)
        print(f"\nConfiguration:")
        print(f"  - Web URL: {WEB_URL}")
        print(f"  - Pages to test: {len(TEST_PAGES)}")
        print(f"  - Concurrent requests per page: {CONCURRENT_REQUESTS}")
        print()

        await self.create_session()
        self.results["start_time"] = datetime.now()

        try:
            # Phase 1: Individual page tests
            print("📄 Phase 1: Testing individual pages...")
            for page in TEST_PAGES:
                print(f"  Testing {page}...", end=" ")
                result = await self.test_page(page, repeat=5)
                self.results["page_results"][page] = result
                status = "✓" if result["avg_time"] < 2.0 else "⚠"
                print(f"{status} avg: {result['avg_time']*1000:.0f}ms")

            # Phase 2: Concurrent load test
            print(f"\n⚡ Phase 2: Concurrent load test...")
            key_pages = ["/", "/analytics", "/transactions"]
            concurrent_results = []

            for page in key_pages:
                result = await self.test_concurrent_load(page, CONCURRENT_REQUESTS)
                concurrent_results.append(result)
                print(f"  ✓ {page}: {result['throughput']:.2f} req/s")

            # Phase 3: Rapid navigation simulation
            print(f"\n🔄 Phase 3: Rapid navigation simulation...")
            navigation_start = time.time()

            for i in range(10):
                page = TEST_PAGES[i % len(TEST_PAGES)]
                await self.test_page(page)

            navigation_time = time.time() - navigation_start
            print(f"  ✓ Completed 10 page navigations in {navigation_time:.2f}s")

            # Phase 4: UI responsiveness check
            print(f"\n🎯 Phase 4: UI responsiveness check...")
            critical_pages = ["/", "/analytics"]

            for page in critical_pages:
                result = await self.test_page(page)
                response_time = result["times"][0] if result["times"] else 0
                status = "✓" if response_time < 1.0 else ("⚠" if response_time < 2.0 else "✗")
                print(f"  {status} {page}: {response_time*1000:.0f}ms {'(excellent)' if response_time < 1.0 else '(acceptable)' if response_time < 2.0 else '(slow)'}")

            self.results["end_time"] = datetime.now()

        except Exception as e:
            print(f"\n❌ Error during stress test: {e}")
            self.results["errors"].append({"fatal": str(e)})
        finally:
            await self.close_session()

    def print_report(self):
        """Print detailed test report"""
        print("\n" + "=" * 80)
        print("📊 FRONTEND STRESS TEST RESULTS")
        print("=" * 80)

        # Duration
        duration = (self.results["end_time"] - self.results["start_time"]).total_seconds()
        print(f"\n⏱️  Duration: {duration:.2f}s")

        # Requests
        print(f"\n📨 Requests:")
        print(f"  Total: {self.results['total_requests']}")
        success_rate = (self.results['successful_requests'] / self.results['total_requests'] * 100) if self.results['total_requests'] > 0 else 0
        print(f"  Successful: {self.results['successful_requests']} ({success_rate:.1f}%)")
        print(f"  Failed: {self.results['failed_requests']}")

        # Performance
        if self.results["response_times"]:
            avg_time = sum(self.results["response_times"]) / len(self.results["response_times"])
            min_time = min(self.results["response_times"])
            max_time = max(self.results["response_times"])

            print(f"\n⚡ Performance:")
            print(f"  Avg Response Time: {avg_time*1000:.2f}ms")
            print(f"  Min Response Time: {min_time*1000:.2f}ms")
            print(f"  Max Response Time: {max_time*1000:.2f}ms")
            print(f"  Throughput: {self.results['total_requests']/duration:.2f} req/s")

        # Page-by-page results
        print(f"\n📄 Page Performance:")
        sorted_pages = sorted(
            self.results["page_results"].items(),
            key=lambda x: x[1]["avg_time"]
        )

        for page, result in sorted_pages:
            status = "🟢" if result["avg_time"] < 1.0 else "🟡" if result["avg_time"] < 2.0 else "🔴"
            print(f"  {status} {page:20s} - avg: {result['avg_time']*1000:6.0f}ms  (min: {result['min_time']*1000:5.0f}ms, max: {result['max_time']*1000:5.0f}ms)")

        # Errors
        if self.results["errors"]:
            print(f"\n❌ Errors ({len(self.results['errors'])}):")
            for i, error in enumerate(self.results["errors"][:10], 1):
                print(f"  {i}. {error}")
            if len(self.results["errors"]) > 10:
                print(f"  ... and {len(self.results['errors']) - 10} more")

        # Assessment
        print(f"\n🎯 Assessment:")
        if success_rate > 95 and avg_time < 1.0:
            print("  ✅ Excellent - Frontend is highly responsive!")
        elif success_rate > 90 and avg_time < 2.0:
            print("  ✅ Good - Frontend performance is acceptable")
        elif success_rate > 80:
            print("  ⚠️  Fair - Some performance issues detected")
        else:
            print("  ❌ Poor - Significant performance problems")

        # Save results
        self.save_results()

        print("\n" + "=" * 80)
        print("✅ FRONTEND STRESS TEST COMPLETE")
        print("=" * 80)

    def save_results(self):
        """Save results to JSON file"""
        filename = f"frontend_stress_test_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"

        results_copy = self.results.copy()
        results_copy["start_time"] = results_copy["start_time"].isoformat()
        results_copy["end_time"] = results_copy["end_time"].isoformat()

        with open(filename, 'w') as f:
            json.dump(results_copy, f, indent=2)

        print(f"\n💾 Results saved to: {filename}")

async def main():
    """Main entry point"""
    runner = FrontendStressTest()

    try:
        await runner.run_stress_test()
        runner.print_report()
    except KeyboardInterrupt:
        print("\n\n⚠️  Test interrupted by user")
        if runner.results["start_time"]:
            runner.results["end_time"] = datetime.now()
            runner.print_report()
    except Exception as e:
        print(f"\n\n❌ Fatal error: {e}")
        import traceback
        traceback.print_exc()
        sys.exit(1)

if __name__ == "__main__":
    print("\nStarting DelTran Frontend Stress Test...")
    print("Press Ctrl+C to stop\n")
    asyncio.run(main())
