@echo off
REM DelTran Infrastructure Setup Verification Script
REM Checks configuration files and prerequisites

setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set INFRA_DIR=%SCRIPT_DIR%..
set PROJECT_ROOT=%SCRIPT_DIR%..\..

echo ================================================
echo DelTran Infrastructure Setup Verification
echo ================================================
echo.

set PASS_COUNT=0
set FAIL_COUNT=0
set WARN_COUNT=0

REM ============================================
REM Check Prerequisites
REM ============================================

echo [CHECK] Prerequisites
echo.

REM Check Docker
where docker >nul 2>nul
if %ERRORLEVEL% equ 0 (
    for /f "tokens=3" %%v in ('docker --version') do set DOCKER_VERSION=%%v
    echo [PASS] Docker installed: !DOCKER_VERSION!
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] Docker not found - Install Docker Desktop
    set /a FAIL_COUNT+=1
)

REM Check Docker Compose
where docker-compose >nul 2>nul
if %ERRORLEVEL% equ 0 (
    echo [PASS] Docker Compose installed
    set /a PASS_COUNT+=1
) else (
    echo [WARN] Docker Compose not found - Using 'docker compose' subcommand
    set /a WARN_COUNT+=1
)

REM Check PostgreSQL client
where psql >nul 2>nul
if %ERRORLEVEL% equ 0 (
    for /f "tokens=3" %%v in ('psql --version') do set PSQL_VERSION=%%v
    echo [PASS] PostgreSQL client installed: !PSQL_VERSION!
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] psql not found - Install PostgreSQL client tools
    set /a FAIL_COUNT+=1
)

REM Check pg_dump
where pg_dump >nul 2>nul
if %ERRORLEVEL% equ 0 (
    echo [PASS] pg_dump installed (backup tool)
    set /a PASS_COUNT+=1
) else (
    echo [WARN] pg_dump not found - Backups will not work
    set /a WARN_COUNT+=1
)

echo.

REM ============================================
REM Check Configuration Files
REM ============================================

echo [CHECK] Configuration Files
echo.

REM Check docker-compose.database.yml
if exist "%INFRA_DIR%\docker-compose.database.yml" (
    echo [PASS] docker-compose.database.yml exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] docker-compose.database.yml not found
    set /a FAIL_COUNT+=1
)

REM Check docker-compose.cache.yml
if exist "%INFRA_DIR%\docker-compose.cache.yml" (
    echo [PASS] docker-compose.cache.yml exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] docker-compose.cache.yml not found
    set /a FAIL_COUNT+=1
)

REM Check SQL schema
if exist "%INFRA_DIR%\sql\001_core_schema.sql" (
    echo [PASS] 001_core_schema.sql exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] 001_core_schema.sql not found
    set /a FAIL_COUNT+=1
)

REM Check migration scripts
if exist "%INFRA_DIR%\scripts\migrate.bat" (
    echo [PASS] migrate.bat exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] migrate.bat not found
    set /a FAIL_COUNT+=1
)

REM Check .env.example
if exist "%INFRA_DIR%\.env.example" (
    echo [PASS] .env.example exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] .env.example not found
    set /a FAIL_COUNT+=1
)

REM Check if .env exists
if exist "%INFRA_DIR%\.env" (
    echo [PASS] .env exists (configured)
    set /a PASS_COUNT+=1
) else (
    echo [WARN] .env not found - Copy from .env.example
    set /a WARN_COUNT+=1
)

echo.

REM ============================================
REM Check RocksDB Configuration
REM ============================================

echo [CHECK] RocksDB Configuration
echo.

if exist "%PROJECT_ROOT%\ledger-core\rocksdb.toml" (
    echo [PASS] rocksdb.toml exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] rocksdb.toml not found
    set /a FAIL_COUNT+=1
)

if exist "%PROJECT_ROOT%\ledger-core\src\storage.rs" (
    echo [PASS] storage.rs exists (RocksDB implementation)
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] storage.rs not found
    set /a FAIL_COUNT+=1
)

echo.

REM ============================================
REM Check Directories
REM ============================================

echo [CHECK] Required Directories
echo.

if not exist "%PROJECT_ROOT%\data" mkdir "%PROJECT_ROOT%\data"
if exist "%PROJECT_ROOT%\data" (
    echo [PASS] data/ directory exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] data/ directory not found
    set /a FAIL_COUNT+=1
)

if not exist "%PROJECT_ROOT%\data\backups" mkdir "%PROJECT_ROOT%\data\backups"
if exist "%PROJECT_ROOT%\data\backups" (
    echo [PASS] data/backups/ directory exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] data/backups/ directory not found
    set /a FAIL_COUNT+=1
)

if not exist "%PROJECT_ROOT%\logs" mkdir "%PROJECT_ROOT%\logs"
if exist "%PROJECT_ROOT%\logs" (
    echo [PASS] logs/ directory exists
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] logs/ directory not found
    set /a FAIL_COUNT+=1
)

echo.

REM ============================================
REM Check Ports Availability
REM ============================================

echo [CHECK] Port Availability
echo.

REM PostgreSQL primary
netstat -an | findstr ":5432" | findstr "LISTENING" >nul
if %ERRORLEVEL% equ 0 (
    echo [WARN] Port 5432 is in use (PostgreSQL primary)
    set /a WARN_COUNT+=1
) else (
    echo [PASS] Port 5432 is available
    set /a PASS_COUNT+=1
)

REM Redis
netstat -an | findstr ":6379" | findstr "LISTENING" >nul
if %ERRORLEVEL% equ 0 (
    echo [WARN] Port 6379 is in use (Redis)
    set /a WARN_COUNT+=1
) else (
    echo [PASS] Port 6379 is available
    set /a PASS_COUNT+=1
)

REM PgBouncer
netstat -an | findstr ":6432" | findstr "LISTENING" >nul
if %ERRORLEVEL% equ 0 (
    echo [WARN] Port 6432 is in use (PgBouncer)
    set /a WARN_COUNT+=1
) else (
    echo [PASS] Port 6432 is available
    set /a PASS_COUNT+=1
)

echo.

REM ============================================
REM Validate SQL Schema
REM ============================================

echo [CHECK] SQL Schema Validation
echo.

REM Count tables in schema
findstr /C:"CREATE TABLE" "%INFRA_DIR%\sql\001_core_schema.sql" >nul
if %ERRORLEVEL% equ 0 (
    for /f %%c in ('findstr /C:"CREATE TABLE" "%INFRA_DIR%\sql\001_core_schema.sql" ^| find /c /v ""') do set TABLE_COUNT=%%c
    echo [PASS] Found !TABLE_COUNT! table definitions
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] No CREATE TABLE statements found
    set /a FAIL_COUNT+=1
)

REM Check for custom types
findstr /C:"CREATE TYPE" "%INFRA_DIR%\sql\001_core_schema.sql" >nul
if %ERRORLEVEL% equ 0 (
    echo [PASS] Custom types defined
    set /a PASS_COUNT+=1
) else (
    echo [FAIL] No custom types found
    set /a FAIL_COUNT+=1
)

REM Check for indexes
findstr /C:"CREATE INDEX" "%INFRA_DIR%\sql\001_core_schema.sql" >nul
if %ERRORLEVEL% equ 0 (
    for /f %%c in ('findstr /C:"CREATE INDEX" "%INFRA_DIR%\sql\001_core_schema.sql" ^| find /c /v ""') do set INDEX_COUNT=%%c
    echo [PASS] Found !INDEX_COUNT! index definitions
    set /a PASS_COUNT+=1
) else (
    echo [WARN] No indexes defined
    set /a WARN_COUNT+=1
)

echo.

REM ============================================
REM Summary
REM ============================================

echo ================================================
echo Verification Summary
echo ================================================
echo.
echo [PASS] Checks passed: %PASS_COUNT%
echo [WARN] Warnings:      %WARN_COUNT%
echo [FAIL] Checks failed: %FAIL_COUNT%
echo.

if %FAIL_COUNT% equ 0 (
    if %WARN_COUNT% equ 0 (
        echo ================================================
        echo [SUCCESS] All checks passed!
        echo.
        echo Ready to proceed with:
        echo   1. Start Docker Desktop
        echo   2. cd infra
        echo   3. docker-compose -f docker-compose.database.yml up -d
        echo   4. docker-compose -f docker-compose.cache.yml up -d
        echo   5. cd scripts
        echo   6. migrate.bat up
        echo ================================================
        exit /b 0
    ) else (
        echo ================================================
        echo [OK] Setup complete with warnings
        echo.
        echo Please review warnings above before proceeding.
        echo ================================================
        exit /b 0
    )
) else (
    echo ================================================
    echo [ERROR] Setup verification failed
    echo.
    echo Please fix the failed checks before proceeding:
    echo.
    if !FAIL_COUNT! gtr 0 (
        echo   - Install missing prerequisites
        echo   - Ensure all configuration files exist
        echo   - Check file permissions
    )
    echo ================================================
    exit /b 1
)
