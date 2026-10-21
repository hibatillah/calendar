$ErrorActionPreference = 'Stop'

. (Join-Path $PSScriptRoot 'dev-desktop.ps1') -LibraryOnly

$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('calendar-watch-test-' + [guid]::NewGuid())

try {
    $null = New-Item -ItemType Directory -Path (Join-Path $testRoot 'apps/desktop/src') -Force
    $null = New-Item -ItemType Directory -Path (Join-Path $testRoot '.local') -Force
    $originalSnapshot = Get-SourceSnapshot $testRoot

    Set-Content -LiteralPath (Join-Path $testRoot '.local/cache') -Value 'ignored'

    if ((Get-SourceSnapshot $testRoot) -ne $originalSnapshot) { throw 'Cache changes triggered a rebuild.' }

    $sourceFile = Join-Path $testRoot 'apps/desktop/src/main.rs'

    Set-Content -LiteralPath $sourceFile -Value 'fn main() {}'

    $addedSnapshot = Get-SourceSnapshot $testRoot

    if ($addedSnapshot -eq $originalSnapshot) { throw 'New source was not detected.' }

    Set-Content -LiteralPath $sourceFile -Value 'fn main() { println!("changed"); }'

    if ((Get-SourceSnapshot $testRoot) -eq $addedSnapshot) { throw 'Changed source was not detected.' }

    Remove-Item -LiteralPath $sourceFile

    if ((Get-SourceSnapshot $testRoot) -ne $originalSnapshot) { throw 'Deleted source was not detected.' }

    $extension = if ($IsWindows) { '.exe' } else { '' }

    if (-not (Test-OwnedExecutable (Join-Path $testRoot "target/debug/calendar-desktop$extension") $testRoot 'calendar-desktop')) { throw 'Owned build rejected.' }
    if (-not (Test-OwnedExecutable (Join-Path $testRoot ".local/dev-desktop/generation/calendar-desktop$extension") $testRoot 'calendar-desktop')) { throw 'Owned runtime rejected.' }
    if (Test-OwnedExecutable (Join-Path "$testRoot-other" "target/debug/calendar-desktop$extension") $testRoot 'calendar-desktop') { throw 'Unrelated checkout accepted.' }

    Write-Host 'Desktop watcher checks passed.'
} finally {
    $resolvedRoot = [IO.Path]::GetFullPath($testRoot)
    $temporaryRoot = [IO.Path]::GetFullPath([IO.Path]::GetTempPath())

    if ($resolvedRoot.StartsWith($temporaryRoot) -and (Split-Path $resolvedRoot -Leaf).StartsWith('calendar-watch-test-')) {
        Remove-Item -LiteralPath $resolvedRoot -Recurse -Force
    }
}
