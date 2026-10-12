param([ValidateRange(1,86400)][int]$Seconds = 180, [string]$Output = "$PSScriptRoot/../evidence/overlay-resources.json",
    [ValidateRange(1,[long]::MaxValue)][long]$BudgetBytes = 100000000, [switch]$EnforceBudget)
$ErrorActionPreference = 'Stop'
$expected = (Resolve-Path "$PSScriptRoot/../target/release/desktop-meter.exe").Path
$candidates = @(Get-Process desktop-meter -ErrorAction SilentlyContinue | Where-Object { $_.Path -eq $expected })
if ($candidates.Count -ne 1) { throw '需要唯一运行中的本项目 release 监视器。' }
$meter = $candidates[0]
$baseline = $meter.TotalProcessorTime.TotalSeconds
$watch = [Diagnostics.Stopwatch]::StartNew()
$rows = @()
$terminationReason = $null
$measurementComplete = $false
do {
    try {
        $meter.Refresh()
        if ($meter.HasExited) {
            $terminationReason = '监视器在采样期间退出。'
            break
        }
        $rows += [pscustomobject]@{ elapsed=$watch.Elapsed.TotalSeconds; cpu=$meter.TotalProcessorTime.TotalSeconds;
            workingSetBytes=$meter.WorkingSet64; privateBytes=$meter.PrivateMemorySize64;
            lifetimePeakWorkingSetBytes=$meter.PeakWorkingSet64; lifetimePeakCommitBytes=$meter.PeakPagedMemorySize64;
            workingSetMB=[math]::Round($meter.WorkingSet64/1MB,2); privateMB=[math]::Round($meter.PrivateMemorySize64/1MB,2);
            handles=$meter.HandleCount; threads=$meter.Threads.Count }
    } catch {
        $terminationReason = "进程采样失败：$($_.Exception.Message)"
        break
    }
    if ($watch.Elapsed.TotalSeconds -ge $Seconds) {
        $measurementComplete = $true
        break
    }
    Start-Sleep -Seconds 1
} while ($true)
$watch.Stop()
$last = if ($rows.Count -gt 0) { $rows[-1] } else { $null }
$observedPrivatePeak = ($rows.privateBytes | Measure-Object -Maximum).Maximum
$lifetimePeakWorkingSet = ($rows.lifetimePeakWorkingSetBytes | Measure-Object -Maximum).Maximum
$lifetimePeakCommit = ($rows.lifetimePeakCommitBytes | Measure-Object -Maximum).Maximum
$report = [pscustomobject]@{ pid=$meter.Id; executable=$expected; durationSeconds=$watch.Elapsed.TotalSeconds;
    measurementComplete=$measurementComplete; terminationReason=$terminationReason;
    budgetBytes=$BudgetBytes; observedPrivatePeakBytes=$observedPrivatePeak;
    lifetimePeakWorkingSetBytes=$lifetimePeakWorkingSet; lifetimePeakCommitBytes=$lifetimePeakCommit;
    budgetPassed=($measurementComplete -and $rows.Count -gt 0 -and $observedPrivatePeak -le $BudgetBytes -and $lifetimePeakWorkingSet -le $BudgetBytes -and $lifetimePeakCommit -le $BudgetBytes);
    measurementScope='One selected process and this observation; lifetime peaks include states before observation. Sampled private bytes do not prove every transient private-memory peak.';
    logicalProcessors=[Environment]::ProcessorCount;
    averageMachineCPUPercent=$(if ($null -ne $last) { 100*($last.cpu-$baseline)/$watch.Elapsed.TotalSeconds/[Environment]::ProcessorCount } else { $null });
    workingSetMinMB=($rows.workingSetMB|Measure-Object -Minimum).Minimum;
    workingSetMaxMB=($rows.workingSetMB|Measure-Object -Maximum).Maximum; samples=$rows }
$parent = Split-Path $Output -Parent
New-Item -ItemType Directory -Force $parent | Out-Null
$report | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $Output -Encoding utf8
$report | Select-Object pid,durationSeconds,averageMachineCPUPercent,workingSetMinMB,workingSetMaxMB
$report | Select-Object budgetBytes,observedPrivatePeakBytes,lifetimePeakWorkingSetBytes,lifetimePeakCommitBytes,budgetPassed
if (-not $measurementComplete) { throw '资源采样未完成；已保存中断证据，不能标记内存预算通过。' }
if ($EnforceBudget -and -not $report.budgetPassed) { throw '内存预算门禁失败；完整证据已保存，不得标记全状态达标。' }
