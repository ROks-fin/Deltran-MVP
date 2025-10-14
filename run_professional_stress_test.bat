@echo off
REM Professional 4-Bank Multi-Region Stress Test Runner
REM DelTran Payment System - 3000 TPS Target
REM ================================================

echo.
echo ╔══════════════════════════════════════════════════════════════════════════════╗
echo ║                                                                              ║
echo ║         DELTRAN PROFESSIONAL MULTI-REGION STRESS TEST                        ║
echo ║                                                                              ║
echo ║  4-Bank Scenario: UAE • Israel • Pakistan • India                            ║
echo ║  Target: 3000 TPS Sustained Load                                             ║
echo ║                                                                              ║
echo ╚══════════════════════════════════════════════════════════════════════════════╝
echo.

REM ================================================
REM STEP 1: Check Docker
REM ================================================
echo [1/8] Checking Docker...
docker --version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Docker is not installed or not running
    echo Please install Docker Desktop and start it
    pause
    exit /b 1
)
echo ✓ Docker is available
echo.

REM ================================================
REM STEP 2: Stop existing containers
REM ================================================
echo [2/8] Stopping existing containers...
docker-compose -f infra/docker-compose.database.yml -f infra/docker-compose.cache.yml down >nul 2>&1
echo ✓ Old containers stopped
echo.

REM ================================================
REM STEP 3: Start PostgreSQL
REM ================================================
echo [3/8] Starting PostgreSQL cluster (primary + 2 replicas)...
cd infra
docker-compose -f docker-compose.database.yml up -d postgres-primary
echo Waiting for PostgreSQL to be ready...
timeout /t 10 /nobreak >nul

docker exec deltran-postgres-primary pg_isready -U deltran_app -d deltran >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: PostgreSQL failed to start
    echo Check logs: docker logs deltran-postgres-primary
    cd ..
    pause
    exit /b 1
)
echo ✓ PostgreSQL is ready
echo.

REM ================================================
REM STEP 4: Run database migrations
REM ================================================
echo [4/8] Running database migrations...
echo   - Core schema...
docker exec -i deltran-postgres-primary psql -U deltran_app -d deltran < sql/001_core_schema.sql >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Core schema migration failed
    cd ..
    pause
    exit /b 1
)

echo   - Advanced settlement features...
docker exec -i deltran-postgres-primary psql -U deltran_app -d deltran < sql/002_advanced_settlement.sql >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Advanced settlement migration failed
    cd ..
    pause
    exit /b 1
)

echo   - Test banks initialization...
docker exec -i deltran-postgres-primary psql -U deltran_app -d deltran < sql/003_init_test_banks.sql >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Test banks initialization failed
    cd ..
    pause
    exit /b 1
)
echo ✓ Database migrations completed
cd ..
echo.

REM ================================================
REM STEP 5: Start Redis
REM ================================================
echo [5/8] Starting Redis cluster (master + 2 replicas + 3 sentinels)...
cd infra
docker-compose -f docker-compose.cache.yml up -d redis-master
echo Waiting for Redis to be ready...
timeout /t 5 /nobreak >nul

docker exec deltran-redis-master redis-cli -a redis123 ping >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Redis failed to start
    echo Check logs: docker logs deltran-redis-master
    cd ..
    pause
    exit /b 1
)
echo ✓ Redis is ready
cd ..
echo.

REM ================================================
REM STEP 6: Build and start Gateway
REM ================================================
echo [6/8] Building and starting Gateway service...
cd gateway-go

echo Building gateway binary...
set CGO_ENABLED=1
go build -o gateway.exe ./cmd/gateway >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Gateway build failed
    cd ..
    pause
    exit /b 1
)

echo Starting gateway service...
start "DelTran Gateway" cmd /k "gateway.exe"

echo Waiting for gateway to be ready...
timeout /t 5 /nobreak >nul

REM Test gateway health
curl -s http://localhost:8080/health >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo WARNING: Gateway health check failed - continuing anyway
) else (
    echo ✓ Gateway is ready
)
cd ..
echo.

REM ================================================
REM STEP 7: Install Python dependencies
REM ================================================
echo [7/8] Checking Python dependencies...
python --version >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo ERROR: Python is not installed
    echo Please install Python 3.9 or higher
    pause
    exit /b 1
)

echo Installing required packages...
pip install --quiet aiohttp pytz >nul 2>&1
if %ERRORLEVEL% neq 0 (
    echo WARNING: Failed to install some packages - test may fail
)
echo ✓ Python dependencies ready
echo.

REM ================================================
REM STEP 8: Run stress test
REM ================================================
echo [8/8] Starting professional stress test...
echo.
echo ════════════════════════════════════════════════════════════════════════════════
echo.
echo The stress test will now run for 5 minutes (300 seconds)
echo Target: 3000 TPS across 4 banks
echo.
echo You can:
echo   - Monitor metrics: http://localhost:8080/api/v1/metrics/realtime
echo   - View web dashboard: http://localhost:3000
echo   - Check Redis: http://localhost:8081
echo   - Check PostgreSQL: http://localhost:5050
echo.
echo Press Ctrl+C to stop the test early
echo.
echo ════════════════════════════════════════════════════════════════════════════════
echo.

timeout /t 3 /nobreak >nul

python tests/bank_grade_multi_region_stress.py

REM ================================================
REM Test completed
REM ================================================
echo.
echo ════════════════════════════════════════════════════════════════════════════════
echo.
echo   STRESS TEST COMPLETED
echo.
echo   Results saved to: stress_test_report_*.json
echo.
echo   System is still running. You can:
echo     - Review metrics in web dashboard: http://localhost:3000
echo     - Check payment history: http://localhost:8080/api/v1/payments
echo     - View liquidity pools: Query v_liquidity_pool_status
echo     - Analyze netting cycles: Query v_active_netting_cycles
echo.
echo   To stop all services:
echo     docker-compose -f infra/docker-compose.database.yml ^
echo                     -f infra/docker-compose.cache.yml down
echo.
echo ════════════════════════════════════════════════════════════════════════════════
echo.

pause
