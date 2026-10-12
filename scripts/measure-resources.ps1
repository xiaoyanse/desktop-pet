param([int]$Seconds = 180, [string]$Output = "$PSScriptRoot/../evidence/overlay-resources.json",
    [long]$BudgetBytes = 100000000, [switch]$EnforceBudget)
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
        workingSetBytes=$meter.WorkingSet64; privateBytes=$meter.PrivateMemorySize64;
        lifetimePeakWorkingSetBytes=$meter.PeakWorkingSet64; lifetimePeakCommitBytes=$meter.PeakPagedMemorySize64;
        workingSetMB=[math]::Round($meter.WorkingSet64/1MB,2); privateMB=[math]::Round($meter.PrivateMemorySize64/1MB,2);
        handles=$meter.HandleCount; threads=$meter.Threads.Count }
    if ($watch.Elapsed.TotalSeconds -ge $Seconds) { break }
    Start-Sleep -Seconds 1
} while ($true)
$watch.Stop()
$last = $rows[-1]
$observedPrivatePeak = ($rows.privateBytes | Measure-Object -Maximum).Maximum
$lifetimePeakWorkingSet = ($rows.lifetimePeakWorkingSetBytes | Measure-Object -Maximum).Maximum
$lifetimePeakCommit = ($rows.lifetimePeakCommitBytes | Measure-Object -Maximum).Maximum
$report = [pscustomobject]@{ pid=$meter.Id; executable=$expected; durationSeconds=$watch.Elapsed.TotalSeconds;
    budgetBytes=$BudgetBytes; observedPrivatePeakBytes=$observedPrivatePeak;
    lifetimePeakWorkingSetBytes=$lifetimePeakWorkingSet; lifetimePeakCommitBytes=$lifetimePeakCommit;
    budgetPassed=($observedPrivatePeak -le $BudgetBytes -and $lifetimePeakWorkingSet -le $BudgetBytes -and $lifetimePeakCommit -le $BudgetBytes);
    measurementScope='One selected process and this observation; lifetime peaks include states before observation. Sampled private bytes do not prove every transient private-memory peak.';
    logicalProcessors=[Environment]::ProcessorCount;
    averageMachineCPUPercent=100*($last.cpu-$baseline)/$watch.Elapsed.TotalSeconds/[Environment]::ProcessorCount;
    workingSetMinMB=($rows.workingSetMB|Measure-Object -Minimum).Minimum;
    workingSetMaxMB=($rows.workingSetMB|Measure-Object -Maximum).Maximum; samples=$rows }
$parent = Split-Path $Output -Parent
New-Item -ItemType Directory -Force $parent | Out-Null
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $Output -Encoding utf8
$report | Select-Object pid,durationSeconds,averageMachineCPUPercent,workingSetMinMB,workingSetMaxMB
$report | Select-Object budgetBytes,observedPrivatePeakBytes,lifetimePeakWorkingSetBytes,lifetimePeakCommitBytes,budgetPassed
if ($EnforceBudget -and -not $report.budgetPassed) { throw '内存预算门禁失败；完整证据已保存，不得标记全状态达标。' }
