param(
    [Parameter(Mandatory)][string]$Installer,
    [Parameter(Mandatory)][string]$PackageDirectory
)
$ErrorActionPreference = 'Stop'
$setup = (Get-Item -LiteralPath $Installer).FullName
$package = (Get-Item -LiteralPath $PackageDirectory).FullName
if (-not $setup.StartsWith('E:\', [StringComparison]::OrdinalIgnoreCase)) { throw '安装验收必须使用 E 盘产物。' }
$registry = 'HKCU:\Software\Microsoft\Windows\CurrentVersion\Uninstall\{57F879EB-06E4-44F2-8CC5-A990C358CD0E}_is1'
$menu = Join-Path ([Environment]::GetFolderPath('Programs')) '桌面工作助理'
if ((Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $menu)) { throw '已有同名安装或快捷方式，拒绝影响现有安装。' }
$proof = 'E:\Temp\desktop-meter-installer-' + [DateTime]::Now.ToString('yyyyMMdd-HHmmss')
if (Test-Path -LiteralPath $proof) { throw '验收目录已存在。' }
$null = New-Item -ItemType Directory -Path $proof
$target = Join-Path $proof 'application'
$process = Start-Process -FilePath $setup -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/DIR="' + $target + '"'),('/LOG="' + (Join-Path $proof 'install.log') + '"')) -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(45000)) { throw '安装超时；不终止其他程序。' }
if ($process.ExitCode -ne 0) { throw "安装失败 $($process.ExitCode)" }
foreach ($name in @('desktop-meter.exe','README.md','启动设置.cmd','manifest.json')) {
    if ((Get-FileHash -LiteralPath (Join-Path $package $name)).Hash -ne (Get-FileHash -LiteralPath (Join-Path $target ("current\" + $name))).Hash) { throw '安装后文件与包不一致。' }
}
$record = Get-ItemProperty -LiteralPath $registry
$version = (Get-Content -LiteralPath (Join-Path $package 'manifest.json') -Raw | ConvertFrom-Json).version
if ($record.DisplayVersion -ne $version -or $record.InstallLocation.TrimEnd('\') -ne $target) { throw '卸载注册记录不正确。' }
$uninstaller = Join-Path $target 'uninstall\unins000.exe'
if (-not (Test-Path -LiteralPath $uninstaller)) { throw '卸载器缺失。' }
$exe = Join-Path $target 'current\desktop-meter.exe'
$process = Start-Process -FilePath $exe -ArgumentList @('--update-check-probe','Gitee',('"' + (Join-Path $proof 'installed-update-check.json') + '"')) -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(45000) -or $process.ExitCode -ne 0) { throw '安装版在线检查失败。' }
$sentinel = Join-Path $target 'current\user-keep.txt'
Set-Content -LiteralPath $sentinel -Value 'isolated user-file retention verification' -Encoding utf8NoBOM
$process = Start-Process -FilePath $uninstaller -ArgumentList @('/VERYSILENT','/SUPPRESSMSGBOXES','/NORESTART',('/LOG="' + (Join-Path $proof 'uninstall.log') + '"')) -WindowStyle Hidden -PassThru
if (-not $process.WaitForExit(45000) -or $process.ExitCode -ne 0) { throw '卸载失败。' }
if ((Test-Path -LiteralPath $exe) -or (Test-Path -LiteralPath $registry) -or (Test-Path -LiteralPath $menu) -or -not (Test-Path -LiteralPath $sentinel)) { throw '卸载清理或用户文件保留验证失败。' }
$report = [ordered]@{Version=$version;InstallerSha256=(Get-FileHash -LiteralPath $setup).Hash;InstalledFilesVerified=4;RegistryVerified=$true;InstalledUpdateCheckPassed=$true;UninstallPassed=$true;ExtraUserFileRetained=$true;NormalApplicationStarted=$false;Evidence=$proof}
$report | ConvertTo-Json | Set-Content -LiteralPath (Join-Path $proof 'report.json') -Encoding utf8NoBOM
$report | ConvertTo-Json
