#!/usr/bin/env python3
"""
Professional Bank-Grade Multi-Region Stress Test
=================================================

4-Bank Scenario:
- UAE Bank (Dubai, GMT+4) - AED, USD
- Israel Bank (Tel Aviv, GMT+2) - ILS, USD, EUR
- Pakistan Bank (Karachi, GMT+5) - PKR, USD
- India Bank (Mumbai, GMT+5:30) - INR, USD

Target: 3000 TPS sustained load
Focus Areas:
- Clearing windows (region-specific)
- FX rate volatility and spikes
- Payment netting and multilateral settlement
- Liquidity provider integration
- Risk assessment under stress
- Real-time web dashboard updates

This test simulates realistic cross-border payment patterns with:
1. Domestic payments (within country)
2. Regional payments (UAE<->Israel, Pakistan<->India)
3. Cross-regional payments (UAE<->India, Israel<->Pakistan)
4. Clearing window boundaries
5. FX rate fluctuations
6. Liquidity constraints and rebalancing
"""

import asyncio
import aiohttp
import time
import random
import json
import sys
from datetime import datetime, timedelta
from typing import Dict, List, Tuple
from dataclasses import dataclass, asdict
from decimal import Decimal
import pytz

# ============================================
# CONFIGURATION
# ============================================

API_BASE_URL = "http://localhost:8080/api/v1"
TARGET_TPS = 3000
TEST_DURATION_SECONDS = 300  # 5 minutes
RAMP_UP_SECONDS = 30
REPORT_INTERVAL = 5  # Report every 5 seconds

# ============================================
# BANK DEFINITIONS
# ============================================

@dataclass
class Bank:
    """Represents a participating bank"""
    id: str
    name: str
    bic: str
    country: str
    timezone: str
    primary_currency: str
    secondary_currencies: List[str]
    clearing_window_start: str  # HH:MM local time
    clearing_window_end: str    # HH:MM local time
    daily_liquidity_limit: Decimal  # USD equivalent

    def is_in_clearing_window(self) -> bool:
        """Check if current time is within clearing window"""
        tz = pytz.timezone(self.timezone)
        now_local = datetime.now(tz)
        current_time = now_local.time()

        start_hour, start_min = map(int, self.clearing_window_start.split(':'))
        end_hour, end_min = map(int, self.clearing_window_end.split(':'))

        start_time = datetime.strptime(self.clearing_window_start, '%H:%M').time()
        end_time = datetime.strptime(self.clearing_window_end, '%H:%M').time()

        return start_time <= current_time <= end_time

BANKS = {
    "UAE": Bank(
        id="bank-uae-001",
        name="Emirates National Bank",
        bic="ENBXAEADXXX",
        country="AE",
        timezone="Asia/Dubai",
        primary_currency="AED",
        secondary_currencies=["USD", "EUR", "SAR"],
        clearing_window_start="08:00",
        clearing_window_end="16:00",
        daily_liquidity_limit=Decimal("500000000")  # 500M USD
    ),
    "ISRAEL": Bank(
        id="bank-il-001",
        name="Bank Leumi Israel",
        bic="LUMIILITXXX",
        country="IL",
        timezone="Asia/Jerusalem",
        primary_currency="ILS",
        secondary_currencies=["USD", "EUR", "GBP"],
        clearing_window_start="09:00",
        clearing_window_end="17:00",
        daily_liquidity_limit=Decimal("300000000")  # 300M USD
    ),
    "PAKISTAN": Bank(
        id="bank-pk-001",
        name="Habib Bank Limited",
        bic="HABBPKKKXXX",
        country="PK",
        timezone="Asia/Karachi",
        primary_currency="PKR",
        secondary_currencies=["USD", "AED", "SAR"],
        clearing_window_start="09:00",
        clearing_window_end="17:00",
        daily_liquidity_limit=Decimal("200000000")  # 200M USD
    ),
    "INDIA": Bank(
        id="bank-in-001",
        name="State Bank of India",
        bic="SBININBBXXX",
        country="IN",
        timezone="Asia/Kolkata",
        primary_currency="INR",
        secondary_currencies=["USD", "EUR", "AED"],
        clearing_window_start="10:00",
        clearing_window_end="18:00",
        daily_liquidity_limit=Decimal("800000000")  # 800M USD
    )
}

# ============================================
# FX RATES WITH VOLATILITY
# ============================================

class FXRateEngine:
    """Simulates realistic FX rate volatility"""

    def __init__(self):
        # Base rates (to USD)
        self.base_rates = {
            "USD": Decimal("1.0"),
            "AED": Decimal("3.6725"),
            "ILS": Decimal("3.65"),
            "PKR": Decimal("278.50"),
            "INR": Decimal("83.25"),
            "EUR": Decimal("0.92"),
            "GBP": Decimal("0.79"),
            "SAR": Decimal("3.75")
        }

        # Volatility (standard deviation as % of rate)
        self.volatility = {
            "USD": 0.0,
            "AED": 0.001,  # Pegged to USD, very low volatility
            "ILS": 0.015,  # Moderate volatility
            "PKR": 0.025,  # Higher volatility
            "INR": 0.018,  # Moderate volatility
            "EUR": 0.012,
            "GBP": 0.013,
            "SAR": 0.001   # Pegged to USD
        }

        # Spike simulation
        self.spike_probability = 0.001  # 0.1% chance per rate fetch
        self.spike_magnitude = 0.05     # 5% spike

    def get_rate(self, from_currency: str, to_currency: str) -> Decimal:
        """Get current FX rate with volatility"""
        if from_currency == to_currency:
            return Decimal("1.0")

        # Convert to USD first, then to target
        from_rate = self._get_rate_with_volatility(from_currency)
        to_rate = self._get_rate_with_volatility(to_currency)

        rate = to_rate / from_rate
        return rate.quantize(Decimal("0.000001"))

    def _get_rate_with_volatility(self, currency: str) -> Decimal:
        """Apply volatility and potential spikes"""
        base_rate = self.base_rates[currency]
        volatility = self.volatility[currency]

        # Normal volatility (Brownian motion)
        change = float(base_rate) * volatility * random.gauss(0, 1)
        rate = base_rate + Decimal(str(change))

        # Random spike
        if random.random() < self.spike_probability:
            spike_direction = 1 if random.random() < 0.5 else -1
            spike = float(rate) * self.spike_magnitude * spike_direction
            rate += Decimal(str(spike))
            print(f"  [FX SPIKE] {currency}: {spike_direction * self.spike_magnitude * 100:.2f}% spike")

        return rate

fx_engine = FXRateEngine()

# ============================================
# PAYMENT PATTERNS
# ============================================

class PaymentPatternGenerator:
    """Generates realistic payment patterns"""

    # Payment amount distributions (in USD equivalent)
    AMOUNT_DISTRIBUTIONS = {
        "retail": (10, 5000),          # Small retail payments
        "corporate": (5000, 500000),   # Corporate payments
        "wholesale": (500000, 50000000), # Large wholesale
    }

    # Pattern weights (domestic, regional, cross-regional)
    PATTERN_WEIGHTS = {
        "domestic": 0.60,      # 60% domestic
        "regional": 0.25,      # 25% regional
        "cross_regional": 0.15 # 15% cross-regional
    }

    # Transaction type weights
    TYPE_WEIGHTS = {
        "retail": 0.70,
        "corporate": 0.25,
        "wholesale": 0.05
    }

    # Regional pairs
    REGIONAL_PAIRS = [
        ("UAE", "ISRAEL"),
        ("PAKISTAN", "INDIA"),
        ("UAE", "PAKISTAN"),
        ("ISRAEL", "INDIA")
    ]

    def generate_payment(self) -> Dict:
        """Generate a realistic payment"""
        # Select transaction type
        tx_type = random.choices(
            list(self.TYPE_WEIGHTS.keys()),
            weights=list(self.TYPE_WEIGHTS.values())
        )[0]

        # Select pattern
        pattern = random.choices(
            list(self.PATTERN_WEIGHTS.keys()),
            weights=list(self.PATTERN_WEIGHTS.values())
        )[0]

        # Select banks based on pattern
        if pattern == "domestic":
            sender_bank = receiver_bank = random.choice(list(BANKS.values()))
        elif pattern == "regional":
            sender_code, receiver_code = random.choice(self.REGIONAL_PAIRS)
            sender_bank = BANKS[sender_code]
            receiver_bank = BANKS[receiver_code]
        else:  # cross_regional
            sender_bank, receiver_bank = random.sample(list(BANKS.values()), 2)

        # Generate amount
        min_amt, max_amt = self.AMOUNT_DISTRIBUTIONS[tx_type]
        amount_usd = Decimal(str(random.uniform(min_amt, max_amt)))

        # Select currency (prefer sender's primary or common USD)
        if random.random() < 0.6:
            currency = sender_bank.primary_currency
        else:
            currency = "USD"

        # Convert amount to selected currency
        if currency != "USD":
            rate = fx_engine.get_rate("USD", currency)
            amount = (amount_usd * rate).quantize(Decimal("0.01"))
        else:
            amount = amount_usd.quantize(Decimal("0.01"))

        # Generate realistic reference
        ref_prefix = {
            "retail": "RTL",
            "corporate": "CRP",
            "wholesale": "WSL"
        }[tx_type]

        reference = f"{ref_prefix}{int(time.time() * 1000) % 1000000:06d}"

        return {
            "sender_bic": sender_bank.bic,
            "receiver_bic": receiver_bank.bic,
            "amount": str(amount),
            "currency": currency,
            "ordering_customer": f"{sender_bank.name}/Customer/{random.randint(10000, 99999)}",
            "beneficiary": f"{receiver_bank.name}/Account/{random.randint(10000, 99999)}",
            "reference": reference,
            "remittance_info": f"{tx_type.upper()} payment {reference}",
            "tx_type": tx_type,
            "pattern": pattern
        }

payment_generator = PaymentPatternGenerator()

# ============================================
# LIQUIDITY PROVIDER POOL
# ============================================

class LiquidityProvider:
    """Manages liquidity pool and rebalancing"""

    def __init__(self):
        # Initialize liquidity pools (in currency units)
        self.pools = {
            "AED": Decimal("1000000000"),  # 1B AED
            "ILS": Decimal("500000000"),   # 500M ILS
            "PKR": Decimal("50000000000"), # 50B PKR
            "INR": Decimal("60000000000"), # 60B INR
            "USD": Decimal("2000000000"),  # 2B USD
            "EUR": Decimal("500000000"),   # 500M EUR
        }

        # Thresholds for rebalancing (as % of initial)
        self.low_threshold = Decimal("0.20")  # 20%
        self.high_threshold = Decimal("0.80")  # 80%

        self.initial_pools = self.pools.copy()
        self.rebalance_count = 0

    def deduct(self, currency: str, amount: Decimal) -> bool:
        """Deduct from liquidity pool"""
        if self.pools[currency] >= amount:
            self.pools[currency] -= amount
            self._check_rebalance(currency)
            return True
        return False

    def add(self, currency: str, amount: Decimal):
        """Add to liquidity pool"""
        self.pools[currency] += amount
        self._check_rebalance(currency)

    def _check_rebalance(self, currency: str):
        """Check if rebalancing is needed"""
        current = self.pools[currency]
        initial = self.initial_pools[currency]
        ratio = current / initial

        if ratio < self.low_threshold or ratio > self.high_threshold:
            self._rebalance(currency)

    def _rebalance(self, currency: str):
        """Simulate liquidity rebalancing"""
        self.rebalance_count += 1
        current = self.pools[currency]
        initial = self.initial_pools[currency]
        target = initial * Decimal("0.5")  # Rebalance to 50%

        difference = target - current
        direction = "BUY" if difference > 0 else "SELL"

        print(f"  [LIQUIDITY REBALANCE #{self.rebalance_count}] {direction} {abs(difference):,.2f} {currency} (pool: {current:,.2f} -> {target:,.2f})")

        # Simulate rebalancing (instant for test)
        self.pools[currency] = target

    def get_status(self) -> Dict:
        """Get liquidity pool status"""
        status = {}
        for currency, current in self.pools.items():
            initial = self.initial_pools[currency]
            ratio = float(current / initial * 100)
            status[currency] = {
                "current": float(current),
                "initial": float(initial),
                "ratio_pct": ratio
            }
        return status

liquidity_provider = LiquidityProvider()

# ============================================
# METRICS COLLECTOR
# ============================================

class MetricsCollector:
    """Collects and reports test metrics"""

    def __init__(self):
        self.start_time = time.time()
        self.total_sent = 0
        self.total_success = 0
        self.total_failed = 0
        self.total_volume = {}  # By currency
        self.status_counts = {}
        self.latencies = []
        self.tps_history = []
        self.last_report_time = self.start_time
        self.last_report_count = 0

        self.by_pattern = {
            "domestic": 0,
            "regional": 0,
            "cross_regional": 0
        }

        self.by_type = {
            "retail": 0,
            "corporate": 0,
            "wholesale": 0
        }

        self.clearing_window_violations = 0
        self.fx_spikes_encountered = 0

    def record_success(self, payment: Dict, latency: float):
        """Record successful payment"""
        self.total_sent += 1
        self.total_success += 1
        self.latencies.append(latency)

        currency = payment['currency']
        amount = Decimal(payment['amount'])
        self.total_volume[currency] = self.total_volume.get(currency, Decimal("0")) + amount

        self.by_pattern[payment.get('pattern', 'domestic')] += 1
        self.by_type[payment.get('tx_type', 'retail')] += 1

    def record_failure(self, payment: Dict):
        """Record failed payment"""
        self.total_sent += 1
        self.total_failed += 1

    def get_current_tps(self) -> float:
        """Calculate current TPS"""
        now = time.time()
        elapsed = now - self.last_report_time
        if elapsed < 1:
            return 0

        count_delta = self.total_sent - self.last_report_count
        tps = count_delta / elapsed
        return tps

    def report(self):
        """Print periodic report"""
        now = time.time()
        elapsed = now - self.start_time

        current_tps = self.get_current_tps()
        self.tps_history.append(current_tps)

        avg_latency = sum(self.latencies[-1000:]) / len(self.latencies[-1000:]) if self.latencies else 0
        p95_latency = sorted(self.latencies[-1000:])[int(len(self.latencies[-1000:]) * 0.95)] if len(self.latencies) > 20 else 0
        p99_latency = sorted(self.latencies[-1000:])[int(len(self.latencies[-1000:]) * 0.99)] if len(self.latencies) > 20 else 0

        success_rate = (self.total_success / self.total_sent * 100) if self.total_sent > 0 else 0

        print(f"\n{'='*80}")
        print(f"[{datetime.now().strftime('%H:%M:%S')}] STRESS TEST METRICS (T+{elapsed:.0f}s)")
        print(f"{'='*80}")
        print(f"  TPS: {current_tps:.1f} (target: {TARGET_TPS})")
        print(f"  Total Payments: {self.total_sent:,} | Success: {self.total_success:,} | Failed: {self.total_failed:,}")
        print(f"  Success Rate: {success_rate:.2f}%")
        print(f"  Latency: avg={avg_latency*1000:.1f}ms | p95={p95_latency*1000:.1f}ms | p99={p99_latency*1000:.1f}ms")

        print(f"\n  Volume by Currency:")
        for currency, volume in sorted(self.total_volume.items()):
            print(f"    {currency}: {volume:,.2f}")

        print(f"\n  Payment Patterns:")
        for pattern, count in self.by_pattern.items():
            pct = (count / self.total_sent * 100) if self.total_sent > 0 else 0
            print(f"    {pattern}: {count:,} ({pct:.1f}%)")

        print(f"\n  Transaction Types:")
        for tx_type, count in self.by_type.items():
            pct = (count / self.total_sent * 100) if self.total_sent > 0 else 0
            print(f"    {tx_type}: {count:,} ({pct:.1f}%)")

        print(f"\n  Clearing Window Violations: {self.clearing_window_violations}")
        print(f"  Liquidity Rebalances: {liquidity_provider.rebalance_count}")

        # Liquidity status
        print(f"\n  Liquidity Pool Status:")
        for currency, status in liquidity_provider.get_status().items():
            print(f"    {currency}: {status['ratio_pct']:.1f}% of initial ({status['current']:,.0f} / {status['initial']:,.0f})")

        print(f"{'='*80}\n")

        self.last_report_time = now
        self.last_report_count = self.total_sent

    def final_report(self):
        """Print final test report"""
        elapsed = time.time() - self.start_time

        avg_tps = self.total_sent / elapsed
        max_tps = max(self.tps_history) if self.tps_history else 0
        min_tps = min(self.tps_history) if self.tps_history else 0

        avg_latency = sum(self.latencies) / len(self.latencies) if self.latencies else 0
        p95_latency = sorted(self.latencies)[int(len(self.latencies) * 0.95)] if len(self.latencies) > 20 else 0
        p99_latency = sorted(self.latencies)[int(len(self.latencies) * 0.99)] if len(self.latencies) > 20 else 0

        success_rate = (self.total_success / self.total_sent * 100) if self.total_sent > 0 else 0

        print(f"\n{'='*80}")
        print(f"FINAL TEST REPORT")
        print(f"{'='*80}")
        print(f"Test Duration: {elapsed:.1f}s")
        print(f"Target TPS: {TARGET_TPS} | Achieved: {avg_tps:.1f} (max: {max_tps:.1f}, min: {min_tps:.1f})")
        print(f"Total Payments: {self.total_sent:,}")
        print(f"Success: {self.total_success:,} ({success_rate:.2f}%)")
        print(f"Failed: {self.total_failed:,}")
        print(f"\nLatency Statistics:")
        print(f"  Average: {avg_latency*1000:.1f}ms")
        print(f"  P95: {p95_latency*1000:.1f}ms")
        print(f"  P99: {p99_latency*1000:.1f}ms")
        print(f"\nTotal Volume by Currency:")
        for currency, volume in sorted(self.total_volume.items()):
            usd_equivalent = volume / fx_engine.get_rate(currency, "USD") if currency != "USD" else volume
            print(f"  {currency}: {volume:,.2f} (~${usd_equivalent:,.2f})")

        print(f"\nClearing Window Violations: {self.clearing_window_violations}")
        print(f"Liquidity Rebalances: {liquidity_provider.rebalance_count}")
        print(f"{'='*80}\n")

        # Save detailed report
        report_data = {
            "test_config": {
                "target_tps": TARGET_TPS,
                "duration_seconds": TEST_DURATION_SECONDS,
                "banks": [asdict(bank) for bank in BANKS.values()]
            },
            "results": {
                "total_payments": self.total_sent,
                "successful": self.total_success,
                "failed": self.total_failed,
                "success_rate_pct": success_rate,
                "avg_tps": avg_tps,
                "max_tps": max_tps,
                "min_tps": min_tps,
                "latency_ms": {
                    "average": avg_latency * 1000,
                    "p95": p95_latency * 1000,
                    "p99": p99_latency * 1000
                },
                "volume_by_currency": {k: str(v) for k, v in self.total_volume.items()},
                "patterns": dict(self.by_pattern),
                "types": dict(self.by_type),
                "clearing_window_violations": self.clearing_window_violations,
                "liquidity_rebalances": liquidity_provider.rebalance_count
            }
        }

        report_filename = f"stress_test_report_{datetime.now().strftime('%Y%m%d_%H%M%S')}.json"
        with open(report_filename, 'w') as f:
            json.dump(report_data, f, indent=2)

        print(f"Detailed report saved to: {report_filename}")

metrics = MetricsCollector()

# ============================================
# PAYMENT SENDER
# ============================================

async def send_payment(session: aiohttp.ClientSession, payment: Dict) -> Tuple[bool, float]:
    """Send a single payment"""
    start_time = time.time()

    try:
        # Check clearing window
        sender_bank = next(b for b in BANKS.values() if b.bic == payment['sender_bic'])
        receiver_bank = next(b for b in BANKS.values() if b.bic == payment['receiver_bic'])

        if not sender_bank.is_in_clearing_window() or not receiver_bank.is_in_clearing_window():
            metrics.clearing_window_violations += 1

        # Check liquidity
        currency = payment['currency']
        amount = Decimal(payment['amount'])

        if not liquidity_provider.deduct(currency, amount):
            # Insufficient liquidity, trigger rebalance
            liquidity_provider._rebalance(currency)

        # Generate idempotency key
        idempotency_key = f"stress-test-{int(time.time() * 1000000)}-{random.randint(1000, 9999)}"

        headers = {
            "Content-Type": "application/json",
            "Idempotency-Key": idempotency_key
        }

        async with session.post(
            f"{API_BASE_URL}/payments/initiate",
            json=payment,
            headers=headers,
            timeout=aiohttp.ClientTimeout(total=10)
        ) as response:
            latency = time.time() - start_time

            if response.status in [200, 201]:
                metrics.record_success(payment, latency)

                # Add back to receiver's liquidity pool
                liquidity_provider.add(currency, amount)
                return True, latency
            else:
                metrics.record_failure(payment)
                # Return liquidity
                liquidity_provider.add(currency, amount)
                return False, latency

    except Exception as e:
        latency = time.time() - start_time
        metrics.record_failure(payment)
        return False, latency

# ============================================
# LOAD GENERATOR
# ============================================

async def generate_load():
    """Generate sustained load at target TPS"""
    print(f"Starting stress test: {TARGET_TPS} TPS for {TEST_DURATION_SECONDS}s")
    print(f"Ramp-up period: {RAMP_UP_SECONDS}s\n")

    connector = aiohttp.TCPConnector(limit=500, limit_per_host=500)
    async with aiohttp.ClientSession(connector=connector) as session:
        start_time = time.time()
        next_report_time = start_time + REPORT_INTERVAL

        while time.time() - start_time < TEST_DURATION_SECONDS:
            current_time = time.time()
            elapsed = current_time - start_time

            # Ramp-up logic
            if elapsed < RAMP_UP_SECONDS:
                current_target_tps = int(TARGET_TPS * (elapsed / RAMP_UP_SECONDS))
            else:
                current_target_tps = TARGET_TPS

            # Calculate how many payments to send this iteration
            interval = 0.1  # 100ms intervals
            payments_per_interval = max(1, int(current_target_tps * interval))

            # Generate and send payments
            tasks = []
            for _ in range(payments_per_interval):
                payment = payment_generator.generate_payment()
                tasks.append(send_payment(session, payment))

            await asyncio.gather(*tasks, return_exceptions=True)

            # Periodic reporting
            if current_time >= next_report_time:
                metrics.report()
                next_report_time = current_time + REPORT_INTERVAL

            # Sleep to maintain target rate
            await asyncio.sleep(interval)

    # Final report
    metrics.final_report()

# ============================================
# MAIN
# ============================================

def main():
    """Main entry point"""
    print("""
================================================================================

         DELTRAN PROFESSIONAL MULTI-REGION BANK STRESS TEST

  4-Bank Scenario: UAE - Israel - Pakistan - India
  Target: 3000 TPS Sustained Load

  Focus:
    > Clearing windows (timezone-aware)
    > FX rate volatility and spikes
    > Payment netting and multilateral settlement
    > Liquidity provider pool
    > Real-time web dashboard integration

================================================================================
    """)

    print("\nBank Configuration:")
    print("=" * 80)
    for code, bank in BANKS.items():
        tz = pytz.timezone(bank.timezone)
        local_time = datetime.now(tz).strftime("%H:%M:%S")
        in_window = "[OPEN]" if bank.is_in_clearing_window() else "[CLOSED]"

        print(f"  [{code}] {bank.name}")
        print(f"    BIC: {bank.bic} | Currency: {bank.primary_currency}")
        print(f"    Timezone: {bank.timezone} (Local: {local_time})")
        print(f"    Clearing Window: {bank.clearing_window_start}-{bank.clearing_window_end} {in_window}")
        print(f"    Daily Limit: ${bank.daily_liquidity_limit:,.0f}")
        print()

    print("Starting in 3 seconds...")
    time.sleep(3)

    try:
        asyncio.run(generate_load())
    except KeyboardInterrupt:
        print("\nTest interrupted by user")
        metrics.final_report()
    except Exception as e:
        print(f"\nTest failed with error: {e}")
        import traceback
        traceback.print_exc()
        metrics.final_report()

if __name__ == "__main__":
    main()
