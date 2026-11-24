$ErrorActionPreference = 'Stop'

Set-Location $PSScriptRoot

Start-Process -FilePath (Join-Path $PSScriptRoot 'calendar-desktop.exe') -WorkingDirectory $PSScriptRoot
