@echo off
REM DelTran - Start System and Run Tests

echo ===============================================================
echo    DelTran System - Starting Services and Running Tests
echo ===============================================================
echo.

REM Check if Go is installed
where go >nul 2>nul
if %ERRORLEVEL% NEQ 0 (
    echo [ERROR] Go is not installed or not in PATH
    echo Please install Go from https://golang.org/dl/
    exit /b 1
)

REM Start Gateway Service in background
echo [1/3] Starting Gateway Service...
cd "%~dp0gateway-go"
start /B cmd /c "go run cmd/gateway/main.go > gateway.log 2>&1"
timeout /t 3 /nobreak >nul

REM Check if gateway is running
echo Checking gateway health...
curl -s http://localhost:8080/health >nul 2>nul
if %ERRORLEVEL% EQU 0 (
    echo [OK] Gateway is running on port 8080
) else (
    echo [WARNING] Gateway may not be ready yet, waiting...
    timeout /t 5 /nobreak >nul
)
echo.

REM Run Component Tests
echo [2/3] Running Component Tests...
cd "%~dp0"
python tests\component_test_suite.py
echo.

REM Run Stress Tests (optional - commented out for quick testing)
REM echo [3/3] Running Stress Tests (5 minutes)...
REM python tests\stress_test_multibank.py
REM echo.

echo ===============================================================
echo    Testing Complete!
echo    Gateway running on http://localhost:8080
echo    Web UI available on http://localhost:3000
echo ===============================================================
echo.
echo Press any key to stop gateway service...
pause >nul

REM Kill gateway process
taskkill /F /IM go.exe >nul 2>nul
echo Gateway stopped.
