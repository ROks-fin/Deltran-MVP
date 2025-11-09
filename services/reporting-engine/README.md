# DelTran Reporting Engine

Enterprise-grade reporting system providing real-time analytics, regulatory compliance reports, and one-click audit trails with Excel/CSV export for Big 4 audits.

## Features

### 🎯 Core Capabilities
- **Excel Reports** with Big 4 audit formatting (PwC/Deloitte/EY/KPMG standards)
- **CSV Generation** with streaming for 1M+ rows without memory issues
- **Scheduled Reports** (daily, weekly, monthly, quarterly) with cron
- **S3 Storage** integration with pre-signed URLs
- **Real-time Metrics** with Redis caching
- **Materialized Views** for high-performance aggregations

### 📊 Report Types
1. **AML Compliance Reports**
   - Transaction analysis
   - Risk indicators
   - Suspicious activities
   - Sanctions screening

2. **Settlement Reports**
   - Gross positions
   - Netting results
   - Net obligations
   - Efficiency metrics

3. **Reconciliation Reports**
   - Matched transactions
   - Discrepancies
   - Unmatched items

4. **Operational Reports**
   - System metrics
   - Performance analysis
   - Error tracking

## Quick Start

### Prerequisites
- Go 1.21+
- PostgreSQL 16 with TimescaleDB
- Redis 7.2
- S3-compatible storage (MinIO for development)

### Installation

```bash
# Clone repository
cd services/reporting-engine

# Install dependencies
go mod download

# Copy and configure
cp config.yaml.example config.yaml
# Edit config.yaml with your settings

# Run migrations
psql -h localhost -U deltran -d deltran -f ../../infrastructure/sql/008_reporting_engine.sql

# Build
make build

# Run
make run
```

### Using Docker

```bash
# Build image
make docker-build

# Run container
make docker-run
```

## Configuration

### Environment Variables

```bash
# Database
DB_PASSWORD=your_password

# Redis
REDIS_PASSWORD=your_password

# S3 Storage
S3_ENDPOINT=http://localhost:9000
AWS_ACCESS_KEY_ID=minioadmin
AWS_SECRET_ACCESS_KEY=minioadmin

# Optional
SERVICE_PORT=8087
CONFIG_PATH=config.yaml
```

### config.yaml

See `config.yaml` for full configuration options including:
- Server settings (port, timeouts)
- Database connection
- Redis caching
- S3 storage
- Report scheduler
- Report limits

## API Documentation

### Generate Report

```bash
POST /api/v1/reports/generate
Content-Type: application/json

{
  "type": "aml",
  "format": "excel",
  "period_start": "2025-01-01T00:00:00Z",
  "period_end": "2025-01-31T23:59:59Z",
  "requested_by": "user_id"
}
```

### Get Report

```bash
GET /api/v1/reports/{id}
```

### Download Report

```bash
GET /api/v1/reports/{id}/download
```

Returns a pre-signed URL valid for 15 minutes.

### List Reports

```bash
GET /api/v1/reports?type=aml&limit=50&offset=0
```

### Predefined Reports

```bash
# Generate daily AML report
POST /api/v1/reports/aml/daily

# Generate daily settlement report
POST /api/v1/reports/settlement/daily
```

### Metrics

```bash
# Get live metrics
GET /api/v1/metrics/live?date=2025-01-07
```

### Health Check

```bash
GET /health
```

## Report Scheduling

Reports are automatically generated on the following schedule (UTC):

- **Daily Reports**: 00:30
- **Weekly Reports**: Monday 01:00
- **Monthly Reports**: 1st day 02:00
- **Quarterly Reports**: 1st day of quarter 03:00

Metrics are refreshed every 5 minutes, and materialized views are refreshed every hour.

## Development

### Running Tests

```bash
# Run all tests
make test

# Run with coverage
make test-coverage

# Run benchmarks
make benchmark

# Performance test (1M rows)
make performance-test
```

### Building

```bash
# Build binary
make build

# Build Docker image
make docker-build
```

## Architecture

### Components

1. **Excel Generator** (`internal/generators/excel.go`)
   - Multi-sheet reports
   - Big 4 audit formatting
   - Charts and visualizations
   - Digital signatures

2. **CSV Generator** (`internal/generators/csv.go`)
   - Streaming generation
   - Memory-efficient batch processing
   - Progress tracking

3. **Report Scheduler** (`internal/scheduler/scheduler.go`)
   - Cron-based scheduling
   - Concurrent generation limiting
   - Automatic distribution

4. **Storage Layer** (`internal/storage/`)
   - PostgreSQL for metadata
   - S3 for report files
   - Redis for caching

5. **Report Generators** (`internal/reports/`)
   - AML report generator
   - Settlement report generator
   - Reconciliation report generator
   - Operational report generator

### Database Schema

See `infrastructure/sql/008_reporting_engine.sql` for:
- Report metadata tables
- Scheduled report configuration
- Access audit logs
- Materialized views for performance

## Performance

### Targets
- Excel generation: < 10 seconds for daily reports
- CSV generation: < 30 seconds for 1M rows
- Concurrent reports: Up to 5 simultaneous generations
- Cache hit ratio: > 80%

### Optimization
- Streaming CSV generation for memory efficiency
- Batch processing (1000 rows per batch)
- Materialized views for aggregations
- Redis caching for frequently accessed data
- Connection pooling (max 20 connections)

## Security

### Audit Trail
- All report access logged
- IP address and user agent tracking
- Timestamped access records

### Digital Signatures
- Audit trail sheet with digital signature
- Protected sheet with password
- Generator information and timestamps

### Access Control
- Pre-signed URLs with 15-minute expiry
- S3 bucket permissions
- Database row-level tracking

## Troubleshooting

### Common Issues

1. **Database Connection Failed**
   - Check `DB_PASSWORD` environment variable
   - Verify PostgreSQL is running
   - Check `config.yaml` database settings

2. **S3 Upload Failed**
   - Check S3 credentials
   - Verify bucket exists
   - Check network connectivity

3. **Report Generation Timeout**
   - Increase `reports.timeout` in config
   - Check database query performance
   - Review materialized view refresh

### Logs

Logs are written to stdout in JSON format (production) or console format (development).

```bash
# View logs
docker logs deltran-reporting-engine

# Follow logs
docker logs -f deltran-reporting-engine
```

## Contributing

1. Fork the repository
2. Create a feature branch
3. Make your changes
4. Run tests: `make test`
5. Submit a pull request

## License

Copyright (c) 2025 DelTran. All rights reserved.

## Support

For issues and questions:
- Create an issue on GitHub
- Contact: support@deltran.com
- Documentation: https://docs.deltran.com/reporting-engine
