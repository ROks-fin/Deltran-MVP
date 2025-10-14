@echo off
REM DelTran System - Comprehensive Test Suite Runner (Windows)

setlocal enabledelayedexpansion

set GATEWAY_URL=http://localhost:8080
set TEST_DIR=%~dp0
set TIMESTAMP=%date:~-4%%date:~-7,2%%date:~-10,2%_%time:~0,2%%time:~3,2%%time:~6,2%
set TIMESTAMP=%TIMESTAMP: =0%
set RESULTS_DIR=%TEST_DIR%results

echo ===============================================================
echo    DelTran System - Comprehensive Test Suite
echo ===============================================================
echo.

REM Create results directory
if not exist "%RESULTS_DIR%" mkdir "%RESULTS_DIR%"

REM Check if gateway is running
echo [1/5] Checking Gateway Status...
curl -s %GATEWAY_URL%/health >nul 2>&1
if %ERRORLEVEL% EQU 0 (
    echo [OK] Gateway is running at %GATEWAY_URL%
) else (
    echo [ERROR] Gateway is not running at %GATEWAY_URL%
    echo Please start the gateway service first:
    echo   cd gateway ^&^& cargo run --release
    exit /b 1
)
echo.

REM Run Component Tests
echo [2/5] Running Component Tests...
python "%TEST_DIR%component_test_suite.py" > "%RESULTS_DIR%\component_tests_%TIMESTAMP%.log" 2>&1
set COMPONENT_RESULT=%ERRORLEVEL%
echo.

if %COMPONENT_RESULT% EQU 0 (
    echo [OK] Component tests completed
) else (
    echo [WARNING] Component tests completed with warnings
)
echo.

REM Run Multi-Bank Stress Test
echo [3/5] Running Multi-Bank Integration Stress Test...
echo This will take approximately 5 minutes...
python "%TEST_DIR%stress_test_multibank.py" > "%RESULTS_DIR%\stress_test_%TIMESTAMP%.log" 2>&1
set STRESS_RESULT=%ERRORLEVEL%
echo.

if %STRESS_RESULT% EQU 0 (
    echo [OK] Stress test completed
) else (
    echo [ERROR] Stress test failed
)
echo.

REM Check Gateway Metrics
echo [4/5] Checking System Metrics...
curl -s %GATEWAY_URL%/api/v1/metrics/live 2>nul
if %ERRORLEVEL% EQU 0 (
    echo [OK] Metrics endpoint responsive
) else (
    echo [ERROR] Failed to fetch metrics
)
echo.

REM Check Recent Transactions
echo [5/5] Checking Recent Transactions...
curl -s %GATEWAY_URL%/api/payments 2>nul
if %ERRORLEVEL% EQU 0 (
    echo [OK] Payments endpoint responsive
) else (
    echo [ERROR] Failed to fetch payments
)
echo.

REM Generate Summary Report
echo ===============================================================
echo    TEST SUMMARY
echo ===============================================================
echo.
echo Test Run: %TIMESTAMP%
echo Gateway URL: %GATEWAY_URL%
echo.
echo Results:

if %COMPONENT_RESULT% EQU 0 (
    echo   [OK] Component Tests
) else (
    echo   [WARNING] Component Tests
)

if %STRESS_RESULT% EQU 0 (
    echo   [OK] Stress Test
) else (
    echo   [ERROR] Stress Test
)

echo.
echo Log files saved to: %RESULTS_DIR%
echo.

REM Overall result
if %COMPONENT_RESULT% EQU 0 (
    if %STRESS_RESULT% EQU 0 (
        echo ===============================================================
        echo    ALL TESTS PASSED
        echo ===============================================================
        exit /b 0
    )
)

echo ===============================================================
echo    TESTS COMPLETED WITH ISSUES
echo ===============================================================
exit /b 1
