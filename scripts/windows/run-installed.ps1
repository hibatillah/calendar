$ErrorActionPreference = 'Stop'

# Installed files are read-only; each Windows user owns their configuration and cache.
$dataDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Calendar'
$backendProcess = $null
$launchMutex = New-Object System.Threading.Mutex($false, 'Local\Calendar.InstalledLauncher')
$ownsLaunch = $false

try {
    try { $ownsLaunch = $launchMutex.WaitOne(0) } catch [System.Threading.AbandonedMutexException] { $ownsLaunch = $true }
    if (!$ownsLaunch) { return }

    New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
    Set-Location -LiteralPath $dataDirectory

    if (!(Test-Path -LiteralPath '.env')) {
        Copy-Item -LiteralPath (Join-Path $PSScriptRoot '.env.example') -Destination '.env'
    }

    $backendPath = Join-Path $PSScriptRoot 'calendar-backend.exe'
    $desktopPath = Join-Path $PSScriptRoot 'calendar-desktop.exe'
    $existingService = $null

    try {
        $existingService = Invoke-RestMethod 'http://127.0.0.1:47821/health' -TimeoutSec 2
    } catch { }

    if ($existingService) {
        if ($existingService.mode -ne 'GOOGLE_CALENDAR' -or !(Test-Path -LiteralPath '.local/session-token')) {
            throw 'Another application or Calendar installation is using port 47821. Close it before launching the installed Calendar.'
        }
        try {
            $sessionToken = (Get-Content -LiteralPath '.local/session-token' -Raw).Trim()
            $connection = Invoke-RestMethod 'http://127.0.0.1:47821/graphql' -Method Post -ContentType 'application/json' -Headers @{ 'x-calendar-token' = $sessionToken } -Body '{"query":"{ mode }"}' -TimeoutSec 2
            if ($connection.data.mode -ne 'GOOGLE_CALENDAR') { throw 'Unexpected backend response.' }
        } catch {
            throw 'A different Calendar installation is using port 47821. Close it before launching the installed Calendar.'
        } finally {
            $sessionToken = $null
        }
    } else {
        $backendProcess = Start-Process -FilePath $backendPath -WorkingDirectory $dataDirectory -WindowStyle Hidden -PassThru
        $ready = $false

        for ($attempt = 0; $attempt -lt 50; $attempt++) {
            Start-Sleep -Milliseconds 300

            if ($backendProcess.HasExited) { throw 'The backend exited before startup. Check the logs in %LOCALAPPDATA%\Calendar\.local\logs.' }

            try {
                $health = Invoke-RestMethod 'http://127.0.0.1:47821/health' -TimeoutSec 1
                if ($health.mode -eq 'GOOGLE_CALENDAR') { $ready = $true; break }
            } catch { }
        }

        if (!$ready) { throw 'The backend did not become ready. Check the logs in %LOCALAPPDATA%\Calendar\.local\logs.' }
    }

    $desktopProcess = Start-Process -FilePath $desktopPath -WorkingDirectory $dataDirectory -PassThru
    $desktopProcess.WaitForExit()
} catch {
    Add-Type -AssemblyName PresentationFramework
    [System.Windows.MessageBox]::Show($_.Exception.Message, 'Calendar could not start') | Out-Null
} finally {
    if ($backendProcess -and !$backendProcess.HasExited) {
        Stop-Process -Id $backendProcess.Id
    }
    if ($ownsLaunch) { $launchMutex.ReleaseMutex() }
    $launchMutex.Dispose()
}
