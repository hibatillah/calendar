$ErrorActionPreference = 'Stop'
Set-Location $PSScriptRoot
$backendPath = Join-Path $PSScriptRoot 'randevu-backend.exe'
$desktopPath = Join-Path $PSScriptRoot 'randevu-desktop.exe'
try {
    $health = Invoke-RestMethod 'http://127.0.0.1:47821/health' -TimeoutSec 2
    if ($health.mode -ne 'GOOGLE_CALENDAR') { throw 'An unrelated service is using port 47821.' }
    if (!(Test-Path -LiteralPath '.local/session-token')) { throw 'Another Randevu installation is running. Close it before launching this copy.' }
} catch {
    if ($_.Exception.Message -match 'Another Randevu|unrelated service') { throw }
    Start-Process -FilePath $backendPath -WorkingDirectory $PSScriptRoot -WindowStyle Hidden
    $ready = $false
    for ($attempt = 0; $attempt -lt 30; $attempt++) {
        Start-Sleep -Milliseconds 300
        try {
            $health = Invoke-RestMethod 'http://127.0.0.1:47821/health' -TimeoutSec 1
            if ($health.mode -eq 'GOOGLE_CALENDAR') { $ready = $true; break }
        } catch { }
    }
    if (!$ready) { throw 'The local backend did not start. Check .local/logs.' }
}
Start-Process -FilePath $desktopPath -WorkingDirectory $PSScriptRoot
