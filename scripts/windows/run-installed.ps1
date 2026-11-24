$ErrorActionPreference = 'Stop'

# Installed files are read-only; each Windows user owns their configuration and cache.
$dataDirectory = Join-Path ([Environment]::GetFolderPath('LocalApplicationData')) 'Calendar'
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

    $desktopProcess = Start-Process -FilePath (Join-Path $PSScriptRoot 'calendar-desktop.exe') -WorkingDirectory $dataDirectory -PassThru
    $desktopProcess.WaitForExit()
} catch {
    Add-Type -AssemblyName PresentationFramework
    [System.Windows.MessageBox]::Show($_.Exception.Message, 'Calendar could not start') | Out-Null
} finally {
    if ($ownsLaunch) { $launchMutex.ReleaseMutex() }
    $launchMutex.Dispose()
}
