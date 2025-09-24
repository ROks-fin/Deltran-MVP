#!/usr/bin/env python3

"""
DelTran Rail MVP - Seed Data Generator
Generates test data for banks, accounts, and sample payments
"""

import asyncio
import random
import sys
from datetime import datetime, timedelta
from decimal import Decimal
from uuid import uuid4

# Add shared to path
sys.path.append('../')

from shared.clients.postgres_client import postgres_client
from shared.utils.uuidv7 import generate_uuidv7, generate_uetr

# Sample banks and BIC codes
BANKS = [
    {"name": "Emirates NBD", "bic": "EBILAEAD", "country": "AE", "currency": "AED"},
    {"name": "First Abu Dhabi Bank", "bic": "NBADAEAD", "country": "AE", "currency": "AED"},
    {"name": "State Bank of India", "bic": "SBININBB", "country": "IN", "currency": "INR"},
    {"name": "HDFC Bank", "bic": "HDFCINBB", "country": "IN", "currency": "INR"},
    {"name": "JPMorgan Chase", "bic": "CHASUS33", "country": "US", "currency": "USD"},
    {"name": "Bank of America", "bic": "BOFAUS3N", "country": "US", "currency": "USD"},
    {"name": "Deutsche Bank", "bic": "DEUTDEFF", "country": "DE", "currency": "EUR"},
    {"name": "BNP Paribas", "bic": "BNPAFRPP", "country": "FR", "currency": "EUR"},
    {"name": "HSBC UK", "bic": "HBUKGB4B", "country": "GB", "currency": "GBP"},
    {"name": "Barclays", "bic": "BARCGB22", "country": "GB", "currency": "GBP"},
]

# Payment purposes
PAYMENT_PURPOSES = [
    "TRADE", "SERVICES", "INVESTMENT", "PERSONAL", "GOVERNMENT", "CHARITY"
]

# Settlement methods
SETTLEMENT_METHODS = ["INSTANT", "PVP", "NETTING", "CORRESPONDENT"]


def generate_iban(country_code: str, bank_code: str = None) -> str:
    """Generate a realistic-looking IBAN"""
    if country_code == "AE":
        # UAE IBAN format: AE07 0331 2345 6789 0123 456
        return f"AE{random.randint(10,99)}{random.randint(1000,9999)}{random.randint(100000000000000,999999999999999)}"
    elif country_code == "IN":
        # Indian account number format
        return f"IN{random.randint(10,99)}{random.randint(1000,9999)}{random.randint(100000000000,999999999999)}"
    elif country_code == "US":
        # US account format
        return f"US{random.randint(10,99)}{random.randint(100000000000000000000000,999999999999999999999999)}"
    elif country_code == "GB":
        # UK IBAN format
        return f"GB{random.randint(10,99)}{random.choice(['BARC', 'HSBC'])}{random.randint(10000000,99999999)}"
    elif country_code == "DE":
        # German IBAN format
        return f"DE{random.randint(10,99)}{random.randint(10000000,99999999)}{random.randint(1000000000,9999999999)}"
    elif country_code == "FR":
        # French IBAN format
        return f"FR{random.randint(10,99)}{random.randint(10000,99999)}{random.randint(100000000000,999999999999)}1"
    else:
        # Generic format
        return f"{country_code}{random.randint(10,99)}{random.randint(100000000000000000,999999999999999999)}"


def generate_account_data(banks: list) -> list:
    """Generate test account data"""
    accounts = []

    for bank in banks:
        # Create 10-50 accounts per bank
        num_accounts = random.randint(10, 50)
        for _ in range(num_accounts):
            account = {
                "account_id": str(uuid4()),
                "iban": generate_iban(bank["country"]),
                "bank_name": bank["name"],
                "bic": bank["bic"],
                "currency": bank["currency"],
                "country": bank["country"],
                "account_type": random.choice(["CHECKING", "SAVINGS", "CORPORATE", "NOSTRO", "VOSTRO"]),
                "balance": Decimal(str(random.uniform(10000, 10000000))).quantize(Decimal('0.01')),
                "status": "ACTIVE",
                "created_at": datetime.utcnow()
            }
            accounts.append(account)

    return accounts


def generate_micropayments(accounts: list, count: int = 1000) -> list:
    """Generate micropayment test data"""
    payments = []

    # Exchange rates for conversion
    rates = {
        ("USD", "AED"): 3.67,
        ("USD", "INR"): 83.0,
        ("AED", "INR"): 22.6,
        ("USD", "EUR"): 0.85,
        ("USD", "GBP"): 0.75,
        ("EUR", "GBP"): 0.88
    }

    for i in range(count):
        debtor = random.choice(accounts)
        creditor = random.choice([acc for acc in accounts if acc["country"] != debtor["country"]])

        # Generate payment amount (micropayments: $1-$1000 equivalent)
        if debtor["currency"] == "USD":
            amount = Decimal(str(random.uniform(1, 1000))).quantize(Decimal('0.01'))
        elif debtor["currency"] == "AED":
            amount = Decimal(str(random.uniform(3.67, 3670))).quantize(Decimal('0.01'))
        elif debtor["currency"] == "INR":
            amount = Decimal(str(random.uniform(83, 83000))).quantize(Decimal('0.01'))
        elif debtor["currency"] == "EUR":
            amount = Decimal(str(random.uniform(0.85, 850))).quantize(Decimal('0.01'))
        elif debtor["currency"] == "GBP":
            amount = Decimal(str(random.uniform(0.75, 750))).quantize(Decimal('0.01'))
        else:
            amount = Decimal(str(random.uniform(1, 1000))).quantize(Decimal('0.01'))

        payment = {
            "transaction_id": str(generate_uuidv7()),
            "uetr": str(generate_uetr()),
            "amount": str(amount),
            "currency": debtor["currency"],
            "debtor_account": debtor["iban"],
            "creditor_account": creditor["iban"],
            "debtor_bic": debtor["bic"],
            "creditor_bic": creditor["bic"],
            "payment_purpose": random.choice(PAYMENT_PURPOSES),
            "settlement_method": random.choice(SETTLEMENT_METHODS),
            "status": random.choice(["INITIATED", "VALIDATED", "APPROVED", "SETTLED", "COMPLETED"]),
            "created_at": datetime.utcnow() - timedelta(minutes=random.randint(0, 10080)),  # Last week
            "idempotency_key": str(uuid4())
        }
        payments.append(payment)

    return payments


def generate_aed_inr_scenarios(accounts: list) -> list:
    """Generate specific AED↔INR test scenarios"""
    aed_accounts = [acc for acc in accounts if acc["currency"] == "AED"]
    inr_accounts = [acc for acc in accounts if acc["currency"] == "INR"]

    if not aed_accounts or not inr_accounts:
        print("⚠️  No AED or INR accounts available for scenarios")
        return []

    scenarios = []
    scenarios_data = [
        {"amount": "10000", "purpose": "TRADE", "description": "Construction materials export"},
        {"amount": "50000", "purpose": "SERVICES", "description": "IT consulting services"},
        {"amount": "25000", "purpose": "TRADE", "description": "Textiles import"},
        {"amount": "75000", "purpose": "INVESTMENT", "description": "Real estate investment"},
        {"amount": "15000", "purpose": "PERSONAL", "description": "Family remittance"},
    ]

    for scenario in scenarios_data:
        # AED to INR
        aed_to_inr = {
            "transaction_id": str(generate_uuidv7()),
            "uetr": str(generate_uetr()),
            "amount": scenario["amount"],
            "currency": "AED",
            "debtor_account": random.choice(aed_accounts)["iban"],
            "creditor_account": random.choice(inr_accounts)["iban"],
            "payment_purpose": scenario["purpose"],
            "settlement_method": "PVP",
            "status": "INITIATED",
            "created_at": datetime.utcnow(),
            "idempotency_key": str(uuid4())
        }
        scenarios.append(aed_to_inr)

        # INR to AED (reverse flow)
        inr_amount = str(Decimal(scenario["amount"]) * Decimal("22.6"))  # Convert AED to INR
        inr_to_aed = {
            "transaction_id": str(generate_uuidv7()),
            "uetr": str(generate_uetr()),
            "amount": inr_amount,
            "currency": "INR",
            "debtor_account": random.choice(inr_accounts)["iban"],
            "creditor_account": random.choice(aed_accounts)["iban"],
            "payment_purpose": scenario["purpose"],
            "settlement_method": "PVP",
            "status": "INITIATED",
            "created_at": datetime.utcnow(),
            "idempotency_key": str(uuid4())
        }
        scenarios.append(inr_to_aed)

    return scenarios


async def seed_database():
    """Main seeding function"""
    print("🌱 Starting DelTran Rail MVP database seeding...")

    try:
        # Connect to database
        postgres_client.database_url = "postgresql://deltran:deltran123@localhost:5432/deltran"
        await postgres_client.connect()
        print("✅ Connected to database")

        # Generate test data
        print("📊 Generating test data...")

        # Accounts
        accounts = generate_account_data(BANKS)
        print(f"  Generated {len(accounts)} test accounts")

        # Micropayments
        micropayments = generate_micropayments(accounts, 1000)  # Start with 1K for demo
        print(f"  Generated {len(micropayments)} micropayments")

        # AED↔INR scenarios
        scenarios = generate_aed_inr_scenarios(accounts)
        print(f"  Generated {len(scenarios)} AED↔INR scenarios")

        # Insert data
        print("💾 Inserting data into database...")

        # Note: In a real implementation, we'd create an accounts table
        # For now, we'll just insert the payment data

        if micropayments:
            await postgres_client.bulk_upsert(
                "payments",
                micropayments,
                conflict_columns=["transaction_id"],
                update_columns=["status", "updated_at"]
            )
            print(f"  ✅ Inserted {len(micropayments)} micropayments")

        if scenarios:
            await postgres_client.bulk_upsert(
                "payments",
                scenarios,
                conflict_columns=["transaction_id"],
                update_columns=["status", "updated_at"]
            )
            print(f"  ✅ Inserted {len(scenarios)} scenario payments")

        # Generate some risk assessments
        print("🎯 Generating risk assessments...")
        risk_assessments = []
        for payment in (micropayments + scenarios)[:100]:  # First 100 payments
            risk_score = random.uniform(0, 100)
            factors = []

            if float(payment["amount"]) > 50000:
                factors.append("HIGH_VALUE")
            if payment["currency"] in ["AED", "INR"]:
                factors.append("HIGH_RISK_CURRENCY")
            if random.random() < 0.1:  # 10% chance
                factors.append("UNUSUAL_PATTERN")

            assessment = {
                "transaction_id": payment["transaction_id"],
                "risk_score": round(risk_score, 2),
                "risk_factors": factors,
                "recommended_action": "APPROVE" if risk_score < 20 else "ENHANCED_MONITORING" if risk_score < 60 else "MANUAL_REVIEW",
                "assessed_at": datetime.utcnow()
            }
            risk_assessments.append(assessment)

        if risk_assessments:
            await postgres_client.bulk_upsert(
                "risk_assessments",
                risk_assessments,
                conflict_columns=["transaction_id"],
                update_columns=["risk_score", "assessed_at"]
            )
            print(f"  ✅ Inserted {len(risk_assessments)} risk assessments")

        print("🎉 Database seeding completed successfully!")
        print(f"📈 Summary:")
        print(f"  • {len(micropayments)} micropayments")
        print(f"  • {len(scenarios)} AED↔INR scenarios")
        print(f"  • {len(risk_assessments)} risk assessments")
        print(f"  • {len(BANKS)} bank participants")

    except Exception as e:
        print(f"❌ Error during seeding: {e}")
        raise
    finally:
        await postgres_client.disconnect()


if __name__ == "__main__":
    asyncio.run(seed_database())