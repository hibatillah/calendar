$ErrorActionPreference = 'Stop'

Set-Location (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)

$desktopPath = Join-Path $PWD 'target/debug/calendar-desktop.exe'

. (Join-Path $PSScriptRoot 'dev-desktop.ps1') -LibraryOnly
Get-Process calendar-desktop -ErrorAction SilentlyContinue | Where-Object {
    Test-OwnedExecutable $_.Path $repositoryRoot $_.ProcessName
} | ForEach-Object { & taskkill /F /PID $_.Id }
& (Join-Path $PSScriptRoot 'cargo.ps1') build --workspace

if ($LASTEXITCODE -ne 0) { throw 'Build failed; see compiler output.' }

Start-Process -FilePath $desktopPath -WorkingDirectory $PWD
