#!/usr/bin/env python3
"""
Load Testing Script for DelTran Settlement Rail

Generates realistic payment traffic and measures:
- Throughput (TPS)
- Latency (p50, p95, p99)
- Error rate
- Resource utilization

Usage:
    python load_test.py --target http://localhost:8080 --duration 60 --rps 1000
"""

import argparse
import asyncio
import json
import random
import statistics
import time
from dataclasses import dataclass
from datetime import datetime
from typing import List, Optional
from uuid import uuid4

import aiohttp
import psutil


@dataclass
class PaymentRequest:
    """Payment request"""

    payment_id: str
    amount: float
    currency: str
    debtor_bic: str
    creditor_bic: str
    debtor_account: str
    creditor_account: str
    reference: str
    timestamp: str


@dataclass
class TestResult:
    """Test result metrics"""

    total_requests: int
    successful_requests: int
    failed_requests: int
    duration_seconds: float
    throughput_tps: float
    latencies_ms: List[float]
    p50_ms: float
    p95_ms: float
    p99_ms: float
    min_ms: float
    max_ms: float
    avg_ms: float
    error_rate: float
    cpu_usage_percent: float
    memory_usage_mb: float


# Sample BIC codes (real banks)
BIC_CODES = [
    "DEUTDEFF",  # Deutsche Bank
    "CHASUS33",  # JP Morgan Chase
    "HSBCGB2L",  # HSBC
    "BNPAFRPP",  # BNP Paribas
    "CRESCHZZ",  # Credit Suisse
    "CITIUS33",  # Citibank
    "BOFAUS3N",  # Bank of America
    "WFBIUS6S",  # Wells Fargo
    "COBADEFF",  # Commerzbank
    "RBOSGGSX",  # RBS
]

# Sample currencies
CURRENCIES = ["USD", "EUR", "GBP", "CHF", "JPY"]


def generate_payment() -> PaymentRequest:
    """Generate random payment request"""
    amount = round(random.uniform(100.0, 1000000.0), 2)
    currency = random.choice(CURRENCIES)
    debtor_bic = random.choice(BIC_CODES)
    creditor_bic = random.choice([b for b in BIC_CODES if b != debtor_bic])

    return PaymentRequest(
        payment_id=str(uuid4()),
        amount=amount,
        currency=currency,
        debtor_bic=debtor_bic,
        creditor_bic=creditor_bic,
        debtor_account=f"ACC{random.randint(100000000, 999999999)}",
        creditor_account=f"ACC{random.randint(100000000, 999999999)}",
        reference=f"REF-{random.randint(1000, 9999)}",
        timestamp=datetime.utcnow().isoformat() + "Z",
    )


async def submit_payment(
    session: aiohttp.ClientSession, target_url: str, payment: PaymentRequest
) -> tuple[bool, float]:
    """
    Submit payment and measure latency

    Returns:
        (success, latency_ms)
    """
    start = time.perf_counter()

    try:
        async with session.post(
            f"{target_url}/api/v1/payments",
            json={
                "payment_id": payment.payment_id,
                "amount": payment.amount,
                "currency": payment.currency,
                "debtor_bic": payment.debtor_bic,
                "creditor_bic": payment.creditor_bic,
                "debtor_account": payment.debtor_account,
                "creditor_account": payment.creditor_account,
                "reference": payment.reference,
                "timestamp": payment.timestamp,
            },
            timeout=aiohttp.ClientTimeout(total=30),
        ) as response:
            success = response.status == 200 or response.status == 201
            latency = (time.perf_counter() - start) * 1000  # Convert to ms
            return success, latency

    except Exception as e:
        latency = (time.perf_counter() - start) * 1000
        print(f"Error submitting payment: {e}")
        return False, latency


async def run_load_test(
    target_url: str, duration_seconds: int, target_rps: int, num_workers: int = 100
) -> TestResult:
    """
    Run load test

    Args:
        target_url: Target service URL
        duration_seconds: Test duration in seconds
        target_rps: Target requests per second
        num_workers: Number of concurrent workers

    Returns:
        TestResult with metrics
    """
    print(f"Starting load test:")
    print(f"  Target: {target_url}")
    print(f"  Duration: {duration_seconds}s")
    print(f"  Target RPS: {target_rps}")
    print(f"  Workers: {num_workers}")
    print()

    # Track metrics
    successful_requests = 0
    failed_requests = 0
    latencies = []

    # Start time
    start_time = time.time()
    end_time = start_time + duration_seconds

    # Create HTTP session
    connector = aiohttp.TCPConnector(limit=num_workers, limit_per_host=num_workers)
    async with aiohttp.ClientSession(connector=connector) as session:

        # Worker function
        async def worker():
            nonlocal successful_requests, failed_requests

            while time.time() < end_time:
                payment = generate_payment()
                success, latency = await submit_payment(session, target_url, payment)

                if success:
                    successful_requests += 1
                else:
                    failed_requests += 1

                latencies.append(latency)

                # Rate limiting (sleep to achieve target RPS)
                await asyncio.sleep(num_workers / target_rps)

        # Start workers
        tasks = [asyncio.create_task(worker()) for _ in range(num_workers)]

        # Progress reporting
        last_report = start_time
        while time.time() < end_time:
            await asyncio.sleep(1)

            # Report every 5 seconds
            if time.time() - last_report >= 5:
                elapsed = time.time() - start_time
                total = successful_requests + failed_requests
                current_tps = total / elapsed if elapsed > 0 else 0

                print(
                    f"  [{int(elapsed)}s] "
                    f"Requests: {total}, "
                    f"Success: {successful_requests}, "
                    f"Failed: {failed_requests}, "
                    f"TPS: {current_tps:.1f}"
                )

                last_report = time.time()

        # Wait for workers to complete
        await asyncio.gather(*tasks, return_exceptions=True)

    # Calculate metrics
    actual_duration = time.time() - start_time
    total_requests = successful_requests + failed_requests
    throughput = total_requests / actual_duration if actual_duration > 0 else 0

    # Latency percentiles
    sorted_latencies = sorted(latencies)
    p50 = sorted_latencies[int(len(sorted_latencies) * 0.50)] if sorted_latencies else 0
    p95 = sorted_latencies[int(len(sorted_latencies) * 0.95)] if sorted_latencies else 0
    p99 = sorted_latencies[int(len(sorted_latencies) * 0.99)] if sorted_latencies else 0
    min_latency = min(sorted_latencies) if sorted_latencies else 0
    max_latency = max(sorted_latencies) if sorted_latencies else 0
    avg_latency = statistics.mean(sorted_latencies) if sorted_latencies else 0

    error_rate = (
        (failed_requests / total_requests * 100) if total_requests > 0 else 0
    )

    # System metrics
    cpu_percent = psutil.cpu_percent(interval=1)
    memory_mb = psutil.virtual_memory().used / (1024 * 1024)

    return TestResult(
        total_requests=total_requests,
        successful_requests=successful_requests,
        failed_requests=failed_requests,
        duration_seconds=actual_duration,
        throughput_tps=throughput,
        latencies_ms=sorted_latencies,
        p50_ms=p50,
        p95_ms=p95,
        p99_ms=p99,
        min_ms=min_latency,
        max_ms=max_latency,
        avg_ms=avg_latency,
        error_rate=error_rate,
        cpu_usage_percent=cpu_percent,
        memory_usage_mb=memory_mb,
    )


def print_results(result: TestResult):
    """Print test results"""
    print()
    print("=" * 70)
    print("LOAD TEST RESULTS")
    print("=" * 70)
    print()

    print("Throughput:")
    print(f"  Total Requests:      {result.total_requests:,}")
    print(f"  Successful:          {result.successful_requests:,}")
    print(f"  Failed:              {result.failed_requests:,}")
    print(f"  Duration:            {result.duration_seconds:.2f}s")
    print(f"  Throughput:          {result.throughput_tps:.2f} TPS")
    print(f"  Error Rate:          {result.error_rate:.2f}%")
    print()

    print("Latency:")
    print(f"  Min:                 {result.min_ms:.2f}ms")
    print(f"  Average:             {result.avg_ms:.2f}ms")
    print(f"  p50 (median):        {result.p50_ms:.2f}ms")
    print(f"  p95:                 {result.p95_ms:.2f}ms")
    print(f"  p99:                 {result.p99_ms:.2f}ms")
    print(f"  Max:                 {result.max_ms:.2f}ms")
    print()

    print("System Resources:")
    print(f"  CPU Usage:           {result.cpu_usage_percent:.1f}%")
    print(f"  Memory Usage:        {result.memory_usage_mb:.1f} MB")
    print()

    # Performance assessment
    print("Assessment:")

    if result.throughput_tps >= 1000:
        print("  ✓ Throughput target met (≥1000 TPS)")
    else:
        print(f"  ✗ Throughput below target: {result.throughput_tps:.0f} TPS (target: 1000 TPS)")

    if result.p95_ms <= 100:
        print("  ✓ Latency target met (p95 ≤100ms)")
    else:
        print(f"  ✗ Latency above target: p95={result.p95_ms:.0f}ms (target: ≤100ms)")

    if result.error_rate < 1.0:
        print("  ✓ Error rate acceptable (<1%)")
    else:
        print(f"  ✗ High error rate: {result.error_rate:.1f}%")

    print()


def save_results(result: TestResult, output_file: str):
    """Save results to JSON file"""
    data = {
        "timestamp": datetime.utcnow().isoformat() + "Z",
        "metrics": {
            "total_requests": result.total_requests,
            "successful_requests": result.successful_requests,
            "failed_requests": result.failed_requests,
            "duration_seconds": result.duration_seconds,
            "throughput_tps": result.throughput_tps,
            "error_rate_percent": result.error_rate,
        },
        "latency": {
            "min_ms": result.min_ms,
            "avg_ms": result.avg_ms,
            "p50_ms": result.p50_ms,
            "p95_ms": result.p95_ms,
            "p99_ms": result.p99_ms,
            "max_ms": result.max_ms,
        },
        "resources": {
            "cpu_usage_percent": result.cpu_usage_percent,
            "memory_usage_mb": result.memory_usage_mb,
        },
    }

    with open(output_file, "w") as f:
        json.dump(data, f, indent=2)

    print(f"Results saved to: {output_file}")


async def main():
    """Main entry point"""
    parser = argparse.ArgumentParser(description="DelTran Load Testing")
    parser.add_argument(
        "--target",
        default="http://localhost:8080",
        help="Target service URL",
    )
    parser.add_argument(
        "--duration",
        type=int,
        default=60,
        help="Test duration in seconds",
    )
    parser.add_argument(
        "--rps",
        type=int,
        default=1000,
        help="Target requests per second",
    )
    parser.add_argument(
        "--workers",
        type=int,
        default=100,
        help="Number of concurrent workers",
    )
    parser.add_argument(
        "--output",
        default="load_test_results.json",
        help="Output file for results",
    )

    args = parser.parse_args()

    # Run load test
    result = await run_load_test(
        target_url=args.target,
        duration_seconds=args.duration,
        target_rps=args.rps,
        num_workers=args.workers,
    )

    # Print results
    print_results(result)

    # Save results
    save_results(result, args.output)


if __name__ == "__main__":
    asyncio.run(main())