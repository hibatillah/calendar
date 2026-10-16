$ErrorActionPreference = 'Stop'
Set-Location (Split-Path $PSScriptRoot -Parent)
$desktopPath = Join-Path $PWD 'target/debug/calendar-desktop.exe'
$backendPath = Join-Path $PWD 'target/debug/calendar-backend.exe'
Get-Process calendar-desktop,calendar-backend -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $desktopPath -or $_.Path -eq $backendPath } | ForEach-Object { & taskkill /F /PID $_.Id }
# Stop binaries from the previous name before binding the same local backend port.
$legacyDesktopPath = Join-Path $PWD 'target/debug/randevu-desktop.exe'
$legacyBackendPath = Join-Path $PWD 'target/debug/randevu-backend.exe'
Get-Process randevu-desktop,randevu-backend -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $legacyDesktopPath -or $_.Path -eq $legacyBackendPath } | ForEach-Object { & taskkill /F /PID $_.Id }
& (Join-Path $PSScriptRoot 'cargo.ps1') build --workspace
if ($LASTEXITCODE -ne 0) { throw 'Build failed; see compiler output.' }
try { $null = Invoke-RestMethod 'http://127.0.0.1:47821/health' -TimeoutSec 2 }
catch { Start-Process -FilePath $backendPath -WorkingDirectory $PWD -WindowStyle Hidden }
Start-Process -FilePath $desktopPath -WorkingDirectory $PWD
