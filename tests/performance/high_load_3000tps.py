#!/usr/bin/env python3
# -*- coding: utf-8 -*-
"""
DelTran MVP - High Load Stress Test
Target: 3000 TPS (Transactions Per Second)
"""

import requests
import time
import json
import threading
import random
from datetime import datetime
from collections import defaultdict
from statistics import mean, median

# Configuration for 3000 TPS
GATEWAY_URL = "http://localhost:8080"
TARGET_TPS = 3000
TEST_DURATION_SECONDS = 60  # 1 minute test
CONCURRENT_USERS = 150  # Increased to achieve 3000 TPS

# Calculate requests per user to achieve target TPS
REQUESTS_PER_USER = int((TARGET_TPS * TEST_DURATION_SECONDS) / CONCURRENT_USERS)

# Metrics
results = {
    "success": 0,
    "failed": 0,
    "response_times": [],
    "errors": defaultdict(int),
    "start_time": None,
    "end_time": None,
    "status_codes": defaultdict(int)
}
results_lock = threading.Lock()


def generate_transfer():
    """Generate random transfer data"""
    timestamp = int(time.time() * 1000000)
    thread_id = threading.get_ident()

    return {
        "sender_bank": f"BANK{random.randint(1, 5):03d}",
        "receiver_bank": f"BANK{random.randint(1, 20):03d}",
        "amount": random.randint(100, 10000),
        "from_currency": "USD",
        "to_currency": "USD",
        "reference": f"HLT-{timestamp}-{thread_id}",
        "idempotency_key": f"hlt-{timestamp}-{thread_id}-{random.randint(10000, 99999)}"
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
            timeout=30
        )
        duration = (time.time() - start) * 1000

        with results_lock:
            results["response_times"].append(duration)
            results["status_codes"][response.status_code] += 1

            if response.status_code in [200, 201]:
                results["success"] += 1
            else:
                results["failed"] += 1
                if response.status_code == 429:
                    results["errors"]["rate_limited"] += 1
                elif response.status_code == 503:
                    results["errors"]["service_unavailable"] += 1
                elif response.status_code == 401:
                    results["errors"]["unauthorized"] += 1
                else:
                    results["errors"][f"http_{response.status_code}"] += 1

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
            results["errors"]["exception"] += 1
            results["failed"] += 1


def user_simulation(user_id, requests_count):
    """Simulate aggressive user load"""
    print(f"[User {user_id:3d}] Starting - {requests_count} requests")

    for i in range(requests_count):
        send_request()
        # Minimal sleep for maximum throughput
        time.sleep(random.uniform(0.001, 0.005))

    print(f"[User {user_id:3d}] Completed")


def print_progress():
    """Print progress during test"""
    while True:
        time.sleep(5)
        with results_lock:
            total = results["success"] + results["failed"]
            if total == 0:
                continue
            elapsed = time.time() - results["start_time"]
            current_tps = total / elapsed if elapsed > 0 else 0
            print(f"[Progress] Elapsed: {elapsed:.1f}s | Requests: {total} | TPS: {current_tps:.1f} | Success: {results['success']} | Failed: {results['failed']}")

        # Stop progress thread when test completes
        if results["end_time"] is not None:
            break


def print_results():
    """Print comprehensive test results"""
    total = results["success"] + results["failed"]
    if total == 0:
        print("No requests completed")
        return

    duration = (results["end_time"] - results["start_time"])
    actual_tps = total / duration

    print("\n" + "=" * 70)
    print("HIGH LOAD STRESS TEST RESULTS - 3000 TPS TARGET")
    print("=" * 70)

    print(f"\n[TEST CONFIGURATION]")
    print(f"  Target TPS:        {TARGET_TPS}")
    print(f"  Concurrent Users:  {CONCURRENT_USERS}")
    print(f"  Test Duration:     {TEST_DURATION_SECONDS}s (planned)")
    print(f"  Actual Duration:   {duration:.2f}s")

    print(f"\n[THROUGHPUT]")
    print(f"  Total Requests:    {total}")
    print(f"  Successful:        {results['success']} ({results['success']/total*100:.1f}%)")
    print(f"  Failed:            {results['failed']} ({results['failed']/total*100:.1f}%)")
    print(f"  ACTUAL TPS:        {actual_tps:.2f} req/s")
    print(f"  Target Achievement: {actual_tps/TARGET_TPS*100:.1f}%")

    if results["response_times"]:
        response_times = sorted(results["response_times"])
        print(f"\n[LATENCY]")
        print(f"  Min:        {min(response_times):.2f} ms")
        print(f"  Average:    {mean(response_times):.2f} ms")
        print(f"  Median:     {median(response_times):.2f} ms")
        print(f"  P75:        {response_times[int(len(response_times) * 0.75)]:.2f} ms")
        print(f"  P90:        {response_times[int(len(response_times) * 0.90)]:.2f} ms")
        print(f"  P95:        {response_times[int(len(response_times) * 0.95)]:.2f} ms")
        print(f"  P99:        {response_times[int(len(response_times) * 0.99)]:.2f} ms")
        print(f"  P99.9:      {response_times[int(len(response_times) * 0.999)]:.2f} ms")
        print(f"  Max:        {max(response_times):.2f} ms")

    if results["status_codes"]:
        print(f"\n[HTTP STATUS CODES]")
        for status, count in sorted(results["status_codes"].items()):
            print(f"  {status}: {count} ({count/total*100:.1f}%)")

    if results["errors"]:
        print(f"\n[ERROR BREAKDOWN]")
        for error_type, count in sorted(results["errors"].items(), key=lambda x: x[1], reverse=True):
            print(f"  {error_type}: {count}")

    # Performance Rating for High Load
    print(f"\n[PERFORMANCE RATING]")
    success_rate = results["success"] / total * 100 if total > 0 else 0
    avg_latency = mean(results["response_times"]) if results["response_times"] else float('inf')

    print(f"  Success Rate:     {success_rate:.1f}%")
    print(f"  Avg Latency:      {avg_latency:.2f} ms")
    print(f"  Achieved TPS:     {actual_tps:.1f}")

    if actual_tps >= TARGET_TPS * 0.9 and success_rate > 95:
        rating = "[EXCELLENT] - Exceeded performance targets!"
    elif actual_tps >= TARGET_TPS * 0.7 and success_rate > 85:
        rating = "[GOOD] - Met most performance requirements"
    elif actual_tps >= TARGET_TPS * 0.5 and success_rate > 70:
        rating = "[ACCEPTABLE] - Moderate performance under load"
    else:
        rating = "[NEEDS OPTIMIZATION] - System struggled under load"

    print(f"  Overall:          {rating}")
    print("=" * 70)

    # Save detailed report
    save_report(total, duration, actual_tps)


def save_report(total, duration, actual_tps):
    """Save detailed JSON report"""
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_path = f"tests/performance/high_load_report_{timestamp}.json"

    report_data = {
        "test_type": "high_load_stress_test",
        "target_tps": TARGET_TPS,
        "timestamp": datetime.now().isoformat(),
        "config": {
            "concurrent_users": CONCURRENT_USERS,
            "planned_duration": TEST_DURATION_SECONDS,
            "actual_duration": duration,
            "requests_per_user": REQUESTS_PER_USER
        },
        "results": {
            "total_requests": total,
            "successful": results["success"],
            "failed": results["failed"],
            "success_rate": results["success"] / total * 100 if total > 0 else 0,
            "actual_tps": actual_tps,
            "target_achievement_pct": actual_tps / TARGET_TPS * 100
        },
        "latency": {
            "min": min(results["response_times"]) if results["response_times"] else 0,
            "avg": mean(results["response_times"]) if results["response_times"] else 0,
            "median": median(results["response_times"]) if results["response_times"] else 0,
            "p95": sorted(results["response_times"])[int(len(results["response_times"]) * 0.95)] if results["response_times"] else 0,
            "p99": sorted(results["response_times"])[int(len(results["response_times"]) * 0.99)] if results["response_times"] else 0,
            "max": max(results["response_times"]) if results["response_times"] else 0
        },
        "status_codes": dict(results["status_codes"]),
        "errors": dict(results["errors"])
    }

    try:
        with open(report_path, 'w') as f:
            json.dump(report_data, f, indent=2)
        print(f"\n[REPORT] Saved to: {report_path}")
    except Exception as e:
        print(f"\n[WARNING] Could not save report: {e}")


def main():
    """Main test orchestrator"""
    print("\n" + "=" * 70)
    print("DelTran MVP - HIGH LOAD STRESS TEST")
    print("=" * 70)
    print(f"\nTarget: {TARGET_TPS} TPS (Transactions Per Second)")
    print(f"Strategy: {CONCURRENT_USERS} concurrent users")
    print(f"Duration: ~{TEST_DURATION_SECONDS} seconds")
    print(f"Total Requests: ~{CONCURRENT_USERS * REQUESTS_PER_USER}")

    # Quick health check
    print("\n[PREFLIGHT] Checking Gateway...")
    try:
        resp = requests.get(f"{GATEWAY_URL}/health", timeout=5)
        health = resp.json()
        print(f"[PREFLIGHT] Gateway Status: {resp.status_code}")
        print(f"[PREFLIGHT] Service: {health.get('service', 'unknown')}")
        print(f"[PREFLIGHT] Uptime: {health.get('uptime', 'unknown')}")
    except Exception as e:
        print(f"[ERROR] Gateway not available: {e}")
        print("[ABORT] Cannot proceed without Gateway")
        return

    print("\n[START] Launching high load stress test...")
    print(f"[START] T-minus 3 seconds...")
    time.sleep(3)

    # Start progress monitor
    progress_thread = threading.Thread(target=print_progress, daemon=True)
    progress_thread.start()

    # Start test
    results["start_time"] = time.time()

    threads = []
    print(f"\n[RAMPUP] Starting {CONCURRENT_USERS} concurrent users...")

    for i in range(CONCURRENT_USERS):
        thread = threading.Thread(
            target=user_simulation,
            args=(i + 1, REQUESTS_PER_USER),
            daemon=False
        )
        threads.append(thread)
        thread.start()

        # Rapid stagger to spread load
        if i % 10 == 0:
            time.sleep(0.05)

    print(f"[RUNNING] All {CONCURRENT_USERS} users active - test in progress...")

    # Wait for all threads
    for thread in threads:
        thread.join()

    results["end_time"] = time.time()

    print("\n[COMPLETE] All requests finished")

    # Print results
    print_results()


if __name__ == "__main__":
    main()
