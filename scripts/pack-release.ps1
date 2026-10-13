param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory)][int]$ChecksPassed,
    [Parameter(Mandatory)][int]$PreviewPages
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$source = Join-Path $projectRoot 'target\release\desktop-meter.exe'
$sourceHash = (Get-FileHash -LiteralPath $source -Algorithm SHA256).Hash
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
$manifestPath = Join-Path $projectRoot "dist\update-manifest-$Version.json"
$manifest | ConvertTo-Json | Set-Content -LiteralPath $manifestPath -Encoding utf8NoBOM
[pscustomobject]@{Archive=$archive; UpdateManifest=$manifestPath; SourceCommit=$commit} | ConvertTo-Json
