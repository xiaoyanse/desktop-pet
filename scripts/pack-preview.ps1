param(
    [Parameter(Mandatory)][string]$SourceExecutable,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-fA-F]{64}$')][string]$ExpectedSha256,
    [Parameter(Mandatory)][ValidatePattern('^[0-9a-fA-F]{40}$')][string]$SourceCommit,
    [Parameter(Mandatory)][ValidatePattern('^[a-zA-Z0-9.-]+$')][string]$Version,
    [Parameter(Mandatory)][ValidateRange(1,100000)][int]$ChecksPassed,
    [Parameter(Mandatory)][ValidateRange(1,1000)][int]$PreviewPages
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$source = (Get-Item -LiteralPath $SourceExecutable).FullName
$actual = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
if ($actual -ne $ExpectedSha256) { throw '程序与指定验证版本不一致，拒绝打包。' }
$distribution = Join-Path $projectRoot 'dist'
$packageName = "DesktopMeter-$Version-win-x64"
$package = Join-Path $distribution $packageName
$archive = "$package.zip"
if ((Test-Path -LiteralPath $package) -or (Test-Path -LiteralPath $archive)) {
    throw '测试包已存在，拒绝覆盖；请使用新的版本名。'
}
$null = New-Item -ItemType Directory -Path $package
$executable = Join-Path $package 'desktop-meter.exe'
Copy-Item -LiteralPath $source -Destination $executable
if ((Get-FileHash -LiteralPath $executable -Algorithm SHA256).Hash -ne $ExpectedSha256) {
    throw '复制后的程序校验失败。'
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'docs\阶段测试版说明.md') -Destination (Join-Path $package 'README.md')
$launcher = "@echo off`r`nstart `"`" `"%~dp0desktop-meter.exe`" --settings`r`n"
[System.IO.File]::WriteAllText((Join-Path $package '启动设置.cmd'), $launcher, [System.Text.Encoding]::ASCII)
$manifest = [ordered]@{
    version = $Version
    sourceCommit = $SourceCommit
    createdAt = [DateTimeOffset]::Now.ToString('o')
    architecture = 'windows-x64'
    automaticChecksPassed = $ChecksPassed
    nativeSyntheticPreviewPages = $PreviewPages
    fieldAcceptanceComplete = $false
    uncommittedWorkIncluded = $false
    files = @(Get-ChildItem -LiteralPath $package -File | Sort-Object Name | ForEach-Object {
        [ordered]@{ name = $_.Name; bytes = $_.Length; sha256 = (Get-FileHash -LiteralPath $_.FullName -Algorithm SHA256).Hash }
    })
}
$manifest | ConvertTo-Json -Depth 5 | Set-Content -LiteralPath (Join-Path $package 'manifest.json') -Encoding utf8
Compress-Archive -LiteralPath $package -DestinationPath $archive -CompressionLevel Optimal
[ordered]@{
    package = $package
    archive = $archive
    archiveBytes = (Get-Item -LiteralPath $archive).Length
    archiveSha256 = (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash
    executableSha256 = $actual
} | ConvertTo-Json
