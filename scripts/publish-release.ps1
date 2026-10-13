param(
    [Parameter(Mandatory)][ValidatePattern('^\d+\.\d+\.\d+$')][string]$Version,
    [Parameter(Mandatory)][string]$NotesFile
)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path $PSScriptRoot -Parent
$gh = 'C:\Program Files\GitHub CLI\gh.exe'
$repository = 'xiaoyanse/desktop-pet'
$tag = "v$Version"
$archive = Join-Path $projectRoot "dist\DesktopMeter-$Version-win-x64.zip"
$manifestPath = Join-Path $projectRoot "dist\update-manifest-$Version.json"
$manifest = Get-Content -LiteralPath $manifestPath -Raw | ConvertFrom-Json
if ($manifest.schema -ne 1 -or $manifest.version -ne $Version -or $manifest.file_name -ne [IO.Path]::GetFileName($archive) -or $manifest.size -ne (Get-Item -LiteralPath $archive).Length -or $manifest.sha256 -ine (Get-FileHash -LiteralPath $archive -Algorithm SHA256).Hash) { throw '发布包与更新清单不一致。' }
if (& git -C $projectRoot status --porcelain) { throw '发布需要干净工作区。' }
$notes = Get-Content -LiteralPath $NotesFile -Raw
Import-Module 'E:\CodexData\PowerShell\Modules\Codex.Gitee\Codex.Gitee.psd1'
$existingGitee = Invoke-GiteeApi -Path "/repos/$repository/releases"
if ($existingGitee | Where-Object { $_.tag_name -eq $tag }) { throw 'Gitee 版本已存在，拒绝覆盖。' }
$githubReleaseList = & $gh api "repos/$repository/releases"
if ($LASTEXITCODE -ne 0) { throw '无法检查 GitHub 发布记录。' }
if (($githubReleaseList | ConvertFrom-Json) | Where-Object { $_.tag_name -eq $tag }) { throw 'GitHub 版本已存在，拒绝覆盖。' }
$sourceCommit = (Get-Content -LiteralPath (Join-Path $projectRoot "dist\DesktopMeter-$Version-win-x64\manifest.json") -Raw | ConvertFrom-Json).sourceCommit
if (& git -C $projectRoot diff --name-only $sourceCommit HEAD -- src assets Cargo.toml Cargo.lock build.rs) { throw '当前源代码与打包时的提交不一致。' }
& git -c credential.helper= -c 'credential.helper=!"C:/Program Files/GitHub CLI/gh.exe" auth git-credential' -C $projectRoot push origin main
if ($LASTEXITCODE -ne 0) { throw 'GitHub 推送失败。' }
& git -C $projectRoot push gitee main
if ($LASTEXITCODE -ne 0) { throw 'Gitee 推送失败，GitHub 已推送；未创建 Release。' }
& $gh release create $tag --repo $repository --target $sourceCommit --title "桌面工作助理 $Version" --notes-file $NotesFile --draft
if ($LASTEXITCODE -ne 0) { throw 'GitHub 草稿创建失败。' }
$uploadDirectory = Join-Path $projectRoot "dist\release-assets-$Version"
if (Test-Path -LiteralPath $uploadDirectory) { throw '上传暂存目录已存在，拒绝覆盖。' }
$null = New-Item -ItemType Directory -Path $uploadDirectory
$uploadManifest = Join-Path $uploadDirectory 'update-manifest.json'
Copy-Item -LiteralPath $manifestPath -Destination $uploadManifest
& $gh release upload $tag $archive $uploadManifest --repo $repository
if ($LASTEXITCODE -ne 0) { throw 'GitHub 附件上传失败；保留草稿供检查。' }
$githubAssets = (& $gh release view $tag --repo $repository --json assets | ConvertFrom-Json).assets
if ($LASTEXITCODE -ne 0 -or $githubAssets.Count -ne 2 -or -not ($githubAssets | Where-Object { $_.name -eq $manifest.file_name -and $_.size -eq $manifest.size })) { throw 'GitHub 草稿附件回读失败。' }
$giteeRelease = Invoke-GiteeApi -Method POST -Path "/repos/$repository/releases" -Parameters @{tag_name=$tag;name="桌面工作助理 $Version";body=$notes;prerelease='false';target_commitish=$sourceCommit}
Add-Type -AssemblyName System.Net.Http
$handler = New-Object System.Net.Http.HttpClientHandler
$handler.AllowAutoRedirect = $false
$client = New-Object System.Net.Http.HttpClient($handler)
$client.DefaultRequestHeaders.UserAgent.ParseAdd('DesktopMeter-Release/' + $Version)
$client.Timeout = [TimeSpan]::FromMinutes(3)
$token = [CodexGiteeCredential]::Load('Codex:Gitee:API')
try {
    foreach ($attachment in @($archive, $uploadManifest)) {
        $multipart = New-Object System.Net.Http.MultipartFormDataContent
        $stream = [IO.File]::OpenRead($attachment)
        try {
            $authentication = New-Object System.Net.Http.StringContent($token)
            $authentication.Headers.ContentDisposition = New-Object System.Net.Http.Headers.ContentDispositionHeaderValue('form-data')
            $authentication.Headers.ContentDisposition.Name = '"access_token"'
            $multipart.Add($authentication)
            $content = New-Object System.Net.Http.StreamContent($stream)
            $content.Headers.ContentType = New-Object System.Net.Http.Headers.MediaTypeHeaderValue('application/octet-stream')
            $content.Headers.ContentDisposition = New-Object System.Net.Http.Headers.ContentDispositionHeaderValue('form-data')
            $content.Headers.ContentDisposition.Name = '"file"'
            $content.Headers.ContentDisposition.FileName = '"' + [IO.Path]::GetFileName($attachment) + '"'
            $multipart.Add($content)
            $response = $client.PostAsync("https://gitee.com/api/v5/repos/$repository/releases/$($giteeRelease.id)/attach_files", $multipart).GetAwaiter().GetResult()
            try {
                if (-not $response.IsSuccessStatusCode) {
                    $reason = $response.Content.ReadAsStringAsync().GetAwaiter().GetResult().Replace($token, '[redacted]')
                    if ($reason.Length -gt 800) { $reason = $reason.Substring(0, 800) }
                    throw "Gitee 附件上传失败 HTTP $([int]$response.StatusCode)：$reason；不自动重试或删除版本。"
                }
            } finally { $response.Dispose() }
        } finally { $multipart.Dispose(); $stream.Dispose() }
    }
} finally { $token=$null; $client.Dispose(); $handler.Dispose() }
$giteePublished = Invoke-GiteeApi -Path "/repos/$repository/releases/$($giteeRelease.id)"
$giteePublished | ConvertTo-Json -Depth 8 | Set-Content -LiteralPath (Join-Path $uploadDirectory 'gitee-release-response.json') -Encoding utf8NoBOM
$giteeLinks = if ($giteePublished.assets -is [array]) { $giteePublished.assets } else { $giteePublished.assets.links }
if (-not ($giteeLinks | Where-Object { $_.name -eq $manifest.file_name }) -or -not ($giteeLinks | Where-Object { $_.name -eq 'update-manifest.json' })) { throw 'Gitee 附件回读不完整；GitHub 保留草稿，检查发布响应文件。' }
& $gh release edit $tag --repo $repository --draft=false
if ($LASTEXITCODE -ne 0) { throw 'GitHub 公开发布失败，草稿保留。' }
[pscustomobject]@{GitHub="https://github.com/$repository/releases/tag/$tag";Gitee="https://gitee.com/$repository/releases/tag/$tag";Version=$Version} | ConvertTo-Json
