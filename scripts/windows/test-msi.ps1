$ErrorActionPreference = 'Stop'

if ($env:CI -ne 'true') { throw 'Install/uninstall smoke tests must run in a disposable CI runner.' }

Set-Location (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)
$installer = (Resolve-Path 'dist/calendar-windows-x64.msi').Path
$installationDirectory = Join-Path $env:ProgramFiles 'Calendar'
$dataDirectory = Join-Path $env:LOCALAPPDATA 'Calendar'
$preservedFile = Join-Path $dataDirectory 'installer-smoke-test.txt'
$installed = $false

New-Item -ItemType Directory -Path $dataDirectory -Force | Out-Null
Set-Content -LiteralPath $preservedFile -Value 'Uninstall must preserve user data.'

function Invoke-Installer([string]$Action, [string]$LogName) {
    $logPath = Join-Path $PWD "dist/$LogName"
    $process = Start-Process msiexec.exe -ArgumentList "$Action `"$installer`" /qn /norestart /l*v `"$logPath`"" -Wait -PassThru -WindowStyle Hidden
    if ($process.ExitCode -notin @(0, 3010)) { throw "Windows Installer failed with $($process.ExitCode). See $logPath." }
}

try {
    Invoke-Installer '/i' 'msi-install.log'
    $installed = $true
    foreach ($file in @('calendar-desktop.exe', 'calendar-backend.exe', 'Run-Calendar.ps1', '.env.example')) {
        if (!(Test-Path -LiteralPath (Join-Path $installationDirectory $file))) { throw "Missing installed file: $file" }
    }

    $menuDirectory = Join-Path ([Environment]::GetFolderPath('CommonPrograms')) 'Calendar'
    foreach ($shortcut in @('Calendar.lnk', 'Uninstall Calendar.lnk')) {
        if (!(Test-Path -LiteralPath (Join-Path $menuDirectory $shortcut))) { throw "Missing shortcut: $shortcut" }
    }

    Invoke-Installer '/x' 'msi-uninstall.log'
    $installed = $false

    if (Test-Path -LiteralPath (Join-Path $installationDirectory 'calendar-desktop.exe')) { throw 'Uninstall left the application executable behind.' }
    if (Test-Path -LiteralPath (Join-Path $menuDirectory 'Calendar.lnk')) { throw 'Uninstall left its Start menu shortcut behind.' }
    if (!(Test-Path -LiteralPath $preservedFile)) { throw 'Uninstall removed user data.' }
} finally {
    if ($installed) { Invoke-Installer '/x' 'msi-cleanup.log' }
    Remove-Item -LiteralPath $preservedFile -ErrorAction SilentlyContinue
}

Write-Host 'MSI install, shortcuts, uninstall, and user-data preservation passed.'
