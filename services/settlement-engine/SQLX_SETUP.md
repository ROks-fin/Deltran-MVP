# SQLx Offline Mode Setup

## Current Status

The settlement-engine uses SQLx's compile-time checked queries (`query!` and `query_as!` macros). These require database connectivity at compile time OR cached query metadata.

## Why Compilation Fails

Currently, `SQLX_OFFLINE=true` is set in `build.rs`, but there's no cached query metadata in `.sqlx/query-metadata.json`. This is intentional for the MVP development phase.

## To Enable Compilation with Database

Once the database schema is set up:

1. **Set up the database URL**:
   ```bash
   export DATABASE_URL="postgresql://username:password@localhost/settlement_db"
   ```

2. **Run the SQL migrations** (from the infrastructure setup):
   ```bash
   psql $DATABASE_URL < ../../infrastructure/sql/001_core_schema.sql
   psql $DATABASE_URL < ../../infrastructure/sql/002_advanced_settlement.sql
   ```

3. **Generate query metadata**:
   ```bash
   cargo sqlx prepare
   ```

   This creates `.sqlx/query-metadata.json` with all query type information.

4. **Commit the metadata**:
   ```bash
   git add .sqlx/
   ```

## Alternative: Dynamic Queries

If you prefer to compile without database access, you can replace the compile-time checked macros with dynamic queries:

- Replace `sqlx::query!(...)` with `sqlx::query(...)`
- Replace `sqlx::query_as!(Type, ...)` with `sqlx::query_as::<_, Type>(...)`

This loses compile-time SQL validation but allows compilation without database connectivity.

## CI/CD

For CI/CD pipelines, ensure `.sqlx/query-metadata.json` is committed to the repository so builds can succeed in offline mode.
