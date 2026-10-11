param([int]$Seconds = 180, [string]$Output = "$PSScriptRoot/../evidence/overlay-resources.json")
$ErrorActionPreference = 'Stop'
$expected = (Resolve-Path "$PSScriptRoot/../target/release/desktop-meter.exe").Path
$candidates = @(Get-Process desktop-meter -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $expected })
if ($candidates.Count -ne 1) { throw '需要唯一运行中的本项目 release 监视器。' }
$meter = $candidates[0]
$baseline = $meter.TotalProcessorTime.TotalSeconds
$watch = [Diagnostics.Stopwatch]::StartNew()
$rows = @()
do {
    $meter.Refresh()
    if ($meter.HasExited) { throw '监视器在采样期间退出。' }
    $rows += [pscustomobject]@{ elapsed=$watch.Elapsed.TotalSeconds; cpu=$meter.TotalProcessorTime.TotalSeconds;
        workingSetMB=[math]::Round($meter.WorkingSet64/1MB,2); privateMB=[math]::Round($meter.PrivateMemorySize64/1MB,2);
        handles=$meter.HandleCount; threads=$meter.Threads.Count }
    if ($watch.Elapsed.TotalSeconds -ge $Seconds) { break }
    Start-Sleep -Seconds 10
} while ($true)
$watch.Stop()
$last = $rows[-1]
$report = [pscustomobject]@{ pid=$meter.Id; executable=$expected; durationSeconds=$watch.Elapsed.TotalSeconds;
    logicalProcessors=[Environment]::ProcessorCount;
    averageMachineCPUPercent=100*($last.cpu-$baseline)/$watch.Elapsed.TotalSeconds/[Environment]::ProcessorCount;
    workingSetMinMB=($rows.workingSetMB|Measure-Object -Minimum).Minimum;
    workingSetMaxMB=($rows.workingSetMB|Measure-Object -Maximum).Maximum; samples=$rows }
$parent = Split-Path $Output -Parent
New-Item -ItemType Directory -Force $parent | Out-Null
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $Output -Encoding utf8
$report | Select-Object pid,durationSeconds,averageMachineCPUPercent,workingSetMinMB,workingSetMaxMB
