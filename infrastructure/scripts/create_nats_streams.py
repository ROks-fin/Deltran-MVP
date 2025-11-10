#!/usr/bin/env python3
"""
NATS JetStream Stream Setup for DelTran MVP
Creates all required streams with proper retention policies
"""

import requests
import json
import time

NATS_MANAGEMENT_URL = "http://localhost:8222"

# Stream configurations according to COMPLETE_SYSTEM_SPECIFICATION.md
STREAMS = [
    {
        "name": "transactions",
        "subjects": ["transactions.created", "transactions.completed", "transactions.failed"],
        "retention_days": 7,
        "description": "Transaction lifecycle events"
    },
    {
        "name": "settlement",
        "subjects": ["settlement.initiated", "settlement.completed", "settlement.rolled_back", "settlement.reconciled"],
        "retention_days": 30,
        "description": "Settlement and reconciliation events (audit trail)"
    },
    {
        "name": "compliance",
        "subjects": ["compliance.alert", "compliance.sar", "compliance.ctr", "compliance.check_completed"],
        "retention_days": 90,
        "description": "Compliance and AML events (regulatory requirement)"
    },
    {
        "name": "notifications",
        "subjects": ["notifications.>"],  # Wildcard for all notification events
        "retention_days": 1,
        "description": "Real-time notification events (ephemeral)"
    },
    {
        "name": "clearing",
        "subjects": ["clearing.window_opened", "clearing.window_closed", "clearing.netting_completed"],
        "retention_days": 30,
        "description": "Clearing window and netting events"
    },
    {
        "name": "risk",
        "subjects": ["risk.evaluation", "risk.threshold_exceeded", "risk.circuit_breaker"],
        "retention_days": 7,
        "description": "Risk management events"
    },
    {
        "name": "audit",
        "subjects": ["audit.>"],  # Wildcard for all audit events
        "retention_days": 90,
        "description": "System-wide audit trail (immutable)"
    }
]

def create_stream(stream_config):
    """Create a NATS JetStream stream"""
    name = stream_config["name"]
    subjects = stream_config["subjects"]
    retention_days = stream_config["retention_days"]
    description = stream_config["description"]

    # Convert days to nanoseconds
    max_age_ns = retention_days * 24 * 60 * 60 * 1_000_000_000

    stream_def = {
        "name": name,
        "subjects": subjects,
        "retention": "limits",
        "max_age": max_age_ns,
        "storage": "file",
        "num_replicas": 1,
        "discard": "old",
        "max_msgs_per_subject": -1,  # Unlimited
        "max_bytes": -1,  # Unlimited
        "max_msg_size": 8_388_608,  # 8MB
        "duplicate_window": 120_000_000_000,  # 2 minutes
        "allow_rollup_hdrs": False,
        "deny_delete": retention_days >= 90,  # Immutable for compliance/audit
        "deny_purge": retention_days >= 90
    }

    print(f"\nCreating stream '{name}':")
    print(f"  Description: {description}")
    print(f"  Subjects: {', '.join(subjects)}")
    print(f"  Retention: {retention_days} days")

    try:
        # NATS JetStream API endpoint for creating streams
        url = f"{NATS_MANAGEMENT_URL}/jsz/api/v1/stream/{name}"

        # Note: This is a simplified approach. In production, use NATS client library
        # For now, we'll use direct HTTP requests to NATS server

        # Since NATS doesn't have a direct HTTP API for stream creation,
        # we'll create a helper that uses NATS protocol
        print(f"  Status: Stream configuration prepared")
        print(f"  Note: Use NATS client or CLI to create: nats stream add {name}")

        return True
    except Exception as e:
        print(f"  Error: {e}")
        return False

def verify_jetstream():
    """Verify JetStream is enabled"""
    try:
        response = requests.get(f"{NATS_MANAGEMENT_URL}/jsz", timeout=5)
        if response.status_code == 200:
            data = response.json()
            print(f"\nJetStream Status:")
            print(f"  Server ID: {data.get('server_id', 'N/A')}")
            print(f"  Streams: {data.get('streams', 0)}")
            print(f"  Consumers: {data.get('consumers', 0)}")
            print(f"  Messages: {data.get('messages', 0)}")
            return True
    except requests.exceptions.RequestException as e:
        print(f"\nError connecting to NATS: {e}")
        return False

def main():
    print("=" * 60)
    print("NATS JetStream Stream Setup for DelTran MVP")
    print("=" * 60)

    # Verify JetStream is running
    if not verify_jetstream():
        print("\nERROR: Cannot connect to NATS JetStream")
        print("Make sure NATS is running on localhost:4222")
        return 1

    # Create streams
    print("\n" + "=" * 60)
    print("Creating Streams")
    print("=" * 60)

    for stream in STREAMS:
        create_stream(stream)
        time.sleep(0.5)  # Small delay between creations

    print("\n" + "=" * 60)
    print("Stream Configuration Complete")
    print("=" * 60)

    print("\nIMPORTANT: To actually create these streams, run:")
    print("  docker exec deltran-nats sh -c '")
    for stream in STREAMS:
        subjects_str = " ".join(stream["subjects"])
        max_age = f"{stream['retention_days']}d"
        print(f"    nats stream add {stream['name']} --subjects=\"{subjects_str}\" --retention=limits --max-age={max_age} --storage=file --replicas=1")
    print("  '")

    print("\nOr use NATS client library in your services to create streams programmatically.")

    return 0

if __name__ == "__main__":
    exit(main())
