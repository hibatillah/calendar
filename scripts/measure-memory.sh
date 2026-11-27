#!/usr/bin/env bash
# Measure working set and private bytes of a running or freshly launched process.
#
# Usage:
#   scripts/measure-memory.sh [options] <process-name|PID>
#   scripts/measure-memory.sh [options] --launch <executable> [-- <arguments>...]
#
# Options:
#   --count N     number of runs; the median is reported (default 3)
#   --delay S     seconds to wait after launch (or before each sample of a running process) (default 60)
#   --label TEXT  state name printed with the result, e.g. "tray idle", "window open idle"
#   --launch EXE  start EXE for every run, sample it after --delay seconds, then stop it
#   --keep        with --launch, leave the process running after the last sample
#
# States (set them up, then run this script against the process):
#   tray process idle        --launch <tray-or-app.exe>            (no window ever opened)
#   UI window open idle      --launch <app.exe>                    (window opens at start)
#   UI window open after sync  run against the PID after the sync finished, with --count and --delay 60
#   UI process after close   --launch <app.exe> -- --close-after-secs 10   (or close the window by hand
#                            and run against the PID; the process must still be alive when sampled)
#
# Memory columns are MiB. Working set = WorkingSet64. Private bytes = PrivateMemorySize64 (committed
# private memory). When a process name matches several processes, their values are summed.
set -euo pipefail

count=3
delay=60
label=""
launch=""
keep=0
target=""
launch_arguments=()

while [[ $# -gt 0 ]]; do
    case "$1" in
        --count) count="$2"; shift 2 ;;
        --delay) delay="$2"; shift 2 ;;
        --label) label="$2"; shift 2 ;;
        --launch) launch="$2"; shift 2 ;;
        --keep) keep=1; shift ;;
        --) shift; launch_arguments=("$@"); break ;;
        -h|--help) sed -n '2,24p' "$0"; exit 0 ;;
        -*) echo "unknown option: $1" >&2; exit 2 ;;
        *) target="$1"; shift ;;
    esac
done

if [[ -z "$launch" && -z "$target" ]]; then
    echo "give a process name, a PID or --launch <executable>" >&2
    exit 2
fi

if [[ -n "$launch" ]] && command -v cygpath >/dev/null 2>&1; then
    launch="$(cygpath -w "$launch")"
fi

export MEASURE_COUNT="$count"
export MEASURE_DELAY="$delay"
export MEASURE_LABEL="$label"
export MEASURE_LAUNCH="$launch"
export MEASURE_KEEP="$keep"
export MEASURE_TARGET="$target"
export MEASURE_ARGUMENTS="${launch_arguments[*]:-}"

read -r -d '' powershell_script <<'POWERSHELL' || true
$ErrorActionPreference = 'Stop'
$ProgressPreference = 'SilentlyContinue'
$count = [int]$env:MEASURE_COUNT
$delay = [int]$env:MEASURE_DELAY
$launch = $env:MEASURE_LAUNCH
$keep = $env:MEASURE_KEEP -eq '1'
$target = $env:MEASURE_TARGET
$label = $env:MEASURE_LABEL
$mib = 1MB

function Get-Median([double[]]$values) {
    $sorted = $values | Sort-Object
    $middle = [int][math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2 -eq 1) { return $sorted[$middle] }
    return ($sorted[$middle - 1] + $sorted[$middle]) / 2
}

function Get-Sample($processes) {
    $workingSet = 0.0
    $privateBytes = 0.0
    foreach ($process in $processes) {
        $process.Refresh()
        $workingSet += $process.WorkingSet64
        $privateBytes += $process.PrivateMemorySize64
    }
    return [pscustomobject]@{ WorkingSetMiB = $workingSet / $mib; PrivateMiB = $privateBytes / $mib }
}

$samples = @()
for ($run = 1; $run -le $count; $run++) {
    $started = $null
    if ($launch) {
        $startArguments = @{ FilePath = $launch; PassThru = $true }
        if ($env:MEASURE_ARGUMENTS) { $startArguments.ArgumentList = $env:MEASURE_ARGUMENTS }
        $started = Start-Process @startArguments
        Start-Sleep -Seconds $delay
        $processes = @(Get-Process -Id $started.Id -ErrorAction Stop)
    } else {
        if ($run -gt 1 -or $delay -gt 0) { Start-Sleep -Seconds $delay }
        if ($target -match '^\d+$') {
            $processes = @(Get-Process -Id ([int]$target) -ErrorAction Stop)
        } else {
            $processes = @(Get-Process -Name ($target -replace '\.exe$', '') -ErrorAction Stop)
        }
    }
    $sample = Get-Sample $processes
    $samples += $sample
    '{0} run {1}: processes={2} working_set={3:N1} MiB private_bytes={4:N1} MiB' -f $label, $run, $processes.Count, $sample.WorkingSetMiB, $sample.PrivateMiB
    if ($started -and -not ($keep -and $run -eq $count)) {
        Stop-Process -Id $started.Id -Force -ErrorAction SilentlyContinue
        Start-Sleep -Seconds 2
    }
}
'{0} median of {1}: working_set={2:N1} MiB private_bytes={3:N1} MiB' -f $label, $count,
    (Get-Median ($samples | ForEach-Object { $_.WorkingSetMiB })),
    (Get-Median ($samples | ForEach-Object { $_.PrivateMiB }))
POWERSHELL

encoded_script="$(printf '%s' "$powershell_script" | iconv -f UTF-8 -t UTF-16LE | base64 -w0)"
powershell.exe -NoProfile -NonInteractive -EncodedCommand "$encoded_script"
