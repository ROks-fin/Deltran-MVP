@echo off
echo Testing Regulatory Reports API...
echo.

echo 1. Testing report generation...
curl -s -X POST http://localhost:8080/api/v1/compliance/reports/generate -H "Content-Type: application/json" -d "{\"report_type\":\"AML_ANNUAL\",\"format\":\"excel\",\"start_date\":\"2025-01-01T00:00:00Z\",\"end_date\":\"2025-01-31T23:59:59Z\",\"generated_by\":\"test_user\"}"
echo.
echo.

timeout /t 2 /nobreak > nul

echo 2. Testing report list...
curl -s http://localhost:8080/api/v1/compliance/reports
echo.
echo.

echo 3. Testing report statistics...
curl -s http://localhost:8080/api/v1/compliance/reports/stats
echo.
echo.

echo Done!
