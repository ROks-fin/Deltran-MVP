@echo off
REM DelTran Database Migration Script for Windows
REM Handles schema creation and upgrades

setlocal enabledelayedexpansion

set SCRIPT_DIR=%~dp0
set SQL_DIR=%SCRIPT_DIR%..\sql
set LOG_DIR=%SCRIPT_DIR%..\..\logs

REM Database connection parameters
if not defined POSTGRES_HOST set POSTGRES_HOST=localhost
if not defined POSTGRES_PORT set POSTGRES_PORT=5432
if not defined POSTGRES_DB set POSTGRES_DB=deltran
if not defined POSTGRES_USER set POSTGRES_USER=deltran_app
if not defined POSTGRES_PASSWORD set POSTGRES_PASSWORD=changeme123

REM Create log directory
if not exist "%LOG_DIR%" mkdir "%LOG_DIR%"
set LOG_FILE=%LOG_DIR%\migration_%date:~-4,4%%date:~-10,2%%date:~-7,2%_%time:~0,2%%time:~3,2%%time:~6,2%.log
set LOG_FILE=%LOG_FILE: =0%

echo ===================================== >> "%LOG_FILE%"
echo DelTran Database Migration >> "%LOG_FILE%"
echo Started: %date% %time% >> "%LOG_FILE%"
echo ===================================== >> "%LOG_FILE%"

REM Check if psql is available
where psql >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] psql command not found. Please install PostgreSQL client.
    echo [ERROR] psql command not found. >> "%LOG_FILE%"
    exit /b 1
)

REM Parse command
set COMMAND=%1
if "%COMMAND%"=="" set COMMAND=help

if "%COMMAND%"=="up" goto migrate_up
if "%COMMAND%"=="status" goto status
if "%COMMAND%"=="backup" goto backup
if "%COMMAND%"=="help" goto help
if "%COMMAND%"=="--help" goto help
if "%COMMAND%"=="-h" goto help

echo [ERROR] Unknown command: %COMMAND%
goto help

:migrate_up
echo [INFO] Starting database migration...
echo [INFO] Starting database migration... >> "%LOG_FILE%"

REM Check PostgreSQL connection
echo [INFO] Checking PostgreSQL connection...
set PGPASSWORD=%POSTGRES_PASSWORD%
psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d postgres -c "SELECT 1" >nul 2>nul
if %ERRORLEVEL% neq 0 (
    echo [ERROR] Cannot connect to PostgreSQL at %POSTGRES_HOST%:%POSTGRES_PORT%
    echo [ERROR] Cannot connect to PostgreSQL >> "%LOG_FILE%"
    exit /b 1
)
echo [INFO] PostgreSQL connection successful

REM Create database if not exists
echo [INFO] Creating database '%POSTGRES_DB%' if not exists...
psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d postgres -tc "SELECT 1 FROM pg_database WHERE datname = '%POSTGRES_DB%'" | findstr "1" >nul
if %ERRORLEVEL% neq 0 (
    psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d postgres -c "CREATE DATABASE %POSTGRES_DB%"
    echo [INFO] Database created
)

REM Create migration tracking table
echo [INFO] Creating migration tracking table...
psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d %POSTGRES_DB% -c "CREATE SCHEMA IF NOT EXISTS public; CREATE TABLE IF NOT EXISTS public.schema_migrations (id SERIAL PRIMARY KEY, version VARCHAR(50) UNIQUE NOT NULL, name VARCHAR(255) NOT NULL, applied_at TIMESTAMPTZ DEFAULT NOW(), execution_time_ms INTEGER, checksum VARCHAR(64), success BOOLEAN DEFAULT true);"

REM Apply migrations
echo [INFO] Applying migrations...
set MIGRATION_COUNT=0

for %%f in ("%SQL_DIR%\*.sql") do (
    echo [INFO] Applying migration: %%~nxf
    echo [INFO] Applying migration: %%~nxf >> "%LOG_FILE%"

    REM Extract version from filename (first 3 digits)
    set "filename=%%~nf"
    set "version=!filename:~0,3!"
    set "name=!filename:~4!"

    REM Apply migration
    psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d %POSTGRES_DB% -v ON_ERROR_STOP=1 -f "%%f" >> "%LOG_FILE%" 2>&1
    if !ERRORLEVEL! equ 0 (
        echo [SUCCESS] Migration !version! applied successfully
        echo [SUCCESS] Migration !version! applied successfully >> "%LOG_FILE%"

        REM Record migration
        psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d %POSTGRES_DB% -c "INSERT INTO public.schema_migrations (version, name, success) VALUES ('!version!', '!name!', true) ON CONFLICT (version) DO UPDATE SET applied_at = NOW(), success = true;"

        set /a MIGRATION_COUNT+=1
    ) else (
        echo [ERROR] Migration !version! failed
        echo [ERROR] Migration !version! failed >> "%LOG_FILE%"

        REM Record failed migration
        psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d %POSTGRES_DB% -c "INSERT INTO public.schema_migrations (version, name, success) VALUES ('!version!', '!name!', false) ON CONFLICT (version) DO UPDATE SET applied_at = NOW(), success = false;"

        goto migration_failed
    )
)

echo.
echo ===================================
echo Migration complete!
echo Applied %MIGRATION_COUNT% migration(s)
echo Log file: %LOG_FILE%
echo ===================================
echo.
exit /b 0

:migration_failed
echo.
echo ===================================
echo [ERROR] Migration failed!
echo Check log file: %LOG_FILE%
echo ===================================
echo.
exit /b 1

:status
echo [INFO] Migration status:
set PGPASSWORD=%POSTGRES_PASSWORD%
psql -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d %POSTGRES_DB% -c "SELECT version, name, applied_at, execution_time_ms || 'ms' as execution_time, CASE WHEN success THEN 'OK' ELSE 'FAIL' END as status FROM public.schema_migrations ORDER BY version DESC LIMIT 10;"
exit /b 0

:backup
echo [INFO] Creating database backup...
set BACKUP_DIR=%SCRIPT_DIR%..\..\data\backups
if not exist "%BACKUP_DIR%" mkdir "%BACKUP_DIR%"

set BACKUP_FILE=%BACKUP_DIR%\deltran_backup_%date:~-4,4%%date:~-10,2%%date:~-7,2%_%time:~0,2%%time:~3,2%%time:~6,2%.sql
set BACKUP_FILE=%BACKUP_FILE: =0%

set PGPASSWORD=%POSTGRES_PASSWORD%
pg_dump -h %POSTGRES_HOST% -p %POSTGRES_PORT% -U %POSTGRES_USER% -d %POSTGRES_DB% -F p -f "%BACKUP_FILE%"

if %ERRORLEVEL% equ 0 (
    echo [SUCCESS] Backup created: %BACKUP_FILE%
    echo [INFO] Compressing backup...
    REM Windows doesn't have gzip by default, skip compression
    echo [INFO] Backup ready: %BACKUP_FILE%
) else (
    echo [ERROR] Backup failed
    exit /b 1
)
exit /b 0

:help
echo.
echo DelTran Database Migration Tool
echo.
echo Usage: %~nx0 ^<command^> [options]
echo.
echo Commands:
echo     up              Apply all pending migrations
echo     status          Show migration status
echo     backup          Create database backup
echo     help            Show this help message
echo.
echo Environment Variables:
echo     POSTGRES_HOST     Database host (default: localhost)
echo     POSTGRES_PORT     Database port (default: 5432)
echo     POSTGRES_DB       Database name (default: deltran)
echo     POSTGRES_USER     Database user (default: deltran_app)
echo     POSTGRES_PASSWORD Database password (default: changeme123)
echo.
echo Examples:
echo     %~nx0 up
echo     %~nx0 status
echo     %~nx0 backup
echo.
exit /b 0
