param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory)][int]$ChecksPassed,
    [Parameter(Mandatory)][int]$PreviewPages,
    [Parameter(Mandatory)][string]$BaselineDirectory
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$source = Join-Path $projectRoot 'target\release\desktop-meter.exe'
$baseline = (Get-Item -LiteralPath $BaselineDirectory).FullName
$baseManifest = Get-Content -LiteralPath (Join-Path $baseline 'manifest.json') -Raw | ConvertFrom-Json
$baseVersion = $baseManifest.version
if ($baseVersion -notmatch '^\d+\.\d+\.\d+$' -or [version]$baseVersion -ge [version]$Version) { throw '增量基线版本必须早于发布版本。' }
$baseExecutable = Join-Path $baseline 'desktop-meter.exe'
$baseExpected = $baseManifest.files | Where-Object name -eq 'desktop-meter.exe'
if (-not $baseExpected -or (Get-FileHash -LiteralPath $baseExecutable -Algorithm SHA256).Hash -ine $baseExpected.sha256) { throw '基线程序与原发布清单不符。' }
$compiler = 'C:\Program Files (x86)\Inno Setup 6\ISCC.exe'
if (-not (Test-Path -LiteralPath $compiler)) { throw '缺少 Inno Setup 6 安装包编译器。' }
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
$icon = Get-ChildItem -Path (Join-Path $projectRoot 'target\release\build\desktop-meter-*\out\desktop-meter.ico') -File |
    Sort-Object LastWriteTimeUtc -Descending | Select-Object -First 1
if (-not $icon) { throw '缺少构建生成的品牌安装图标。' }
$null = & (Join-Path $PSScriptRoot 'verify-windows-icons.ps1') -Executable $source -IconFile $icon.FullName
$commit = (& git -C $projectRoot rev-parse HEAD).Trim()
if ($LASTEXITCODE -ne 0 -or (& git -C $projectRoot status --porcelain)) { throw '发布打包需要已验证并提交的干净工作区。' }
if ((Get-Content -LiteralPath (Join-Path $projectRoot 'Cargo.toml') -Raw) -notmatch ('(?m)^version = "' + [regex]::Escape($Version) + '"')) { throw '版本参数与 Cargo.toml 不一致。' }
& (Join-Path $PSScriptRoot 'pack-preview.ps1') -SourceExecutable $source -ExpectedSha256 $sourceHash -SourceCommit $commit -Version $Version -ChecksPassed $ChecksPassed -PreviewPages $PreviewPages
$archive = Join-Path $projectRoot "dist\DesktopMeter-$Version-win-x64.zip"
$manifest = [ordered]@{
    schema = 1
    version = $Version
    file_name = [IO.Path]::GetFileName($archive)
    size = (Get-Item -LiteralPath $archive).Length
    sha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash.ToLowerInvariant()
    executable_sha256 = $sourceHash.ToLowerInvariant()
}
if ($manifest.size -gt 100MB) { throw '发布包超过更新客户端上限。' }
$package = Join-Path $projectRoot "dist\DesktopMeter-$Version-win-x64"
$deltaName = "DesktopMeter-$Version-from-$baseVersion-delta-win-x64"
$deltaDirectory = Join-Path $projectRoot "dist\$deltaName"
$deltaArchive = "$deltaDirectory.zip"
if ((Test-Path -LiteralPath $deltaDirectory) -or (Test-Path -LiteralPath $deltaArchive)) { throw '增量暂存已存在，拒绝覆盖。' }
$null = New-Item -ItemType Directory -Path $deltaDirectory
$patch = Join-Path $deltaDirectory 'desktop-meter.exe.patch'
$process = Start-Process -FilePath $source -ArgumentList @('--make-update-delta', ('"' + $baseExecutable + '"'), ('"' + (Join-Path $package 'desktop-meter.exe') + '"'), ('"' + $patch + '"')) -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0) { throw 'Rust 差分生成失败。' }
foreach ($name in @('README.md','启动设置.cmd','manifest.json')) { Copy-Item -LiteralPath (Join-Path $package $name) -Destination (Join-Path $deltaDirectory $name) }
$reconstructed = Join-Path $projectRoot "dist\delta-reconstructed-$Version.exe"
if (Test-Path -LiteralPath $reconstructed) { throw '重建验证文件已存在，拒绝覆盖。' }
$process = Start-Process -FilePath $source -ArgumentList @('--apply-update-delta', ('"' + $baseExecutable + '"'), ('"' + $patch + '"'), ('"' + $reconstructed + '"')) -WindowStyle Hidden -PassThru -Wait
if ($process.ExitCode -ne 0 -or (Get-FileHash -LiteralPath $reconstructed -Algorithm SHA256).Hash -ine $sourceHash) { throw '差分重建不是发布程序，拒绝发布。' }
Remove-Item -LiteralPath $reconstructed
Compress-Archive -LiteralPath $deltaDirectory -DestinationPath $deltaArchive -CompressionLevel Optimal
$manifest.delta = [ordered]@{
    base_version = $baseVersion
    base_executable_sha256 = (Get-FileHash -LiteralPath $baseExecutable -Algorithm SHA256).Hash.ToLowerInvariant()
    executable_size = (Get-Item -LiteralPath $source).Length
    file_name = [IO.Path]::GetFileName($deltaArchive)
    size = (Get-Item -LiteralPath $deltaArchive).Length
    sha256 = (Get-FileHash -LiteralPath $deltaArchive -Algorithm SHA256).Hash.ToLowerInvariant()
}
$setup = Join-Path $projectRoot "dist\DesktopMeter-$Version-setup-win-x64.exe"
if (Test-Path -LiteralPath $setup) { throw '安装包已存在，拒绝覆盖。' }
& $compiler "/DReleaseVersion=$Version" "/DPackageDir=$package" "/DIconPath=$($icon.FullName)" "/DOutputPath=$(Join-Path $projectRoot 'dist')" (Join-Path $projectRoot 'packaging\windows\desktop-meter.iss')
if ($LASTEXITCODE -ne 0 -or -not (Test-Path -LiteralPath $setup)) { throw 'Windows 安装包构建失败。' }
$null = & (Join-Path $PSScriptRoot 'verify-windows-icons.ps1') -Executable $source -IconFile $icon.FullName -Installer $setup
$manifest.installer = [ordered]@{
    file_name = [IO.Path]::GetFileName($setup)
    size = (Get-Item -LiteralPath $setup).Length
    sha256 = (Get-FileHash -LiteralPath $setup -Algorithm SHA256).Hash.ToLowerInvariant()
}
$manifestPath = Join-Path $projectRoot "dist\update-manifest-$Version.json"
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath $manifestPath -Encoding utf8NoBOM
[pscustomobject]@{Installer=$setup; FullArchive=$archive; DeltaArchive=$deltaArchive; UpdateManifest=$manifestPath; SourceCommit=$commit} | ConvertTo-Json
