$CargoArguments = $args
$ErrorActionPreference = 'Stop'

Set-Location (Split-Path (Split-Path $PSScriptRoot -Parent) -Parent)

if ($IsWindows) {
    $visualStudioLocator = Join-Path ${env:ProgramFiles(x86)} 'Microsoft Visual Studio/Installer/vswhere.exe'
    $visualStudioPath = & $visualStudioLocator -latest -products '*' -requires Microsoft.VisualStudio.Component.VC.Tools.x86.x64 -property installationPath

    if (-not $visualStudioPath) { throw 'Install Visual Studio C++ Build Tools and a Windows SDK.' }

    Import-Module (Join-Path $visualStudioPath 'Common7/Tools/Microsoft.VisualStudio.DevShell.dll')
    Enter-VsDevShell -VsInstallPath $visualStudioPath -SkipAutomaticLocation -DevCmdArguments '-arch=x64 -host_arch=x64' | Out-Null
}
if ($IsWindows) { & cargo +stable-x86_64-pc-windows-msvc @CargoArguments }
else { & cargo @CargoArguments }

exit $LASTEXITCODE
