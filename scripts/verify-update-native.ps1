param([Parameter(Mandatory)][string]$HelperExecutable)
$ErrorActionPreference = 'Stop'
$helperSource = (Get-Item -LiteralPath $HelperExecutable).FullName
if (-not $helperSource.StartsWith('E:\', [StringComparison]::OrdinalIgnoreCase)) { throw '本机验收仅使用 E 盘程序与隔离数据。' }
$testRoot = 'E:\Temp\desktop-meter-update-native-' + [DateTime]::Now.ToString('yyyyMMdd-HHmmss')
if (Test-Path -LiteralPath $testRoot) { throw '测试目录已存在。' }
$null = New-Item -ItemType Directory -Path $testRoot
$results = @()
foreach ($case in @('success', 'rollback')) {
    $nonce = [DateTime]::UtcNow.Ticks.ToString()
    $stage = 'E:\DesktopMeterData\updates\stage-' + $nonce
    $caseRoot = Join-Path $testRoot $case
    $target = Join-Path $caseRoot 'application'
    $payload = Join-Path $stage 'payload'
    $null = New-Item -ItemType Directory -Path $target -Force
    $null = New-Item -ItemType Directory -Path $payload
    $counter = Join-Path $caseRoot 'original-starts.txt'
    $oldSource = @"
fn main() {
    let path = r##"$counter"##;
    let count: u32 = std::fs::read_to_string(path).ok().and_then(|s| s.parse().ok()).unwrap_or(0);
    std::fs::write(path, (count + 1).to_string()).unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
}
"@
    $newSource = if ($case -eq 'success') {
@"
fn main() {
    let args: Vec<String> = std::env::args().collect();
    assert_eq!(args.get(1).map(String::as_str), Some("--update-ack"));
    let stage = std::path::Path::new(&args[2]).parent().unwrap();
    std::fs::write(stage.join("healthy.txt"), "$nonce`:0.2.0").unwrap();
    std::thread::sleep(std::time::Duration::from_millis(500));
}
"@
    } else { 'fn main() { std::process::exit(1); }' }
    $oldSourceFile = Join-Path $caseRoot 'old.rs'
    $newSourceFile = Join-Path $caseRoot 'new.rs'
    Set-Content -LiteralPath $oldSourceFile -Value $oldSource -Encoding utf8NoBOM
    Set-Content -LiteralPath $newSourceFile -Value $newSource -Encoding utf8NoBOM
    & rustc --crate-name original_fixture $oldSourceFile -o (Join-Path $target 'desktop-meter.exe')
    if ($LASTEXITCODE -ne 0) { throw '旧版隔离程序编译失败。' }
    & rustc --crate-name candidate_fixture $newSourceFile -o (Join-Path $payload 'desktop-meter.exe')
    if ($LASTEXITCODE -ne 0) { throw '新版隔离程序编译失败。' }
    Get-ChildItem -LiteralPath $target -Filter '*.pdb' | ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination $caseRoot }
    Get-ChildItem -LiteralPath $payload -Filter '*.pdb' | ForEach-Object { Move-Item -LiteralPath $_.FullName -Destination (Join-Path $caseRoot ('candidate-' + $_.Name)) }
    foreach ($folder in @($target, $payload)) {
        Set-Content -LiteralPath (Join-Path $folder 'manifest.json') -Value '{"version":"0.2.0"}' -Encoding utf8NoBOM
        Set-Content -LiteralPath (Join-Path $folder 'README.md') -Value 'Synthetic updater fixture; contains no user data.' -Encoding utf8NoBOM
        Set-Content -LiteralPath (Join-Path $folder '启动设置.cmd') -Value '@echo off' -Encoding ascii
    }
    $oldHash = (Get-FileHash -LiteralPath (Join-Path $target 'desktop-meter.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    $newHash = (Get-FileHash -LiteralPath (Join-Path $payload 'desktop-meter.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    $parent = Start-Process -FilePath (Join-Path $target 'desktop-meter.exe') -WorkingDirectory $target -WindowStyle Hidden -PassThru
    $job = @{target=[IO.Path]::GetFullPath($target);old_hash=$oldHash;version='0.2.0';new_hash=$newHash;nonce=$nonce;parent=$parent.Id}
    $jobPath = Join-Path $stage 'job.json'
    $job | ConvertTo-Json | Set-Content -LiteralPath $jobPath -Encoding utf8NoBOM
    $helper = Join-Path $stage 'helper.exe'
    Copy-Item -LiteralPath $helperSource -Destination $helper
    # Deliberately inherit the application directory, matching Explorer/shortcut startup.
    # The production helper must detach itself before attempting directory replacement.
    $helperProcess = Start-Process -FilePath $helper -WorkingDirectory $target -ArgumentList @('--apply-update', ('"' + $jobPath + '"'), $parent.Id, '--quiet-helper') -WindowStyle Hidden -PassThru
    if (-not $helperProcess.WaitForExit(45000)) { $helperProcess.Kill(); throw '隔离更新辅助程序超时。' }
    Start-Sleep -Milliseconds 700
    $actualHash = (Get-FileHash -LiteralPath (Join-Path $target 'desktop-meter.exe') -Algorithm SHA256).Hash.ToLowerInvariant()
    $expectedHash = if ($case -eq 'success') { $newHash } else { $oldHash }
    $resultPath = Join-Path $stage 'result.txt'
    $valid = ($actualHash -eq $expectedHash) -and (Test-Path -LiteralPath $resultPath)
    if ($case -eq 'rollback') { $valid = $valid -and ((Get-Content -LiteralPath $counter -Raw).Trim() -eq '2') }
    if (-not $valid) { throw "隔离更新 $case 验证失败，保留 $stage 与 $testRoot 取证。" }
    $results += [ordered]@{case=$case;passed=$valid;helperExitCode=$helperProcess.ExitCode;initialHelperWorkingDirectory=$target;stage=$stage;target=$target;result=(Get-Content -LiteralPath $resultPath -Raw).Trim();currentHash=$actualHash}
}
$report = [ordered]@{helperSha256=(Get-FileHash -LiteralPath $helperSource -Algorithm SHA256).Hash;fixtures=$results;normalUserProcessesTouched=$false;scope='真实文件替换、辅助进程、新版确认与失败回退；候选子程序为隔离 Rust 程序，不等同于完整 GUI 更新现场验收。'}
$report | ConvertTo-Json -Depth 6 | Set-Content -LiteralPath (Join-Path $testRoot 'report.json') -Encoding utf8NoBOM
$report | ConvertTo-Json -Depth 6
