#!/usr/bin/env python3
"""
Comprehensive Component Test Suite
Tests each DelTran system component individually
"""

import asyncio
import aiohttp
import json
from datetime import datetime
from typing import Dict, List, Tuple

GATEWAY_URL = "http://localhost:8080"

class TestResults:
    def __init__(self):
        self.tests_run = 0
        self.tests_passed = 0
        self.tests_failed = 0
        self.failures = []

    def record_pass(self, test_name: str):
        self.tests_run += 1
        self.tests_passed += 1
        print(f"  [OK] {test_name}")

    def record_fail(self, test_name: str, reason: str):
        self.tests_run += 1
        self.tests_failed += 1
        self.failures.append((test_name, reason))
        print(f"  [FAIL] {test_name}: {reason}")

    def print_summary(self):
        print("\n" + "="*80)
        print("TEST SUMMARY")
        print("="*80)
        print(f"Total Tests: {self.tests_run}")
        print(f"Passed: {self.tests_passed} ({self.tests_passed/self.tests_run*100:.1f}%)")
        print(f"Failed: {self.tests_failed} ({self.tests_failed/self.tests_run*100:.1f}%)")

        if self.failures:
            print(f"\nFailures:")
            for test_name, reason in self.failures:
                print(f"  - {test_name}: {reason}")
        print("="*80 + "\n")


results = TestResults()


async def test_gateway_service(session: aiohttp.ClientSession):
    """Test Gateway Service API"""
    print("\nGATEWAY SERVICE TESTS")
    print("-" * 80)

    # Health check
    try:
        async with session.get(f"{GATEWAY_URL}/health") as resp:
            if resp.status == 200:
                data = await resp.json()
                if data.get("status") == "healthy":
                    results.record_pass("Health check")
                else:
                    results.record_fail("Health check", f"Unhealthy: {data}")
            else:
                results.record_fail("Health check", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Health check", str(e))

    # Bank registration
    test_bank = {
        "bank_id": "TEST001",
        "name": "Test Bank",
        "swift_code": "TESTUS33",
        "country": "US",
        "primary_currency": "USD"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/banks/register", json=test_bank) as resp:
            if resp.status in [200, 201, 409]:
                results.record_pass("Bank registration")
            else:
                results.record_fail("Bank registration", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Bank registration", str(e))

    # List banks
    try:
        async with session.get(f"{GATEWAY_URL}/api/banks") as resp:
            if resp.status == 200:
                data = await resp.json()
                if "banks" in data:
                    results.record_pass(f"List banks ({len(data['banks'])} found)")
                else:
                    results.record_fail("List banks", "No 'banks' field in response")
            else:
                results.record_fail("List banks", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("List banks", str(e))

    # Payment creation
    payment = {
        "sender_bank_id": "TEST001",
        "receiver_bank_id": "BANK001",
        "amount": 1000.00,
        "currency": "USD",
        "sender_account": "ACC123456",
        "receiver_account": "ACC789012",
        "reference": "TEST-PAYMENT-001",
        "sender_name": "Test Sender",
        "receiver_name": "Test Receiver"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/payments", json=payment) as resp:
            if resp.status in [200, 201]:
                data = await resp.json()
                results.record_pass(f"Payment creation (ID: {data.get('payment_id', 'unknown')})")
            else:
                text = await resp.text()
                results.record_fail("Payment creation", f"HTTP {resp.status}: {text[:100]}")
    except Exception as e:
        results.record_fail("Payment creation", str(e))

    # List payments
    try:
        async with session.get(f"{GATEWAY_URL}/api/payments") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"List payments ({len(data.get('payments', []))} found)")
            else:
                results.record_fail("List payments", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("List payments", str(e))


async def test_settlement_engine(session: aiohttp.ClientSession):
    """Test Settlement Engine"""
    print("\nSETTLEMENT ENGINE TESTS")
    print("-" * 80)

    # Batch creation
    try:
        async with session.post(f"{GATEWAY_URL}/api/batches/create") as resp:
            if resp.status in [200, 201]:
                data = await resp.json()
                results.record_pass(f"Batch creation (ID: {data.get('batch_id', 'unknown')})")
            else:
                text = await resp.text()
                results.record_fail("Batch creation", f"HTTP {resp.status}: {text[:100]}")
    except Exception as e:
        results.record_fail("Batch creation", str(e))

    # List batches
    try:
        async with session.get(f"{GATEWAY_URL}/api/batches") as resp:
            if resp.status == 200:
                data = await resp.json()
                batches = data.get('batches', [])
                results.record_pass(f"List batches ({len(batches)} found)")
            else:
                results.record_fail("List batches", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("List batches", str(e))

    # Netting status
    try:
        async with session.get(f"{GATEWAY_URL}/api/netting/status") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Netting status")
            else:
                results.record_fail("Netting status", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Netting status", str(e))

    # Settlement instruction
    settlement_data = {
        "batch_id": "BATCH-TEST-001",
        "currency": "USD",
        "amount": 5000.00,
        "participants": ["BANK001", "BANK002"]
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/settlement/execute", json=settlement_data) as resp:
            if resp.status in [200, 201, 400, 404]:  # 400/404 acceptable if batch doesn't exist
                results.record_pass("Settlement instruction")
            else:
                results.record_fail("Settlement instruction", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Settlement instruction", str(e))


async def test_compliance_service(session: aiohttp.ClientSession):
    """Test Compliance Service"""
    print("\nCOMPLIANCE SERVICE TESTS")
    print("-" * 80)

    # Compliance status
    try:
        async with session.get(f"{GATEWAY_URL}/api/compliance/status") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass("Compliance status")
            else:
                results.record_fail("Compliance status", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Compliance status", str(e))

    # Get limits
    try:
        async with session.get(f"{GATEWAY_URL}/api/limits") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Get compliance limits")
            else:
                results.record_fail("Get compliance limits", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Get compliance limits", str(e))

    # Set limit
    limit_data = {
        "bank_id": "TEST001",
        "limit_type": "daily",
        "amount": 1000000.00,
        "currency": "USD"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/limits/set", json=limit_data) as resp:
            if resp.status in [200, 201]:
                results.record_pass("Set compliance limit")
            else:
                text = await resp.text()
                results.record_fail("Set compliance limit", f"HTTP {resp.status}: {text[:100]}")
    except Exception as e:
        results.record_fail("Set compliance limit", str(e))

    # Check transaction
    check_data = {
        "bank_id": "TEST001",
        "amount": 50000.00,
        "currency": "USD",
        "counterparty": "BANK002"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/compliance/check", json=check_data) as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Compliance check (approved: {data.get('approved', False)})")
            else:
                results.record_fail("Compliance check", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Compliance check", str(e))


async def test_risk_engine(session: aiohttp.ClientSession):
    """Test Risk Engine"""
    print("\nRISK ENGINE TESTS")
    print("-" * 80)

    # Risk assessment
    risk_data = {
        "bank_id": "TEST001",
        "transaction_amount": 100000.00,
        "currency": "USD",
        "counterparty_id": "BANK002",
        "transaction_type": "payment"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/risk/assess", json=risk_data) as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Risk assessment (score: {data.get('risk_score', 'N/A')})")
            else:
                text = await resp.text()
                results.record_fail("Risk assessment", f"HTTP {resp.status}: {text[:100]}")
    except Exception as e:
        results.record_fail("Risk assessment", str(e))

    # Exposure calculation
    try:
        async with session.get(f"{GATEWAY_URL}/api/risk/exposure/TEST001") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Exposure calculation")
            else:
                results.record_fail("Exposure calculation", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Exposure calculation", str(e))

    # VaR calculation
    try:
        async with session.get(f"{GATEWAY_URL}/api/risk/var/TEST001") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass("VaR calculation")
            else:
                results.record_fail("VaR calculation", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("VaR calculation", str(e))


async def test_reconciliation(session: aiohttp.ClientSession):
    """Test Reconciliation Service"""
    print("\nRECONCILIATION TESTS")
    print("-" * 80)

    # Reconciliation status
    try:
        async with session.get(f"{GATEWAY_URL}/api/reconciliation/status") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass("Reconciliation status")
            else:
                results.record_fail("Reconciliation status", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Reconciliation status", str(e))

    # Start reconciliation
    recon_data = {
        "batch_id": "BATCH-TEST-001",
        "reconciliation_type": "payment"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/reconciliation/start", json=recon_data) as resp:
            if resp.status in [200, 201, 404]:  # 404 acceptable if batch doesn't exist
                results.record_pass("Start reconciliation")
            else:
                results.record_fail("Start reconciliation", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Start reconciliation", str(e))

    # Get discrepancies
    try:
        async with session.get(f"{GATEWAY_URL}/api/reconciliation/discrepancies") as resp:
            if resp.status == 200:
                data = await resp.json()
                count = len(data.get('discrepancies', []))
                results.record_pass(f"Get discrepancies ({count} found)")
            else:
                results.record_fail("Get discrepancies", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Get discrepancies", str(e))


async def test_message_bus(session: aiohttp.ClientSession):
    """Test Message Bus Integration"""
    print("\nMESSAGE BUS TESTS")
    print("-" * 80)

    # Publish test message
    message_data = {
        "topic": "test.payments",
        "payload": {
            "test": True,
            "timestamp": datetime.now().isoformat()
        }
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/messages/publish", json=message_data) as resp:
            if resp.status in [200, 201]:
                results.record_pass("Publish message")
            else:
                text = await resp.text()
                results.record_fail("Publish message", f"HTTP {resp.status}: {text[:100]}")
    except Exception as e:
        results.record_fail("Publish message", str(e))

    # Check message stats
    try:
        async with session.get(f"{GATEWAY_URL}/api/messages/stats") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Message stats")
            else:
                results.record_fail("Message stats", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Message stats", str(e))


async def test_ledger_core(session: aiohttp.ClientSession):
    """Test Ledger Core"""
    print("\nLEDGER CORE TESTS")
    print("-" * 80)

    # Create ledger entry
    ledger_entry = {
        "account_id": "TEST001",
        "amount": 1000.00,
        "currency": "USD",
        "entry_type": "credit",
        "reference": "TEST-LEDGER-001"
    }

    try:
        async with session.post(f"{GATEWAY_URL}/api/ledger/entry", json=ledger_entry) as resp:
            if resp.status in [200, 201]:
                results.record_pass("Create ledger entry")
            else:
                text = await resp.text()
                results.record_fail("Create ledger entry", f"HTTP {resp.status}: {text[:100]}")
    except Exception as e:
        results.record_fail("Create ledger entry", str(e))

    # Get balance
    try:
        async with session.get(f"{GATEWAY_URL}/api/ledger/balance/TEST001") as resp:
            if resp.status == 200:
                data = await resp.json()
                results.record_pass(f"Get balance ({data.get('balance', 'N/A')} {data.get('currency', '')})")
            else:
                results.record_fail("Get balance", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Get balance", str(e))

    # Get transaction history
    try:
        async with session.get(f"{GATEWAY_URL}/api/ledger/history/TEST001") as resp:
            if resp.status == 200:
                data = await resp.json()
                count = len(data.get('transactions', []))
                results.record_pass(f"Get transaction history ({count} transactions)")
            else:
                results.record_fail("Get transaction history", f"HTTP {resp.status}")
    except Exception as e:
        results.record_fail("Get transaction history", str(e))


async def main():
    """Run all component tests"""
    print("\n" + "="*80)
    print("DELTRAN COMPONENT TEST SUITE")
    print("="*80)

    async with aiohttp.ClientSession() as session:
        await test_gateway_service(session)
        await test_settlement_engine(session)
        await test_compliance_service(session)
        await test_risk_engine(session)
        await test_reconciliation(session)
        await test_message_bus(session)
        await test_ledger_core(session)

    results.print_summary()


if __name__ == "__main__":
    asyncio.run(main())
