# DelTran Infrastructure Setup

This directory contains all infrastructure configuration for the DelTran payment system.

## Prerequisites

- Docker Desktop for Windows
- PostgreSQL client tools (psql, pg_dump)
- 8GB RAM minimum (16GB recommended)
- 20GB disk space

## Quick Start

### 1. Start Database Cluster

```bash
# Start PostgreSQL cluster (primary + 2 replicas + PgBouncer + pgAdmin)
docker-compose -f docker-compose.database.yml up -d

# Check status
docker-compose -f docker-compose.database.yml ps

# View logs
docker-compose -f docker-compose.database.yml logs -f postgres-primary
```

**Services Available:**
- PostgreSQL Primary: `localhost:5432`
- PostgreSQL Replica 1: `localhost:5433`
- PostgreSQL Replica 2: `localhost:5434`
- PgBouncer (Connection Pool): `localhost:6432`
- pgAdmin Web UI: `http://localhost:5050`

### 2. Start Redis Cluster

```bash
# Start Redis cluster (master + 2 replicas + 3 sentinels + web UI)
docker-compose -f docker-compose.cache.yml up -d

# Check status
docker-compose -f docker-compose.cache.yml ps

# Check replication status
docker exec deltran-redis-master redis-cli -a redis123 INFO replication
```

**Services Available:**
- Redis Master: `localhost:6379`
- Redis Replica 1: `localhost:6380`
- Redis Replica 2: `localhost:6381`
- Redis Sentinel 1: `localhost:26379`
- Redis Sentinel 2: `localhost:26380`
- Redis Sentinel 3: `localhost:26381`
- Redis Commander Web UI: `http://localhost:8081`

### 3. Run Database Migrations

```bash
# Windows
cd scripts
migrate.bat up

# Linux/Mac
cd scripts
chmod +x migrate.sh
./migrate.sh up
```

This will:
1. Create the `deltran` database if it doesn't exist
2. Create migration tracking table
3. Apply all SQL migrations from `sql/` directory
4. Create 10 core tables with indexes
5. Load seed data

### 4. Verify Setup

```bash
# Check migration status
migrate.bat status

# Connect to database via PgBouncer
psql -h localhost -p 6432 -U deltran_app -d deltran

# Check tables
\dt deltran.*

# Check replication status
psql -h localhost -p 5432 -U deltran_app -d deltran -c "SELECT * FROM pg_stat_replication;"
```

## Architecture

### PostgreSQL Cluster

```
┌─────────────────────────────────────────┐
│         Application Layer               │
└──────────────┬──────────────────────────┘
               │
        ┌──────▼──────┐
        │  PgBouncer  │ (Connection Pool)
        │   :6432     │
        └──────┬──────┘
               │
        ┌──────▼───────────┐
        │   PostgreSQL     │
        │    Primary       │ (Write)
        │     :5432        │
        └──────┬───────────┘
               │
         ┌─────┴─────┐
         │           │
    ┌────▼───┐   ┌───▼────┐
    │Replica1│   │Replica2│ (Read-only)
    │  :5433 │   │  :5434 │
    └────────┘   └────────┘
```

**Features:**
- **Streaming Replication**: Real-time replication to 2 replicas
- **Connection Pooling**: PgBouncer with transaction mode
- **High Availability**: Automatic failover with replicas
- **Backup**: Automated backups with retention policy

**Configuration:**
- Max Connections: 200 (primary), 1000 (pgbouncer)
- Shared Buffers: 256MB
- Write-Ahead Log: Replica level for streaming
- Compression: Native PostgreSQL compression

### Redis Cluster

```
┌─────────────────────────────────────────┐
│         Application Layer               │
└──────────┬──────────────┬───────────────┘
           │              │
    ┌──────▼──────┐  ┌────▼──────────────┐
    │   Redis     │  │  Redis Sentinel   │
    │   Master    │  │   (3 instances)   │
    │   :6379     │  │  Monitor + Failover│
    └──────┬──────┘  └───────────────────┘
           │
     ┌─────┴─────┐
     │           │
┌────▼───┐   ┌───▼────┐
│Replica1│   │Replica2│
│ :6380  │   │ :6381  │
└────────┘   └────────┘
```

**Features:**
- **Redis Sentinel**: Automatic failover (quorum: 2/3)
- **Replication**: Master-replica replication
- **Persistence**: RDB snapshots + AOF (append-only file)
- **Eviction**: LRU (Least Recently Used)

**Use Cases:**
- Session storage (JWT refresh tokens)
- Rate limiting counters
- Idempotency key tracking
- Circuit breaker state
- Cache layer for hot data

### RocksDB (Ledger Storage)

```
Application
    │
    ├──► RocksDB Instance
         │
         ├──► CF: events (append-only)
         ├──► CF: blocks (finalized)
         ├──► CF: state (payment states)
         ├──► CF: indices (secondary indices)
         ├──► CF: merkle (merkle tree)
         └──► CF: snapshots (snapshots)
```

**Features:**
- **Immutability**: Append-only event log
- **Column Families**: Separate storage for different data types
- **Compression**: Zstd for events/blocks, LZ4 for hot data
- **Compaction**: Universal compaction for write-heavy workload
- **Bloom Filters**: Fast negative lookups on indices

**Configuration:**
- Write Buffer: 64MB × 4 buffers
- Block Cache: 256MB
- Compression: Zstd (best ratio), LZ4 (fast)
- Backup: Point-in-time backups every hour

## Database Schema

### Core Tables (10)

1. **users** - User accounts with authentication
2. **sessions** - JWT refresh token sessions
3. **banks** - Participating financial institutions
4. **bank_accounts** - Settlement accounts
5. **payments** - Payment instructions
6. **transaction_log** - Immutable transaction history
7. **settlement_batches** - Batch processing with netting
8. **compliance_checks** - AML/KYC/Sanctions screening
9. **rate_limits** - Rate limiting and controls
10. **audit_log** - Audit trail (partitioned by month)

### Custom Types

- `user_role`: admin, operator, auditor, viewer
- `payment_status`: pending, processing, settled, failed, cancelled
- `settlement_status`: draft, ready, submitted, settled, failed, cancelled
- `compliance_status`: pending, pass, fail, review
- `currency_code`: USD, EUR, GBP, JPY, CHF, CAD, AUD, CNY, INR, BRL
- `action_type`: freeze, unfreeze, throttle, block

### Key Features

- **Partitioning**: audit_log partitioned by month (auto-created)
- **Indexes**: Comprehensive indexes for all query patterns
- **Constraints**: Foreign keys, checks, and unique constraints
- **Triggers**: auto-updated `updated_at` timestamps
- **Views**: Active payments, bank balances, daily volume

## Environment Variables

Copy `.env.example` to `.env` and update values:

```bash
cp .env.example .env
```

**Critical Variables:**
- `POSTGRES_PASSWORD`: Database password (change in production!)
- `REDIS_PASSWORD`: Redis password (change in production!)
- `JWT_SECRET`: JWT signing key (strong random value!)
- `SESSION_SECRET`: Session encryption key

## Migration Management

### Commands

```bash
# Apply all pending migrations
migrate.bat up

# Check migration status
migrate.bat status

# Create database backup
migrate.bat backup

# Restore from backup
migrate.bat restore ./data/backups/deltran_backup_20251012.sql.gz
```

### Adding New Migrations

1. Create SQL file: `sql/002_feature_name.sql`
2. Version format: `NNN_description.sql` (e.g., `002_add_fx_rates_table.sql`)
3. Run migration: `migrate.bat up`

**Migration Template:**
```sql
-- Migration: 002_add_fx_rates_table.sql
-- Description: Add FX rates table for currency conversion

BEGIN;

CREATE TABLE deltran.fx_rates (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    from_currency currency_code NOT NULL,
    to_currency currency_code NOT NULL,
    rate DECIMAL(20,10) NOT NULL,
    timestamp TIMESTAMPTZ DEFAULT NOW(),
    source VARCHAR(50),
    CONSTRAINT uq_fx_rate UNIQUE (from_currency, to_currency, timestamp)
);

CREATE INDEX idx_fx_rates_currencies ON deltran.fx_rates(from_currency, to_currency);
CREATE INDEX idx_fx_rates_timestamp ON deltran.fx_rates(timestamp DESC);

COMMIT;
```

## Backup & Recovery

### Automated Backups

Backups run automatically every hour (configurable in `.env`):
- Location: `./data/backups/`
- Retention: 30 days (configurable)
- Format: SQL dump (compressed with gzip)

### Manual Backup

```bash
# Full backup
migrate.bat backup

# Backup specific database
pg_dump -h localhost -p 5432 -U deltran_app -d deltran -F c -f backup.dump
```

### Recovery

```bash
# Restore from latest backup
migrate.bat restore ./data/backups/deltran_backup_latest.sql.gz

# Restore specific backup
psql -h localhost -p 5432 -U deltran_app -d deltran < backup.sql
```

## Monitoring

### Database Health

```sql
-- Connection count
SELECT count(*) FROM pg_stat_activity WHERE datname = 'deltran';

-- Replication lag
SELECT
    application_name,
    state,
    sync_state,
    pg_size_pretty(pg_wal_lsn_diff(pg_current_wal_lsn(), replay_lsn)) as lag
FROM pg_stat_replication;

-- Table sizes
SELECT
    schemaname,
    tablename,
    pg_size_pretty(pg_total_relation_size(schemaname||'.'||tablename)) AS size
FROM pg_tables
WHERE schemaname = 'deltran'
ORDER BY pg_total_relation_size(schemaname||'.'||tablename) DESC;

-- Index usage
SELECT
    schemaname,
    tablename,
    indexname,
    idx_scan,
    pg_size_pretty(pg_relation_size(indexrelid)) as size
FROM pg_stat_user_indexes
WHERE schemaname = 'deltran'
ORDER BY idx_scan DESC;
```

### Redis Health

```bash
# Replication status
docker exec deltran-redis-master redis-cli -a redis123 INFO replication

# Memory usage
docker exec deltran-redis-master redis-cli -a redis123 INFO memory

# Connected clients
docker exec deltran-redis-master redis-cli -a redis123 CLIENT LIST
```

### RocksDB Health

```bash
# Check stats via application API
curl http://localhost:8080/api/v1/storage/stats

# Disk usage
du -sh ./data/ledger/
```

## Troubleshooting

### PostgreSQL Won't Start

```bash
# Check logs
docker logs deltran-postgres-primary

# Common issues:
# - Port 5432 already in use
# - Insufficient disk space
# - Corrupted data directory

# Fix: Stop existing PostgreSQL
net stop postgresql

# Or use different port
# Edit docker-compose.database.yml: ports: "5433:5432"
```

### Redis Won't Start

```bash
# Check logs
docker logs deltran-redis-master

# Common issues:
# - Port 6379 already in use
# - Memory limits too low
# - AOF/RDB corruption

# Fix: Clear Redis data
docker-compose -f docker-compose.cache.yml down -v
```

### Replication Issues

```sql
-- Check replication slots
SELECT * FROM pg_replication_slots;

-- Check wal senders
SELECT * FROM pg_stat_replication;

-- Drop and recreate slot
SELECT pg_drop_replication_slot('replication_slot_1');
SELECT pg_create_physical_replication_slot('replication_slot_1');
```

### Migration Failures

```bash
# Check migration log
cat ../logs/migration_*.log

# Check which migrations succeeded
migrate.bat status

# Manually fix database, then mark migration as successful
psql -h localhost -p 6432 -U deltran_app -d deltran
UPDATE public.schema_migrations SET success = true WHERE version = '001';
```

## Performance Tuning

### PostgreSQL

```sql
-- Analyze query performance
EXPLAIN ANALYZE SELECT * FROM deltran.payments WHERE status = 'pending';

-- Update statistics
ANALYZE deltran.payments;

-- Reindex
REINDEX TABLE deltran.payments;

-- Vacuum
VACUUM ANALYZE deltran.payments;
```

### Redis

```bash
# Monitor commands in real-time
docker exec deltran-redis-master redis-cli -a redis123 MONITOR

# Check slow log
docker exec deltran-redis-master redis-cli -a redis123 SLOWLOG GET 10

# Memory optimization
docker exec deltran-redis-master redis-cli -a redis123 CONFIG SET maxmemory-policy allkeys-lru
```

### RocksDB

```toml
# Edit ledger-core/rocksdb.toml

[rocksdb]
# Increase write buffer for higher write throughput
write_buffer_size_mb = 128
max_write_buffer_number = 6

# Increase cache for better read performance
[rocksdb.cache]
block_cache_size_mb = 512
row_cache_size_mb = 256
```

## Security Checklist

- [ ] Change default passwords in `.env`
- [ ] Enable SSL/TLS for PostgreSQL
- [ ] Enable SSL/TLS for Redis
- [ ] Configure firewall rules (only allow app network)
- [ ] Enable audit logging
- [ ] Set up automated backups
- [ ] Configure backup encryption
- [ ] Implement backup rotation
- [ ] Enable pgBouncer connection limits
- [ ] Configure Redis maxmemory and eviction
- [ ] Enable 2FA for admin users
- [ ] Rotate JWT secrets regularly
- [ ] Monitor failed login attempts
- [ ] Set up alerting for anomalies

## Ports Reference

| Service | Port | Protocol | Purpose |
|---------|------|----------|---------|
| PostgreSQL Primary | 5432 | TCP | Write operations |
| PostgreSQL Replica 1 | 5433 | TCP | Read operations |
| PostgreSQL Replica 2 | 5434 | TCP | Read operations |
| PgBouncer | 6432 | TCP | Connection pooling |
| pgAdmin | 5050 | HTTP | Database admin UI |
| Redis Master | 6379 | TCP | Cache/session store |
| Redis Replica 1 | 6380 | TCP | Cache replica |
| Redis Replica 2 | 6381 | TCP | Cache replica |
| Redis Sentinel 1 | 26379 | TCP | Failover manager |
| Redis Sentinel 2 | 26380 | TCP | Failover manager |
| Redis Sentinel 3 | 26381 | TCP | Failover manager |
| Redis Commander | 8081 | HTTP | Redis admin UI |

## Next Steps

After infrastructure is running:

1. **Week 3-4**: Implement JWT Authentication & RBAC
2. **Week 5-6**: Add SWIFT & RTGS Integration
3. **Week 7-8**: Implement Monitoring (Prometheus + Grafana)
4. **Week 9-10**: Add High Availability & Disaster Recovery

See [MASTER_IMPLEMENTATION_PLAN.md](../MASTER_IMPLEMENTATION_PLAN.md) for full roadmap.

## Support

For issues or questions:
- Check logs in `./logs/`
- Review [TROUBLESHOOTING.md](./TROUBLESHOOTING.md)
- Contact DevOps team
