@echo off
REM K6 Test Runner for DelTran MVP (Windows)
REM Runs all K6 performance tests and generates reports

echo ==========================================
echo DelTran MVP - K6 Performance Test Runner
echo ==========================================
echo.

REM Create results directory
set RESULTS_DIR=.\results
if not exist "%RESULTS_DIR%" mkdir "%RESULTS_DIR%"

REM Timestamp for this test run
for /f "tokens=2-4 delims=/ " %%a in ('date /t') do (set mydate=%%c%%a%%b)
for /f "tokens=1-2 delims=/:" %%a in ('time /t') do (set mytime=%%a%%b)
set TIMESTAMP=%mydate%_%mytime%
set RUN_DIR=%RESULTS_DIR%\run_%TIMESTAMP%
mkdir "%RUN_DIR%"

echo Results directory: %RUN_DIR%
echo.

REM Check if K6 is installed
where k6 >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] K6 is not installed!
    echo Install K6: https://k6.io/docs/getting-started/installation/
    exit /b 1
)

echo [OK] K6 is installed
echo.

REM Test execution tracker
set TOTAL_TESTS=0
set PASSED_TESTS=0
set FAILED_TESTS=0

REM 1. Integration Test - Health Checks
echo ==========================================
echo Running: Integration Test - Health Checks
echo ==========================================
set /a TOTAL_TESTS+=1
k6 run --out json="%RUN_DIR%\integration.json" ".\scenarios\integration-test.js"
if %ERRORLEVEL% equ 0 (
    echo [OK] Integration Test completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [FAILED] Integration Test failed
    set /a FAILED_TESTS+=1
)
echo.

REM 2. E2E Transaction Flow Test
echo ==========================================
echo Running: E2E Transaction Flow Test
echo ==========================================
set /a TOTAL_TESTS+=1
k6 run --out json="%RUN_DIR%\e2e.json" ".\scenarios\e2e-transaction.js"
if %ERRORLEVEL% equ 0 (
    echo [OK] E2E Test completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [FAILED] E2E Test failed
    set /a FAILED_TESTS+=1
)
echo.

REM 3. Load Test - Realistic Scenarios
echo ==========================================
echo Running: Load Test - Realistic Scenarios
echo ==========================================
set /a TOTAL_TESTS+=1
k6 run --out json="%RUN_DIR%\load.json" ".\scenarios\load-test-realistic.js"
if %ERRORLEVEL% equ 0 (
    echo [OK] Load Test completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [FAILED] Load Test failed
    set /a FAILED_TESTS+=1
)
echo.

REM 4. WebSocket Test - Notification Engine
echo ==========================================
echo Running: WebSocket Test - Notification Engine
echo ==========================================
set /a TOTAL_TESTS+=1
k6 run --out json="%RUN_DIR%\websocket.json" ".\scenarios\websocket-test.js"
if %ERRORLEVEL% equ 0 (
    echo [OK] WebSocket Test completed successfully
    set /a PASSED_TESTS+=1
) else (
    echo [FAILED] WebSocket Test failed
    set /a FAILED_TESTS+=1
)
echo.

REM Final Summary
echo ==========================================
echo Test Run Summary
echo ==========================================
echo Total Tests:  %TOTAL_TESTS%
echo Passed:       %PASSED_TESTS%
echo Failed:       %FAILED_TESTS%
echo.
echo Results saved to: %RUN_DIR%
echo.

echo ==========================================
echo Test run completed!
echo ==========================================

REM Exit with error code if any tests failed
if %FAILED_TESTS% gtr 0 (
    exit /b 1
) else (
    exit /b 0
)
