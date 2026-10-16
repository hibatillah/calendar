[CmdletBinding()]
param(
    [switch]$Once,
    [switch]$LibraryOnly,
    [int]$DebounceMilliseconds = 800
)

$ErrorActionPreference = 'Stop'
$repositoryRoot = Split-Path $PSScriptRoot -Parent

function Get-SourceSnapshot {
    param([string]$Root)
    $sourcePaths = @('apps/desktop', 'apps/backend', 'crates', 'assets', '.cargo')
    $entries = foreach ($relativePath in $sourcePaths) {
        $sourcePath = Join-Path $Root $relativePath

        if (Test-Path -LiteralPath $sourcePath) {
            Get-ChildItem -LiteralPath $sourcePath -File -Recurse | ForEach-Object {
                '{0}|{1}|{2}' -f $_.FullName, $_.Length, $_.LastWriteTimeUtc.Ticks
            }
        }
    }

    foreach ($fileName in @('Cargo.toml', 'Cargo.lock')) {
        $filePath = Join-Path $Root $fileName

        if (Test-Path -LiteralPath $filePath) {
            $file = Get-Item -LiteralPath $filePath
            $entries += '{0}|{1}|{2}' -f $file.FullName, $file.Length, $file.LastWriteTimeUtc.Ticks
        }
    }

    return ($entries | Sort-Object) -join "`n"
}

function Test-OwnedExecutable {
    param([string]$ExecutablePath, [string]$Root, [string]$ApplicationName)
    if (-not $ExecutablePath) { return $false }

    $extension = if ($IsWindows) { '.exe' } else { '' }
    $fullPath = [IO.Path]::GetFullPath($ExecutablePath)
    $buildPath = [IO.Path]::GetFullPath((Join-Path $Root "target/debug/$ApplicationName$extension"))
    $runRoot = [IO.Path]::GetFullPath((Join-Path $Root '.local/dev-desktop')) + [IO.Path]::DirectorySeparatorChar
    $comparison = if ($IsWindows) { [StringComparison]::OrdinalIgnoreCase } else { [StringComparison]::Ordinal }

    return $fullPath.Equals($buildPath, $comparison) -or (
        $fullPath.StartsWith($runRoot, $comparison) -and
        [IO.Path]::GetFileName($fullPath) -eq "$ApplicationName$extension"
    )
}

function Start-BuiltApplication {
    param([string]$ApplicationName, [string]$BuildPath)
    $generationDirectory = Join-Path $repositoryRoot ('.local/dev-desktop/' + [guid]::NewGuid())
    $null = New-Item -ItemType Directory -Path $generationDirectory -Force
    $executablePath = Join-Path $generationDirectory ([IO.Path]::GetFileName($BuildPath))

    Copy-Item -LiteralPath $BuildPath -Destination $executablePath
    # Copy before stopping: a failed build/copy must not close the current application.
    Get-Process -Name $ApplicationName -ErrorAction SilentlyContinue | Where-Object {
        Test-OwnedExecutable $_.Path $repositoryRoot $ApplicationName
    } | ForEach-Object { Stop-Process -Id $_.Id -Force }

    $startArguments = @{ FilePath = $executablePath; WorkingDirectory = $repositoryRoot; PassThru = $true }

    if ($IsWindows) { $startArguments.WindowStyle = 'Hidden' }

    $applicationProcess = Start-Process @startArguments

    Write-Host "[dev] Started $ApplicationName (process $($applicationProcess.Id))."
}

if ($LibraryOnly) { return }
if ($DebounceMilliseconds -lt 100) { throw 'DebounceMilliseconds must be at least 100.' }

Set-Location $repositoryRoot

$null = New-Item -ItemType Directory -Path (Join-Path $repositoryRoot '.local') -Force
$watchLock = $null

try {
    try {
        $watchLock = [IO.File]::Open((Join-Path $repositoryRoot '.local/dev-desktop.lock'), 'OpenOrCreate', 'ReadWrite', 'None')
    } catch { throw 'Another desktop watcher is already running for this repository.' }

    $extension = if ($IsWindows) { '.exe' } else { '' }
    $applicationNames = @('calendar-backend', 'calendar-desktop')
    $runningHashes = @{}
    # One-time handoff releases Windows executable locks without throwing away warm Cargo artifacts.
    foreach ($applicationName in $applicationNames) {
        $buildPath = Join-Path $repositoryRoot "target/debug/$applicationName$extension"
        $ownedProcesses = @(Get-Process -Name $applicationName -ErrorAction SilentlyContinue | Where-Object {
            Test-OwnedExecutable $_.Path $repositoryRoot $applicationName
        })

        if ($ownedProcesses.Count -gt 0 -and (Test-Path -LiteralPath $buildPath)) {
            if (@($ownedProcesses | Where-Object { $_.Path -eq $buildPath }).Count -gt 0) {
                Start-BuiltApplication $applicationName $buildPath

                $runningHashes[$applicationName] = (Get-FileHash -LiteralPath $buildPath).Hash
            } else {
                $runningHashes[$applicationName] = (Get-FileHash -LiteralPath $ownedProcesses[0].Path).Hash
            }
        }
    }

    Write-Host '[dev] Watching Rust sources, manifests and bundled assets. Ctrl+C stops watching; apps stay open.'
    Write-Host '[dev] This rebuilds/restarts GPUI; unsaved editor and window state reset on restart.'

    $sourceSnapshot = Get-SourceSnapshot $repositoryRoot

    while ($true) {
        Write-Host '[dev] Building…'
        & pwsh -NoProfile -File (Join-Path $PSScriptRoot 'cargo.ps1') build --workspace

        $buildExitCode = $LASTEXITCODE

        if ($buildExitCode -eq 0) {
            foreach ($applicationName in $applicationNames) {
                $buildPath = Join-Path $repositoryRoot "target/debug/$applicationName$extension"
                $buildHash = (Get-FileHash -LiteralPath $buildPath).Hash
                $ownedProcesses = @(Get-Process -Name $applicationName -ErrorAction SilentlyContinue | Where-Object {
                    Test-OwnedExecutable $_.Path $repositoryRoot $applicationName
                })

                if ($runningHashes[$applicationName] -ne $buildHash -or $ownedProcesses.Count -eq 0) {
                    Start-BuiltApplication $applicationName $buildPath

                    $runningHashes[$applicationName] = $buildHash
                }
            }

            Write-Host '[dev] Ready. Only changed executables were restarted.'
        } else {
            Write-Warning 'Build failed. The last running applications stay open; fix the error and save again.'
        }
        if ($Once) { exit $buildExitCode }

        # Snapshot before building so edits made during compilation trigger another build.
        do {
            Start-Sleep -Milliseconds 250

            $nextSnapshot = Get-SourceSnapshot $repositoryRoot
        } while ($nextSnapshot -eq $sourceSnapshot)
        do {
            $sourceSnapshot = $nextSnapshot

            Start-Sleep -Milliseconds $DebounceMilliseconds

            $nextSnapshot = Get-SourceSnapshot $repositoryRoot
        } while ($nextSnapshot -ne $sourceSnapshot)
    }
} finally {
    if ($watchLock) { $watchLock.Dispose() }
}
