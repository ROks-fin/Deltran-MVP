import json
import logging
from typing import Any, Dict, List, Optional, Union

import redis.asyncio as redis
from redis.asyncio import Redis
from redis.exceptions import ConnectionError, TimeoutError

logger = logging.getLogger(__name__)


class RedisClient:
    def __init__(self, url: str = "redis://localhost:6379",
                 max_connections: int = 20, decode_responses: bool = True):
        self.url = url
        self.max_connections = max_connections
        self.decode_responses = decode_responses
        self.pool: Optional[redis.ConnectionPool] = None
        self.client: Optional[Redis] = None

    async def connect(self):
        """Create Redis connection pool"""
        try:
            self.pool = redis.ConnectionPool.from_url(
                self.url,
                max_connections=self.max_connections,
                decode_responses=self.decode_responses,
                socket_connect_timeout=5,
                socket_timeout=5,
                retry_on_timeout=True
            )
            self.client = Redis(connection_pool=self.pool)

            # Test connection
            await self.client.ping()
            logger.info(f"Connected to Redis at {self.url}")

        except Exception as e:
            logger.error(f"Failed to connect to Redis: {e}")
            raise

    async def disconnect(self):
        """Close Redis connection"""
        if self.client:
            await self.client.close()
            self.client = None
            self.pool = None
            logger.info("Disconnected from Redis")

    # Key-Value Operations
    async def set(self, key: str, value: Any, expire: Optional[int] = None) -> bool:
        """Set a key-value pair"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            serialized_value = json.dumps(value) if not isinstance(value, str) else value
            return await self.client.set(key, serialized_value, ex=expire)
        except Exception as e:
            logger.error(f"Failed to set {key}: {e}")
            raise

    async def get(self, key: str, deserialize: bool = True) -> Any:
        """Get a value by key"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            value = await self.client.get(key)
            if value is None:
                return None

            if deserialize:
                try:
                    return json.loads(value)
                except json.JSONDecodeError:
                    return value
            return value
        except Exception as e:
            logger.error(f"Failed to get {key}: {e}")
            raise

    async def delete(self, *keys: str) -> int:
        """Delete one or more keys"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            return await self.client.delete(*keys)
        except Exception as e:
            logger.error(f"Failed to delete keys {keys}: {e}")
            raise

    async def exists(self, key: str) -> bool:
        """Check if key exists"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            return await self.client.exists(key) > 0
        except Exception as e:
            logger.error(f"Failed to check existence of {key}: {e}")
            raise

    async def expire(self, key: str, seconds: int) -> bool:
        """Set expiration for a key"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            return await self.client.expire(key, seconds)
        except Exception as e:
            logger.error(f"Failed to set expiration for {key}: {e}")
            raise

    async def ttl(self, key: str) -> int:
        """Get time to live for a key"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            return await self.client.ttl(key)
        except Exception as e:
            logger.error(f"Failed to get TTL for {key}: {e}")
            raise

    # Hash Operations
    async def hset(self, key: str, mapping: Dict[str, Any]) -> int:
        """Set hash fields"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            # Serialize complex values
            serialized_mapping = {}
            for field, value in mapping.items():
                if isinstance(value, (dict, list)):
                    serialized_mapping[field] = json.dumps(value)
                else:
                    serialized_mapping[field] = str(value)

            return await self.client.hset(key, mapping=serialized_mapping)
        except Exception as e:
            logger.error(f"Failed to hset {key}: {e}")
            raise

    async def hget(self, key: str, field: str, deserialize: bool = True) -> Any:
        """Get hash field"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            value = await self.client.hget(key, field)
            if value is None:
                return None

            if deserialize:
                try:
                    return json.loads(value)
                except json.JSONDecodeError:
                    return value
            return value
        except Exception as e:
            logger.error(f"Failed to hget {key}.{field}: {e}")
            raise

    async def hgetall(self, key: str, deserialize: bool = True) -> Dict[str, Any]:
        """Get all hash fields"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            result = await self.client.hgetall(key)
            if not result:
                return {}

            if deserialize:
                deserialized = {}
                for field, value in result.items():
                    try:
                        deserialized[field] = json.loads(value)
                    except json.JSONDecodeError:
                        deserialized[field] = value
                return deserialized
            return result
        except Exception as e:
            logger.error(f"Failed to hgetall {key}: {e}")
            raise

    async def hdel(self, key: str, *fields: str) -> int:
        """Delete hash fields"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            return await self.client.hdel(key, *fields)
        except Exception as e:
            logger.error(f"Failed to hdel {key}.{fields}: {e}")
            raise

    # List Operations
    async def lpush(self, key: str, *values: Any) -> int:
        """Push values to the left of a list"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            serialized_values = [
                json.dumps(v) if not isinstance(v, str) else v for v in values
            ]
            return await self.client.lpush(key, *serialized_values)
        except Exception as e:
            logger.error(f"Failed to lpush to {key}: {e}")
            raise

    async def rpush(self, key: str, *values: Any) -> int:
        """Push values to the right of a list"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            serialized_values = [
                json.dumps(v) if not isinstance(v, str) else v for v in values
            ]
            return await self.client.rpush(key, *serialized_values)
        except Exception as e:
            logger.error(f"Failed to rpush to {key}: {e}")
            raise

    async def lpop(self, key: str, count: Optional[int] = None, deserialize: bool = True) -> Any:
        """Pop values from the left of a list"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            result = await self.client.lpop(key, count)
            if result is None:
                return None

            if isinstance(result, list):
                if deserialize:
                    return [json.loads(v) if v else None for v in result]
                return result
            else:
                if deserialize:
                    try:
                        return json.loads(result)
                    except json.JSONDecodeError:
                        return result
                return result
        except Exception as e:
            logger.error(f"Failed to lpop from {key}: {e}")
            raise

    async def lrange(self, key: str, start: int = 0, end: int = -1, deserialize: bool = True) -> List[Any]:
        """Get range of list elements"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            result = await self.client.lrange(key, start, end)
            if deserialize:
                return [json.loads(v) for v in result]
            return result
        except Exception as e:
            logger.error(f"Failed to lrange {key}: {e}")
            raise

    # Set Operations
    async def sadd(self, key: str, *values: Any) -> int:
        """Add values to a set"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            serialized_values = [
                json.dumps(v) if not isinstance(v, str) else v for v in values
            ]
            return await self.client.sadd(key, *serialized_values)
        except Exception as e:
            logger.error(f"Failed to sadd to {key}: {e}")
            raise

    async def sismember(self, key: str, value: Any) -> bool:
        """Check if value is in set"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            serialized_value = json.dumps(value) if not isinstance(value, str) else value
            return await self.client.sismember(key, serialized_value)
        except Exception as e:
            logger.error(f"Failed to check membership in {key}: {e}")
            raise

    async def smembers(self, key: str, deserialize: bool = True) -> set:
        """Get all set members"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            result = await self.client.smembers(key)
            if deserialize:
                return {json.loads(v) for v in result}
            return result
        except Exception as e:
            logger.error(f"Failed to get members of {key}: {e}")
            raise

    # Cache-specific methods
    async def cache_set(self, key: str, value: Any, expire: int = 300) -> bool:
        """Set a cached value with expiration"""
        return await self.set(f"cache:{key}", value, expire)

    async def cache_get(self, key: str) -> Any:
        """Get a cached value"""
        return await self.get(f"cache:{key}")

    async def cache_delete(self, key: str) -> int:
        """Delete a cached value"""
        return await self.delete(f"cache:{key}")

    # Idempotency methods
    async def idempotency_check(self, idempotency_key: str, response_data: Any = None, expire: int = 3600) -> Optional[Any]:
        """Check or set idempotency key"""
        redis_key = f"idempotency:{idempotency_key}"

        # Check if key exists
        existing = await self.get(redis_key)
        if existing is not None:
            return existing

        # Set new key if response_data provided
        if response_data is not None:
            await self.set(redis_key, response_data, expire)
            return response_data

        return None

    # Health and monitoring
    async def health_check(self) -> Dict[str, Any]:
        """Check Redis health"""
        if not self.client:
            return {"status": "disconnected"}

        try:
            # Test basic operations
            await self.client.ping()
            info = await self.client.info()

            return {
                "status": "healthy",
                "redis_version": info.get("redis_version"),
                "connected_clients": info.get("connected_clients"),
                "used_memory": info.get("used_memory_human"),
                "keyspace_hits": info.get("keyspace_hits"),
                "keyspace_misses": info.get("keyspace_misses"),
            }
        except Exception as e:
            return {
                "status": "unhealthy",
                "error": str(e)
            }

    async def get_stats(self) -> Dict[str, Any]:
        """Get Redis statistics"""
        if not self.client:
            raise RuntimeError("Not connected to Redis")

        try:
            info = await self.client.info()
            return {
                "memory": {
                    "used": info.get("used_memory"),
                    "used_human": info.get("used_memory_human"),
                    "peak": info.get("used_memory_peak"),
                    "peak_human": info.get("used_memory_peak_human"),
                },
                "stats": {
                    "total_connections": info.get("total_connections_received"),
                    "total_commands": info.get("total_commands_processed"),
                    "keyspace_hits": info.get("keyspace_hits"),
                    "keyspace_misses": info.get("keyspace_misses"),
                },
                "clients": {
                    "connected": info.get("connected_clients"),
                    "blocked": info.get("blocked_clients"),
                }
            }
        except Exception as e:
            logger.error(f"Failed to get Redis stats: {e}")
            raise


# Global instance
redis_client = RedisClient()