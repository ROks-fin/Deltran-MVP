#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
DelTran MVP - Quick Stress Test (No user interaction)
"""

import requests
import time
import json
import threading
import random
from datetime import datetime
from collections import defaultdict
from statistics import mean, median

# Configuration
GATEWAY_URL = "http://localhost:8080"
CONCURRENT_USERS = 20  # Reduced for initial test
REQUESTS_PER_USER = 50

# Metrics
results = {
    "success": 0,
    "failed": 0,
    "response_times": [],
    "errors": defaultdict(int),
    "start_time": None,
    "end_time": None
}
results_lock = threading.Lock()


def generate_transfer():
    """Generate random transfer data"""
    timestamp = int(time.time() * 1000)
    thread_id = threading.get_ident()

    return {
        "sender_bank": "BANK001",
        "receiver_bank": f"BANK{random.randint(1, 20):03d}",
        "amount": random.randint(50, 5000),
        "from_currency": "USD",
        "to_currency": "USD",
        "reference": f"STRESS-{timestamp}-{thread_id}",
        "idempotency_key": f"stress-{timestamp}-{thread_id}-{random.randint(1000, 9999)}"
    }


def send_request():
    """Send a single transfer request"""
    transfer = generate_transfer()

    try:
        start = time.time()
        response = requests.post(
            f"{GATEWAY_URL}/api/v1/transfer",
            json=transfer,
            headers={"Content-Type": "application/json"},
            timeout=10
        )
        duration = (time.time() - start) * 1000

        with results_lock:
            results["response_times"].append(duration)

            if response.status_code in [200, 201]:
                results["success"] += 1
            elif response.status_code == 429:
                results["errors"]["rate_limited"] += 1
                results["failed"] += 1
            else:
                results["errors"][f"status_{response.status_code}"] += 1
                results["failed"] += 1

    except requests.exceptions.Timeout:
        with results_lock:
            results["errors"]["timeout"] += 1
            results["failed"] += 1
    except requests.exceptions.ConnectionError:
        with results_lock:
            results["errors"]["connection_error"] += 1
            results["failed"] += 1
    except Exception as e:
        with results_lock:
            results["errors"][f"exception"] += 1
            results["failed"] += 1


def user_simulation(user_id, requests_count):
    """Simulate user making requests"""
    for i in range(requests_count):
        send_request()
        time.sleep(random.uniform(0.01, 0.05))


def print_results():
    """Print test results"""
    total = results["success"] + results["failed"]
    duration = (results["end_time"] - results["start_time"])

    print("\n" + "=" * 60)
    print("STRESS TEST RESULTS")
    print("=" * 60)

    print(f"\nTest Duration: {duration:.2f} seconds")
    print(f"Concurrent Users: {CONCURRENT_USERS}")
    print(f"Total Requests: {total}")
    print(f"Successful: {results['success']} ({results['success']/total*100:.1f}%)")
    print(f"Failed: {results['failed']} ({results['failed']/total*100:.1f}%)")
    print(f"Throughput: {total/duration:.2f} req/s")

    if results["response_times"]:
        response_times = sorted(results["response_times"])
        print(f"\nResponse Times:")
        print(f"  Min:    {min(response_times):.2f} ms")
        print(f"  Avg:    {mean(response_times):.2f} ms")
        print(f"  Median: {median(response_times):.2f} ms")
        print(f"  P95:    {response_times[int(len(response_times) * 0.95)]:.2f} ms")
        print(f"  P99:    {response_times[int(len(response_times) * 0.99)]:.2f} ms")
        print(f"  Max:    {max(response_times):.2f} ms")

    if results["errors"]:
        print(f"\nError Breakdown:")
        for error_type, count in sorted(results["errors"].items(), key=lambda x: x[1], reverse=True):
            print(f"  {error_type}: {count}")

    # Performance Rating
    print(f"\nPerformance Rating:")
    avg_response = mean(results["response_times"]) if results["response_times"] else float('inf')
    success_rate = results["success"] / total * 100 if total > 0 else 0

    if avg_response < 500 and success_rate > 95:
        rating = "[EXCELLENT]"
    elif avg_response < 1000 and success_rate > 90:
        rating = "[GOOD]"
    elif avg_response < 2000 and success_rate > 80:
        rating = "[ACCEPTABLE]"
    else:
        rating = "[NEEDS IMPROVEMENT]"

    print(f"  {rating}")
    print("=" * 60)


def main():
    """Main test orchestrator"""
    print("\nDelTran MVP - Quick Stress Test")
    print("=" * 60)

    # Quick health check
    print("\nChecking Gateway...")
    try:
        resp = requests.get(f"{GATEWAY_URL}/health", timeout=2)
        print(f"Gateway Status: {resp.status_code}")
    except Exception as e:
        print(f"Gateway Error: {e}")
        print("Aborting test - Gateway not available")
        return

    print(f"\nStarting stress test...")
    print(f"  Concurrent Users: {CONCURRENT_USERS}")
    print(f"  Requests per User: {REQUESTS_PER_USER}")
    print(f"  Total Requests: {CONCURRENT_USERS * REQUESTS_PER_USER}")

    # Start test
    results["start_time"] = time.time()

    threads = []
    for i in range(CONCURRENT_USERS):
        thread = threading.Thread(
            target=user_simulation,
            args=(i + 1, REQUESTS_PER_USER)
        )
        threads.append(thread)
        thread.start()
        time.sleep(0.01)

    # Wait for completion
    for thread in threads:
        thread.join()

    results["end_time"] = time.time()

    # Print results
    print_results()


if __name__ == "__main__":
    main()
