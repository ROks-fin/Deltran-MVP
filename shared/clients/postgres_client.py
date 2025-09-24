import asyncio
import logging
from contextlib import asynccontextmanager
from typing import Any, Dict, List, Optional, Tuple
from urllib.parse import urlparse

import asyncpg
from asyncpg import Pool, Connection

logger = logging.getLogger(__name__)


class PostgresClient:
    def __init__(self, database_url: str, min_connections: int = 5, max_connections: int = 20):
        self.database_url = database_url
        self.min_connections = min_connections
        self.max_connections = max_connections
        self.pool: Optional[Pool] = None

    async def connect(self):
        """Create connection pool"""
        try:
            self.pool = await asyncpg.create_pool(
                self.database_url,
                min_size=self.min_connections,
                max_size=self.max_connections,
                command_timeout=60,
                server_settings={
                    'jit': 'off',
                    'log_statement': 'none'
                }
            )
            logger.info(f"Connected to PostgreSQL with pool size {self.min_connections}-{self.max_connections}")
        except Exception as e:
            logger.error(f"Failed to connect to PostgreSQL: {e}")
            raise

    async def disconnect(self):
        """Close connection pool"""
        if self.pool:
            await self.pool.close()
            self.pool = None
            logger.info("Disconnected from PostgreSQL")

    @asynccontextmanager
    async def connection(self):
        """Get a connection from the pool"""
        if not self.pool:
            raise RuntimeError("Not connected to PostgreSQL")

        conn = await self.pool.acquire()
        try:
            yield conn
        finally:
            await self.pool.release(conn)

    @asynccontextmanager
    async def transaction(self):
        """Get a transactional connection"""
        async with self.connection() as conn:
            async with conn.transaction():
                yield conn

    async def execute(self, query: str, *args) -> str:
        """Execute a query that doesn't return results"""
        async with self.connection() as conn:
            return await conn.execute(query, *args)

    async def fetch(self, query: str, *args) -> List[asyncpg.Record]:
        """Execute a query and return all results"""
        async with self.connection() as conn:
            return await conn.fetch(query, *args)

    async def fetchrow(self, query: str, *args) -> Optional[asyncpg.Record]:
        """Execute a query and return first result"""
        async with self.connection() as conn:
            return await conn.fetchrow(query, *args)

    async def fetchval(self, query: str, *args) -> Any:
        """Execute a query and return single value"""
        async with self.connection() as conn:
            return await conn.fetchval(query, *args)

    async def executemany(self, query: str, args_list: List[Tuple]) -> None:
        """Execute a query multiple times with different arguments"""
        async with self.connection() as conn:
            await conn.executemany(query, args_list)

    async def copy_to_table(self, table_name: str, records: List[Dict[str, Any]],
                           columns: Optional[List[str]] = None) -> int:
        """Bulk copy records to a table"""
        if not records:
            return 0

        if columns is None:
            columns = list(records[0].keys())

        async with self.connection() as conn:
            # Convert records to tuples in column order
            values = [[record.get(col) for col in columns] for record in records]

            result = await conn.copy_records_to_table(
                table_name,
                records=values,
                columns=columns
            )
            return int(result.split()[-1])

    async def insert_returning(self, table: str, data: Dict[str, Any],
                              returning: str = "*") -> Optional[asyncpg.Record]:
        """Insert a record and return specified columns"""
        columns = list(data.keys())
        placeholders = [f"${i+1}" for i in range(len(columns))]
        values = list(data.values())

        query = f"""
            INSERT INTO {table} ({', '.join(columns)})
            VALUES ({', '.join(placeholders)})
            RETURNING {returning}
        """

        return await self.fetchrow(query, *values)

    async def upsert(self, table: str, data: Dict[str, Any], conflict_columns: List[str],
                    update_columns: Optional[List[str]] = None) -> Optional[asyncpg.Record]:
        """Insert or update a record"""
        columns = list(data.keys())
        placeholders = [f"${i+1}" for i in range(len(columns))]
        values = list(data.values())

        if update_columns is None:
            update_columns = [col for col in columns if col not in conflict_columns]

        update_clauses = [f"{col} = EXCLUDED.{col}" for col in update_columns]

        query = f"""
            INSERT INTO {table} ({', '.join(columns)})
            VALUES ({', '.join(placeholders)})
            ON CONFLICT ({', '.join(conflict_columns)})
            DO UPDATE SET {', '.join(update_clauses)}
            RETURNING *
        """

        return await self.fetchrow(query, *values)

    async def bulk_upsert(self, table: str, records: List[Dict[str, Any]],
                         conflict_columns: List[str], update_columns: Optional[List[str]] = None):
        """Bulk upsert records"""
        if not records:
            return

        columns = list(records[0].keys())

        if update_columns is None:
            update_columns = [col for col in columns if col not in conflict_columns]

        # Create temporary table
        temp_table = f"temp_{table}_{asyncio.current_task().get_name() or 'unknown'}"
        temp_table = temp_table.replace('-', '_')[:63]  # PostgreSQL identifier length limit

        async with self.transaction() as conn:
            # Create temporary table
            create_temp_query = f"""
                CREATE TEMP TABLE {temp_table} AS
                SELECT * FROM {table} WHERE FALSE
            """
            await conn.execute(create_temp_query)

            # Bulk insert into temporary table
            await conn.copy_records_to_table(
                temp_table,
                records=[[record.get(col) for col in columns] for record in records],
                columns=columns
            )

            # Upsert from temporary table
            update_clauses = [f"{col} = {temp_table}.{col}" for col in update_columns]

            upsert_query = f"""
                INSERT INTO {table} ({', '.join(columns)})
                SELECT {', '.join(columns)} FROM {temp_table}
                ON CONFLICT ({', '.join(conflict_columns)})
                DO UPDATE SET {', '.join(update_clauses)}
            """
            await conn.execute(upsert_query)

    async def health_check(self) -> Dict[str, Any]:
        """Check database health"""
        if not self.pool:
            return {"status": "disconnected"}

        try:
            async with self.connection() as conn:
                version = await conn.fetchval("SELECT version()")
                pool_stats = {
                    "size": self.pool.get_size(),
                    "min_size": self.pool.get_min_size(),
                    "max_size": self.pool.get_max_size(),
                    "idle_size": self.pool.get_idle_size(),
                }

                return {
                    "status": "healthy",
                    "version": version,
                    "pool": pool_stats
                }
        except Exception as e:
            return {
                "status": "unhealthy",
                "error": str(e)
            }

    async def get_table_stats(self, table_name: str) -> Dict[str, Any]:
        """Get statistics for a table"""
        query = """
            SELECT
                schemaname,
                tablename,
                n_tup_ins as inserts,
                n_tup_upd as updates,
                n_tup_del as deletes,
                n_live_tup as live_tuples,
                n_dead_tup as dead_tuples,
                last_vacuum,
                last_autovacuum,
                last_analyze,
                last_autoanalyze
            FROM pg_stat_user_tables
            WHERE tablename = $1
        """

        stats = await self.fetchrow(query, table_name)
        if not stats:
            return {"error": f"Table {table_name} not found"}

        return dict(stats)

    async def execute_migration(self, migration_sql: str, version: str):
        """Execute a database migration"""
        async with self.transaction() as conn:
            # Execute migration
            await conn.execute(migration_sql)

            # Record migration
            await conn.execute("""
                INSERT INTO schema_migrations (version, applied_at)
                VALUES ($1, NOW())
                ON CONFLICT (version) DO NOTHING
            """, version)

            logger.info(f"Applied migration {version}")

    async def get_applied_migrations(self) -> List[str]:
        """Get list of applied migrations"""
        try:
            records = await self.fetch("""
                SELECT version FROM schema_migrations
                ORDER BY applied_at
            """)
            return [record["version"] for record in records]
        except:
            # Table doesn't exist yet
            return []


# Global instance
postgres_client = PostgresClient(database_url="postgresql://localhost/deltran")