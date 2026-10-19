param([ValidateSet('debug', 'release')][string]$Profile = 'release')
$ErrorActionPreference = 'Stop'

Set-Location (Split-Path $PSScriptRoot -Parent)

$packageDirectory = Join-Path $PWD 'dist/calendar-windows-x64'

if (Test-Path -LiteralPath $packageDirectory) { throw 'Package directory already exists. Use a clean staging directory before packaging.' }

New-Item -ItemType Directory -Path $packageDirectory -Force | Out-Null

foreach ($binary in @('calendar-desktop.exe', 'calendar-backend.exe')) {
    Copy-Item -LiteralPath (Join-Path $PWD "target/$Profile/$binary") -Destination $packageDirectory
}

Copy-Item -LiteralPath 'scripts/run-portable.ps1' -Destination (Join-Path $packageDirectory 'Run-Calendar.ps1')
Copy-Item -LiteralPath '.env.example','LICENSE','README.md','CONTRIBUTING.md' -Destination $packageDirectory
Copy-Item -LiteralPath 'docs' -Destination $packageDirectory -Recurse

$licenseDirectory = Join-Path $packageDirectory 'third-party-licenses'

New-Item -ItemType Directory -Path $licenseDirectory | Out-Null
Copy-Item -LiteralPath 'assets/fonts/OFL.txt','assets/graphiql/LICENSE.graphiql','assets/graphiql/LICENSE.react' -Destination $licenseDirectory
Compress-Archive -Path "$packageDirectory/*" -DestinationPath 'dist/calendar-windows-x64.zip' -Force
Write-Host 'Created dist/calendar-windows-x64.zip (no local state or credentials included).'
