#!/usr/bin/env python3
"""
DelTran MVP - Simple Stress Test
Tests Gateway and all backend services without k6 dependency
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
DURATION_SECONDS = 60  # 1 minute test
CONCURRENT_USERS = 50
REQUESTS_PER_USER = 100

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
        "reference": f"STRESS-TEST-{timestamp}-{thread_id}",
        "idempotency_key": f"stress-{timestamp}-{thread_id}-{random.randint(1000, 9999)}"
    }


def send_request():
    """Send a single transfer request and record metrics"""
    transfer = generate_transfer()

    try:
        start = time.time()
        response = requests.post(
            f"{GATEWAY_URL}/api/v1/transfer",
            json=transfer,
            headers={"Content-Type": "application/json"},
            timeout=10
        )
        duration = (time.time() - start) * 1000  # Convert to ms

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
            results["errors"][f"exception_{type(e).__name__}"] += 1
            results["failed"] += 1


def user_simulation(user_id, requests_count):
    """Simulate a single user making multiple requests"""
    print(f"User {user_id} starting...")

    for i in range(requests_count):
        send_request()
        time.sleep(random.uniform(0.01, 0.1))  # Small random delay

    print(f"User {user_id} completed {requests_count} requests")


def health_check():
    """Check if all services are healthy before starting test"""
    services = {
        "Gateway": f"{GATEWAY_URL}/health",
        "Token Engine": "http://localhost:8081/health",
        "Obligation Engine": "http://localhost:8082/health",
        "Liquidity Router": "http://localhost:8083/health",
        "Risk Engine": "http://localhost:8084/health",
        "Clearing Engine": "http://localhost:8085/health",
        "Compliance Engine": "http://localhost:8086/health",
        "Reporting Engine": "http://localhost:8087/health",
        "Settlement Engine": "http://localhost:8088/health",
        "Notification Engine": "http://localhost:8089/health",
    }

    print("\n🔍 Health Check")
    print("=" * 60)

    all_healthy = True
    for name, url in services.items():
        try:
            resp = requests.get(url, timeout=2)
            if resp.status_code == 200:
                print(f"✅ {name:<20} - Healthy")
            else:
                print(f"⚠️  {name:<20} - Status: {resp.status_code}")
                all_healthy = False
        except Exception as e:
            print(f"❌ {name:<20} - Error: {str(e)}")
            all_healthy = False

    print("=" * 60)
    return all_healthy


def print_results():
    """Print comprehensive test results"""
    total = results["success"] + results["failed"]
    duration = (results["end_time"] - results["start_time"])

    print("\n" + "=" * 60)
    print("📊 STRESS TEST RESULTS")
    print("=" * 60)

    print(f"\n⏱️  Test Duration: {duration:.2f} seconds")
    print(f"👥 Concurrent Users: {CONCURRENT_USERS}")
    print(f"📤 Total Requests: {total}")
    print(f"✅ Successful: {results['success']} ({results['success']/total*100:.1f}%)")
    print(f"❌ Failed: {results['failed']} ({results['failed']/total*100:.1f}%)")
    print(f"🚀 Throughput: {total/duration:.2f} req/s")

    if results["response_times"]:
        response_times = sorted(results["response_times"])
        print(f"\n⏱️  Response Times:")
        print(f"   Min:    {min(response_times):.2f} ms")
        print(f"   Avg:    {mean(response_times):.2f} ms")
        print(f"   Median: {median(response_times):.2f} ms")
        print(f"   P95:    {response_times[int(len(response_times) * 0.95)]:.2f} ms")
        print(f"   P99:    {response_times[int(len(response_times) * 0.99)]:.2f} ms")
        print(f"   Max:    {max(response_times):.2f} ms")

    if results["errors"]:
        print(f"\n⚠️  Error Breakdown:")
        for error_type, count in sorted(results["errors"].items(), key=lambda x: x[1], reverse=True):
            print(f"   {error_type}: {count}")

    # Performance Rating
    print(f"\n📈 Performance Rating:")
    avg_response = mean(results["response_times"]) if results["response_times"] else float('inf')
    success_rate = results["success"] / total * 100

    if avg_response < 500 and success_rate > 95:
        rating = "🌟 EXCELLENT"
    elif avg_response < 1000 and success_rate > 90:
        rating = "✅ GOOD"
    elif avg_response < 2000 and success_rate > 80:
        rating = "⚠️  ACCEPTABLE"
    else:
        rating = "❌ NEEDS IMPROVEMENT"

    print(f"   {rating}")
    print("=" * 60)

    # Save results to file
    timestamp = datetime.now().strftime("%Y%m%d_%H%M%S")
    report_path = f"tests/performance/stress_test_report_{timestamp}.json"

    report_data = {
        "timestamp": datetime.now().isoformat(),
        "config": {
            "duration": duration,
            "concurrent_users": CONCURRENT_USERS,
            "requests_per_user": REQUESTS_PER_USER
        },
        "metrics": {
            "total_requests": total,
            "successful": results["success"],
            "failed": results["failed"],
            "throughput": total/duration,
            "response_times": {
                "min": min(results["response_times"]) if results["response_times"] else 0,
                "avg": mean(results["response_times"]) if results["response_times"] else 0,
                "median": median(results["response_times"]) if results["response_times"] else 0,
                "p95": response_times[int(len(response_times) * 0.95)] if results["response_times"] else 0,
                "max": max(results["response_times"]) if results["response_times"] else 0,
            },
            "errors": dict(results["errors"])
        }
    }

    try:
        with open(report_path, 'w') as f:
            json.dump(report_data, f, indent=2)
        print(f"\n💾 Report saved to: {report_path}")
    except Exception as e:
        print(f"\n⚠️  Could not save report: {e}")


def main():
    """Main test orchestrator"""
    print("\n🚀 DelTran MVP - Stress Test")
    print("=" * 60)

    # Health check
    if not health_check():
        print("\n⚠️  Warning: Some services are not healthy!")
        response = input("Continue anyway? (y/N): ")
        if response.lower() != 'y':
            print("Test aborted.")
            return

    print(f"\n▶️  Starting stress test...")
    print(f"   Concurrent Users: {CONCURRENT_USERS}")
    print(f"   Requests per User: {REQUESTS_PER_USER}")
    print(f"   Total Requests: {CONCURRENT_USERS * REQUESTS_PER_USER}")

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
        time.sleep(0.01)  # Stagger thread starts slightly

    # Wait for all threads to complete
    for thread in threads:
        thread.join()

    results["end_time"] = time.time()

    # Print results
    print_results()


if __name__ == "__main__":
    main()
