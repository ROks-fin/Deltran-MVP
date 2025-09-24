import asyncio
import json
import logging
from typing import Any, Callable, Dict, Optional
from uuid import UUID

import nats
from nats.aio.client import Client as NATS
from nats.js.api import ConsumerConfig, StreamConfig
from nats.js.client import JetStreamContext

from shared.utils.uuidv7 import generate_uuidv7

logger = logging.getLogger(__name__)


class NATSClient:
    def __init__(self, url: str = "nats://localhost:4222"):
        self.url = url
        self.nc: Optional[NATS] = None
        self.js: Optional[JetStreamContext] = None
        self._subscriptions: Dict[str, Any] = {}

    async def connect(self):
        """Connect to NATS server and enable JetStream"""
        try:
            self.nc = await nats.connect(self.url)
            self.js = self.nc.jetstream()

            # Ensure core streams exist
            await self._create_core_streams()

            logger.info(f"Connected to NATS at {self.url}")
        except Exception as e:
            logger.error(f"Failed to connect to NATS: {e}")
            raise

    async def _create_core_streams(self):
        """Create core streams for the application"""
        streams = [
            {
                "name": "PAYMENTS",
                "subjects": ["payment.*", "settlement.*", "risk.*"],
                "max_age": 86400 * 7,  # 7 days
            },
            {
                "name": "LEDGER",
                "subjects": ["ledger.*", "block.*"],
                "max_age": 86400 * 365,  # 1 year
            },
            {
                "name": "COMPLIANCE",
                "subjects": ["compliance.*", "screening.*", "audit.*"],
                "max_age": 86400 * 2555,  # 7 years
            },
            {
                "name": "LIQUIDITY",
                "subjects": ["liquidity.*", "quote.*"],
                "max_age": 300,  # 5 minutes
            }
        ]

        for stream_def in streams:
            try:
                await self.js.stream_info(stream_def["name"])
                logger.info(f"Stream {stream_def['name']} already exists")
            except:
                config = StreamConfig(
                    name=stream_def["name"],
                    subjects=stream_def["subjects"],
                    max_age=stream_def["max_age"]
                )
                await self.js.add_stream(config)
                logger.info(f"Created stream {stream_def['name']}")

    async def disconnect(self):
        """Disconnect from NATS"""
        if self.nc:
            await self.nc.close()
            self.nc = None
            self.js = None
            logger.info("Disconnected from NATS")

    async def publish(self, subject: str, data: Dict[str, Any],
                     headers: Optional[Dict[str, str]] = None) -> str:
        """Publish a message to a subject"""
        if not self.js:
            raise RuntimeError("Not connected to NATS")

        # Add message ID and timestamp
        message_id = str(generate_uuidv7())
        data["message_id"] = message_id
        data["timestamp"] = data.get("timestamp",
                                   generate_uuidv7().hex[:8] + "-" +
                                   generate_uuidv7().hex[8:12] + "-" +
                                   generate_uuidv7().hex[12:16] + "-" +
                                   generate_uuidv7().hex[16:20] + "-" +
                                   generate_uuidv7().hex[20:32])

        # Add trace headers
        if not headers:
            headers = {}
        headers["Nats-Msg-Id"] = message_id

        payload = json.dumps(data, default=str).encode()

        try:
            ack = await self.js.publish(subject, payload, headers=headers)
            logger.debug(f"Published message to {subject}: {message_id}")
            return message_id
        except Exception as e:
            logger.error(f"Failed to publish to {subject}: {e}")
            raise

    async def subscribe(self, subject: str, durable: str,
                       callback: Callable, deliver_policy: str = "new"):
        """Subscribe to a subject with a durable consumer"""
        if not self.js:
            raise RuntimeError("Not connected to NATS")

        try:
            # Create durable consumer
            config = ConsumerConfig(
                durable_name=durable,
                deliver_policy=deliver_policy,
                ack_policy="explicit",
                ack_wait=30,  # 30 seconds
                max_deliver=3,
                replay_policy="instant"
            )

            consumer = await self.js.pull_subscribe(subject, durable, config=config)

            # Start message handler
            asyncio.create_task(self._message_handler(consumer, callback))

            self._subscriptions[f"{subject}:{durable}"] = consumer
            logger.info(f"Subscribed to {subject} with durable {durable}")

        except Exception as e:
            logger.error(f"Failed to subscribe to {subject}: {e}")
            raise

    async def _message_handler(self, consumer, callback: Callable):
        """Handle incoming messages"""
        while True:
            try:
                msgs = await consumer.fetch(batch=10, timeout=1.0)
                for msg in msgs:
                    try:
                        data = json.loads(msg.data.decode())

                        # Add message metadata
                        data["_nats_subject"] = msg.subject
                        data["_nats_reply"] = msg.reply

                        # Execute callback
                        await callback(data)

                        # Acknowledge message
                        await msg.ack()

                    except Exception as e:
                        logger.error(f"Error processing message: {e}")
                        await msg.nak()

            except Exception as e:
                if "timeout" not in str(e).lower():
                    logger.error(f"Error in message handler: {e}")
                await asyncio.sleep(0.1)

    async def request_reply(self, subject: str, data: Dict[str, Any],
                           timeout: float = 5.0) -> Dict[str, Any]:
        """Send a request and wait for reply"""
        if not self.nc:
            raise RuntimeError("Not connected to NATS")

        payload = json.dumps(data, default=str).encode()

        try:
            response = await self.nc.request(subject, payload, timeout=timeout)
            return json.loads(response.data.decode())
        except Exception as e:
            logger.error(f"Request to {subject} failed: {e}")
            raise

    async def get_stream_info(self, stream_name: str) -> Dict[str, Any]:
        """Get information about a stream"""
        if not self.js:
            raise RuntimeError("Not connected to NATS")

        try:
            info = await self.js.stream_info(stream_name)
            return {
                "name": info.config.name,
                "subjects": info.config.subjects,
                "messages": info.state.messages,
                "bytes": info.state.bytes,
                "consumers": info.state.consumer_count,
                "created": info.created.isoformat() if info.created else None
            }
        except Exception as e:
            logger.error(f"Failed to get stream info for {stream_name}: {e}")
            raise

    def health_check(self) -> bool:
        """Check if connection is healthy"""
        return self.nc is not None and not self.nc.is_closed


# Global instance
nats_client = NATSClient()