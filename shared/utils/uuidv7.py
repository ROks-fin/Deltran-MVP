import time
import uuid
from typing import Union


def generate_uuidv7() -> uuid.UUID:
    """Generate UUIDv7 with timestamp ordering"""
    # Get current timestamp in milliseconds
    timestamp_ms = int(time.time() * 1000)

    # Create UUIDv7
    # 48 bits: timestamp (milliseconds since Unix epoch)
    # 12 bits: random data for sub-millisecond ordering
    # 4 bits: version (0111 for version 7)
    # 62 bits: random data
    # 2 bits: variant (10)

    # Convert timestamp to 48-bit value
    timestamp_high = (timestamp_ms >> 16) & 0xFFFFFFFF  # Upper 32 bits
    timestamp_low = timestamp_ms & 0xFFFF  # Lower 16 bits

    # Generate random data for sub-millisecond ordering (12 bits)
    random_a = uuid.uuid4().int & 0xFFF

    # Generate random data for the rest (62 bits)
    random_b = uuid.uuid4().int & 0x3FFFFFFFFFFFFFFF

    # Construct the UUID
    # Format: TTTTTTTT-TTTT-7RRR-YRRR-RRRRRRRRRRRR
    # Where T = timestamp, R = random, Y = variant bits (10)

    uuid_int = (
        (timestamp_high << 96) |  # Bits 127-96: timestamp high
        (timestamp_low << 80) |   # Bits 95-80: timestamp low
        (7 << 76) |              # Bits 79-76: version 7
        (random_a << 64) |       # Bits 75-64: random A
        (2 << 62) |              # Bits 63-62: variant (10)
        random_b                 # Bits 61-0: random B
    )

    return uuid.UUID(int=uuid_int)


def extract_timestamp_from_uuidv7(uuid_obj: Union[str, uuid.UUID]) -> float:
    """Extract timestamp from UUIDv7"""
    if isinstance(uuid_obj, str):
        uuid_obj = uuid.UUID(uuid_obj)

    if uuid_obj.version != 7:
        raise ValueError("UUID is not version 7")

    # Extract 48-bit timestamp from the UUID
    uuid_int = uuid_obj.int
    timestamp_ms = (uuid_int >> 80) & 0xFFFFFFFFFFFF

    return timestamp_ms / 1000.0


def is_uuidv7(uuid_obj: Union[str, uuid.UUID]) -> bool:
    """Check if UUID is version 7"""
    try:
        if isinstance(uuid_obj, str):
            uuid_obj = uuid.UUID(uuid_obj)
        return uuid_obj.version == 7
    except:
        return False


def generate_uetr() -> uuid.UUID:
    """Generate UETR (Unique End-to-end Transaction Reference) as UUIDv4"""
    return uuid.uuid4()


def validate_uetr(uetr: Union[str, uuid.UUID]) -> bool:
    """Validate UETR format"""
    try:
        if isinstance(uetr, str):
            uetr = uuid.UUID(uetr)
        return uetr.version == 4
    except:
        return False