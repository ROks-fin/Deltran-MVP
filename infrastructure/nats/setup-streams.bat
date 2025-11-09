@echo off
REM NATS JetStream Stream Setup for DelTran MVP (Windows)
REM Version: 1.0

setlocal enabledelayedexpansion

set NATS_URL=nats://localhost:4222
if not "%1"=="" set NATS_URL=%1

echo ========================================
echo 🚀 DelTran NATS JetStream Setup
echo ========================================
echo NATS URL: %NATS_URL%
echo.

echo ========================================
echo 1️⃣  CREATING STREAMS
echo ========================================
echo.

REM 1. TOKEN EVENTS STREAM
echo 📦 Creating stream: DELTRAN_TOKEN_EVENTS
nats stream add DELTRAN_TOKEN_EVENTS --subjects="deltran.token.>" --retention=limits --storage=file --replicas=1 --max-age=7d --discard=old --dupe-window=2m --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_TOKEN_EVENTS configured) else (echo   ⚠️  Stream may already exist)

REM 2. OBLIGATION EVENTS STREAM
echo 📦 Creating stream: DELTRAN_OBLIGATION_EVENTS
nats stream add DELTRAN_OBLIGATION_EVENTS --subjects="deltran.obligation.>" --retention=limits --storage=file --replicas=1 --max-age=30d --discard=old --dupe-window=2m --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_OBLIGATION_EVENTS configured) else (echo   ⚠️  Stream may already exist)

REM 3. CLEARING EVENTS STREAM
echo 📦 Creating stream: DELTRAN_CLEARING_EVENTS
nats stream add DELTRAN_CLEARING_EVENTS --subjects="deltran.clearing.>" --retention=limits --storage=file --replicas=1 --max-age=90d --discard=old --dupe-window=2m --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_CLEARING_EVENTS configured) else (echo   ⚠️  Stream may already exist)

REM 4. SETTLEMENT EVENTS STREAM
echo 📦 Creating stream: DELTRAN_SETTLEMENT_EVENTS
nats stream add DELTRAN_SETTLEMENT_EVENTS --subjects="deltran.settlement.>" --retention=limits --storage=file --replicas=1 --max-age=90d --discard=old --dupe-window=2m --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_SETTLEMENT_EVENTS configured) else (echo   ⚠️  Stream may already exist)

REM 5. NOTIFICATION STREAM
echo 📦 Creating stream: DELTRAN_NOTIFICATIONS
nats stream add DELTRAN_NOTIFICATIONS --subjects="deltran.notification.>" --retention=work-queue --storage=file --replicas=1 --max-age=24h --discard=old --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_NOTIFICATIONS configured) else (echo   ⚠️  Stream may already exist)

REM 6. AUDIT LOG STREAM
echo 📦 Creating stream: DELTRAN_AUDIT_LOG
nats stream add DELTRAN_AUDIT_LOG --subjects="deltran.audit.>" --retention=limits --storage=file --replicas=1 --max-age=365d --discard=old --dupe-window=2m --deny-delete --deny-purge --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_AUDIT_LOG configured) else (echo   ⚠️  Stream may already exist)

REM 7. RISK ALERTS STREAM
echo 📦 Creating stream: DELTRAN_RISK_ALERTS
nats stream add DELTRAN_RISK_ALERTS --subjects="deltran.risk.>" --retention=limits --storage=file --replicas=1 --max-age=30d --discard=old --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_RISK_ALERTS configured) else (echo   ⚠️  Stream may already exist)

REM 8. REPORTING STREAM
echo 📦 Creating stream: DELTRAN_REPORTING
nats stream add DELTRAN_REPORTING --subjects="deltran.report.>" --retention=work-queue --storage=file --replicas=1 --max-age=7d --discard=old --server=%NATS_URL% --force 2>nul
if %errorlevel% equ 0 (echo   ✅ Stream DELTRAN_REPORTING configured) else (echo   ⚠️  Stream may already exist)

echo.
echo ========================================
echo 2️⃣  CREATING CONSUMERS
echo ========================================
echo.

REM Notification Engine consumers
echo 👤 Creating notification consumers
nats consumer add DELTRAN_TOKEN_EVENTS notification-token-consumer --filter="deltran.token.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
nats consumer add DELTRAN_OBLIGATION_EVENTS notification-obligation-consumer --filter="deltran.obligation.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
nats consumer add DELTRAN_CLEARING_EVENTS notification-clearing-consumer --filter="deltran.clearing.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
nats consumer add DELTRAN_SETTLEMENT_EVENTS notification-settlement-consumer --filter="deltran.settlement.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
echo   ✅ Notification consumers configured

REM Settlement Engine consumers
echo 👤 Creating settlement consumers
nats consumer add DELTRAN_CLEARING_EVENTS settlement-clearing-consumer --filter="deltran.clearing.window.closed" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
nats consumer add DELTRAN_CLEARING_EVENTS settlement-netting-consumer --filter="deltran.clearing.netting.completed" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
echo   ✅ Settlement consumers configured

REM Reporting Engine consumers
echo 👤 Creating reporting consumers
nats consumer add DELTRAN_CLEARING_EVENTS reporting-clearing-consumer --filter="deltran.clearing.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
nats consumer add DELTRAN_SETTLEMENT_EVENTS reporting-settlement-consumer --filter="deltran.settlement.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
nats consumer add DELTRAN_AUDIT_LOG reporting-audit-consumer --filter="deltran.audit.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
echo   ✅ Reporting consumers configured

REM Compliance Engine consumer
echo 👤 Creating compliance consumer
nats consumer add DELTRAN_AUDIT_LOG compliance-audit-consumer --filter="deltran.audit.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
echo   ✅ Compliance consumer configured

REM Risk Engine consumer
echo 👤 Creating risk consumer
nats consumer add DELTRAN_RISK_ALERTS risk-alert-processor --filter="deltran.risk.>" --ack=explicit --pull --deliver=all --max-pending=1000 --server=%NATS_URL% --force 2>nul
echo   ✅ Risk consumer configured

echo.
echo ========================================
echo 3️⃣  VERIFICATION
echo ========================================
echo.

echo 📊 Listing all streams:
nats stream list --server=%NATS_URL% 2>nul

echo.
echo ========================================
echo ✅ NATS JetStream Setup Complete!
echo ========================================
echo.
echo 📚 Stream Details:
echo   - DELTRAN_TOKEN_EVENTS: 7d retention
echo   - DELTRAN_OBLIGATION_EVENTS: 30d retention
echo   - DELTRAN_CLEARING_EVENTS: 90d retention
echo   - DELTRAN_SETTLEMENT_EVENTS: 90d retention
echo   - DELTRAN_NOTIFICATIONS: 24h retention
echo   - DELTRAN_AUDIT_LOG: 365d retention
echo   - DELTRAN_RISK_ALERTS: 30d retention
echo   - DELTRAN_REPORTING: 7d retention
echo.
echo 🔍 Monitoring:
echo   - NATS Monitoring UI: http://localhost:8222
echo.

pause
