param(
    [ValidateSet('debug', 'release')][string]$Profile = 'release',
    [ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version = '0.1.0',
    [string]$WixPath = 'wix'
)
$ErrorActionPreference = 'Stop'

Set-Location (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)
$packageDirectory = Join-Path $PWD 'dist/calendar-windows-x64'
$installerDirectory = Join-Path $PWD 'dist/msi'

if (!(Test-Path -LiteralPath $packageDirectory)) { & ./scripts/windows/package-windows.ps1 -Profile $Profile }
New-Item -ItemType Directory -Path $installerDirectory -Force | Out-Null

$escapeXml = { param($Value) [System.Security.SecurityElement]::Escape($Value) }
$components = [System.Collections.Generic.List[string]]::new()

foreach ($file in Get-ChildItem -LiteralPath $packageDirectory -File -Recurse | Sort-Object FullName) {
    $relativePath = [System.IO.Path]::GetRelativePath($packageDirectory, $file.FullName)
    if ($relativePath -in @('Run-Calendar.ps1', 'Run-Calendar.sh')) { continue }
    $hash = [Convert]::ToHexString([System.Security.Cryptography.SHA256]::HashData([Text.Encoding]::UTF8.GetBytes($relativePath))).Substring(0, 24)
    $source = & $escapeXml $file.FullName
    $subdirectory = Split-Path $relativePath -Parent
    $directoryAttribute = if ($subdirectory) { ' Subdirectory="' + (& $escapeXml $subdirectory) + '"' } else { '' }
    $components.Add("<Component Id=`"Payload_$hash`" Directory=`"INSTALLFOLDER`"$directoryAttribute Guid=`"*`"><File Source=`"$source`" KeyPath=`"yes`" /></Component>")
}

$launcher = & $escapeXml (Join-Path $PWD 'scripts/windows/run-installed.ps1')
$components.Add("<Component Id=`"InstalledLauncher`" Directory=`"INSTALLFOLDER`" Guid=`"*`"><File Source=`"$launcher`" Name=`"Run-Calendar.ps1`" KeyPath=`"yes`" /></Component>")
$payload = '<Wix xmlns="http://wixtoolset.org/schemas/v4/wxs"><Fragment><ComponentGroup Id="CalendarPayload">' + ($components -join "`n") + '</ComponentGroup></Fragment></Wix>'
Set-Content -LiteralPath (Join-Path $installerDirectory 'Payload.wxs') -Value $payload -Encoding utf8

$licenseText = (Get-Content LICENSE -Raw).Replace('\', '\\').Replace('{', '\{').Replace('}', '\}').Replace("`n", '\par ')
$licensePath = Join-Path $installerDirectory 'License.rtf'
Set-Content -LiteralPath $licensePath -Value ('{\rtf1\ansi ' + $licenseText + '}') -Encoding ascii

& $WixPath build installer/windows/Calendar.wxs (Join-Path $installerDirectory 'Payload.wxs') -arch x64 -ext WixToolset.UI.wixext -d "ProductVersion=$Version" -d "LicensePath=$licensePath" -o dist/calendar-windows-x64.msi
if ($LASTEXITCODE -ne 0) { throw 'WiX MSI build failed.' }
Write-Host "Created dist/calendar-windows-x64.msi ($Version)."
