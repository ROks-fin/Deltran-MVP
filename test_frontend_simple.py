# -*- coding: utf-8 -*-
"""
DelTran Frontend Simple Stress Test
Tests web interface performance
"""

import asyncio
import aiohttp
import time
from datetime import datetime
import json

WEB_URL = "http://localhost:3000"
PAGES = ["/", "/analytics", "/transactions", "/payments"]

class SimpleTest:
    def __init__(self):
        self.results = {
            "total": 0,
            "success": 0,
            "failed": 0,
            "times": []
        }

    async def test_page(self, session, url):
        start = time.time()
        try:
            async with session.get(url, timeout=aiohttp.ClientTimeout(total=10)) as response:
                elapsed = time.time() - start
                self.results["total"] += 1
                self.results["times"].append(elapsed)
                if response.status == 200:
                    self.results["success"] += 1
                    return True, elapsed
                else:
                    self.results["failed"] += 1
                    return False, elapsed
        except Exception as e:
            elapsed = time.time() - start
            self.results["total"] += 1
            self.results["failed"] += 1
            self.results["times"].append(elapsed)
            return False, elapsed

    async def run(self):
        print("="*80)
        print("DELTRAN FRONTEND STRESS TEST")
        print("="*80)
        print(f"Target: {WEB_URL}")
        print(f"Testing {len(PAGES)} pages")
        print()

        async with aiohttp.ClientSession() as session:
            # Test each page 10 times
            for page in PAGES:
                print(f"Testing {page}...", end=" ")
                times = []
                for i in range(10):
                    success, elapsed = await self.test_page(session, f"{WEB_URL}{page}")
                    times.append(elapsed)

                avg = sum(times) / len(times)
                print(f"OK - avg: {avg*1000:.0f}ms")

            # Concurrent test
            print(f"\nConcurrent test (50 requests)...")
            tasks = []
            for _ in range(50):
                page = PAGES[_ % len(PAGES)]
                tasks.append(self.test_page(session, f"{WEB_URL}{page}"))

            start = time.time()
            await asyncio.gather(*tasks)
            duration = time.time() - start
            print(f"Completed in {duration:.2f}s")

        # Results
        print("\n" + "="*80)
        print("RESULTS")
        print("="*80)
        print(f"Total requests: {self.results['total']}")
        print(f"Successful: {self.results['success']}")
        print(f"Failed: {self.results['failed']}")

        if self.results["times"]:
            avg = sum(self.results["times"]) / len(self.results["times"])
            print(f"Avg response time: {avg*1000:.0f}ms")
            print(f"Min response time: {min(self.results['times'])*1000:.0f}ms")
            print(f"Max response time: {max(self.results['times'])*1000:.0f}ms")

        success_rate = (self.results['success'] / self.results['total'] * 100) if self.results['total'] > 0 else 0
        print(f"Success rate: {success_rate:.1f}%")

        if success_rate > 95:
            print("\nStatus: EXCELLENT")
        elif success_rate > 80:
            print("\nStatus: GOOD")
        else:
            print("\nStatus: NEEDS IMPROVEMENT")

        print("="*80)

async def main():
    test = SimpleTest()
    try:
        await test.run()
    except KeyboardInterrupt:
        print("\nTest interrupted")
    except Exception as e:
        print(f"Error: {e}")

if __name__ == "__main__":
    print("\nStarting test...\n")
    asyncio.run(main())
