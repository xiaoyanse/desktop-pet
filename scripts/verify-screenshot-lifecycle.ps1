param(
    [string]$Executable = (Join-Path $PSScriptRoot '..\target\release\desktop-meter.exe'),
    [ValidateRange(1,10)][int]$Trials = 3
)
$ErrorActionPreference = 'Stop'
$probeExecutable = (Resolve-Path -LiteralPath $Executable).Path
$probeRoot = Join-Path $PSScriptRoot ('..\evidence\screenshot-lifecycle-' + [Guid]::NewGuid().ToString('N'))
New-Item -ItemType Directory -Path $probeRoot | Out-Null
$probeRoot = (Resolve-Path -LiteralPath $probeRoot).Path
$results = @()
for ($trial=1; $trial -le $Trials; $trial++) {
    foreach ($strategy in @('hidden','blank')) {
        $probeLog = Join-Path $probeRoot "$strategy-$trial.jsonl"
        $probeArguments = @('--screenshot-lifecycle-probe',('"'+$probeLog+'"'))
        if ($strategy -eq 'blank') { $probeArguments += '--blank-root' }
        $probeProcess = Start-Process -FilePath $probeExecutable -ArgumentList $probeArguments -WindowStyle Hidden -PassThru
        if (-not $probeProcess.WaitForExit(9000)) {
            [void]$probeProcess.CloseMainWindow()
            [void]$probeProcess.WaitForExit(2000)
            throw "Owned probe PID $($probeProcess.Id) exceeded timeout; no unrelated process was stopped."
        }
        if ($probeProcess.ExitCode -ne 0) { throw "Probe exit code: $($probeProcess.ExitCode)" }
        $events = @(Get-Content -LiteralPath $probeLog | ForEach-Object { $_ | ConvertFrom-Json })
        $step = @($events | Where-Object event -eq 'delayed-step-reached')
        $finish = @($events | Where-Object event -eq 'complete')
        if ($step.Count -ne 1 -or $finish.Count -eq 0) { throw 'Missing lifecycle result.' }
        if ($strategy -eq 'hidden') {
            if (-not $step[0].watchdogFired) { throw 'Legacy stall no longer reproduces; reassess framework behavior.' }
        } else {
            $suspended = @($events | Where-Object { $_.event -eq 'own-framebuffer' -and $_.phase -eq 'suspended' })
            $restored = @($events | Where-Object { $_.event -eq 'own-framebuffer' -and $_.phase -eq 'restored' })
            if ($step[0].watchdogFired -or -not $step[0].windowVisible -or -not $step[0].mousePassthrough) { throw 'Transparent root did not advance in the intended native state.' }
            if ($suspended.Count -eq 0 -or $restored.Count -eq 0) { throw 'Missing own framebuffer samples.' }
            foreach ($sample in $suspended) { if ($sample.glError -ne 0 -or $sample.nonzeroAlphaPixels -ne 0) { throw 'Suspended frame was not fully transparent.' } }
            foreach ($sample in $restored) { if ($sample.glError -ne 0 -or $sample.nonzeroAlphaPixels -le 0) { throw 'Restored synthetic image was not painted.' } }
            foreach ($sample in $finish) { if ($sample.mousePassthrough) { throw 'Interactive state was not restored.' } }
        }
        $results += [pscustomobject]@{ Trial=$trial; Strategy=$strategy; PID=$probeProcess.Id; ElapsedMs=$step[0].elapsedMs; WatchdogFired=$step[0].watchdogFired; Log=$probeLog }
    }
}
$results | ConvertTo-Json -Depth 4 | Set-Content -LiteralPath (Join-Path $probeRoot 'summary.json') -Encoding utf8
$results | Format-Table Trial,Strategy,PID,ElapsedMs,WatchdogFired
Write-Output "Evidence: $probeRoot"
