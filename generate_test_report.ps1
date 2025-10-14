$body = @{
    report_type = "AML_ANNUAL"
    format = "excel"
    start_date = "2025-01-01T00:00:00Z"
    end_date = "2025-01-31T23:59:59Z"
    generated_by = "web_user"
} | ConvertTo-Json

Write-Host "Generating AML Annual Report..."
$response = Invoke-RestMethod -Method Post -Uri 'http://localhost:8080/api/v1/compliance/reports/generate' -ContentType 'application/json' -Body $body
$response | ConvertTo-Json

Write-Host "`n`nListing reports..."
Invoke-RestMethod -Uri 'http://localhost:8080/api/v1/compliance/reports' | ConvertTo-Json -Depth 5
