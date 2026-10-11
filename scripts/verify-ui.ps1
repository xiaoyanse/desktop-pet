param([string]$Output = "$PSScriptRoot/../evidence/revision-20261012")
$ErrorActionPreference = 'Stop'
if ((Get-Content "$env:LOCALAPPDATA/DesktopMeter/settings.json" -Raw | ConvertFrom-Json).version -ge 3) {
    throw '本脚本仅适用于版本 2 固定顶部栏。版本 3 使用浮动操作栏，请按 docs/本轮验证.md 中的走查步骤验证；旧坐标不可复用。'
}
Add-Type -AssemblyName System.Drawing
Add-Type -TypeDefinition @'
using System;
using System.Runtime.InteropServices;
public static class MeterUi {
    [StructLayout(LayoutKind.Sequential)] public struct Rect { public int Left, Top, Right, Bottom; }
    [StructLayout(LayoutKind.Sequential)] public struct Point { public int X, Y; }
    [DllImport("user32.dll")] public static extern bool GetWindowRect(IntPtr window, out Rect rect);
    [DllImport("user32.dll")] public static extern bool GetCursorPos(out Point point);
    [DllImport("user32.dll")] public static extern bool SetCursorPos(int x, int y);
    [DllImport("user32.dll")] public static extern IntPtr GetForegroundWindow();
    [DllImport("user32.dll")] public static extern bool SetForegroundWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern uint GetDpiForWindow(IntPtr window);
    [DllImport("user32.dll")] public static extern IntPtr FindWindow(string cls, string name);
    [DllImport("user32.dll")] public static extern bool PostMessage(IntPtr window, uint message, UIntPtr w, IntPtr l);
    [DllImport("user32.dll")] public static extern IntPtr GetWindowLongPtr(IntPtr window, int index);
    [DllImport("user32.dll")] public static extern uint GetWindowThreadProcessId(IntPtr window, out uint pid);
    [DllImport("user32.dll")] public static extern void mouse_event(uint flags, uint x, uint y, uint data, UIntPtr extra);
    [DllImport("user32.dll")] public static extern void keybd_event(byte key, byte scan, uint flags, UIntPtr extra);
    [DllImport("user32.dll")] public static extern short GetAsyncKeyState(int key);
}
'@
$meter = Get-Process desktop-meter | Where-Object { $_.MainWindowTitle -eq '桌面资源监视器' } | Select-Object -First 1
if (!$meter) { throw '请先启动本项目的 desktop-meter.exe。' }
if ($meter.Path -ne (Resolve-Path "$PSScriptRoot/../target/release/desktop-meter.exe").Path) { throw '运行的不是本项目 release 程序。' }
$window = $meter.MainWindowHandle
$tray = [MeterUi]::FindWindow('DesktopMeterTray', 'DesktopMeter tray')
if ($tray -eq [IntPtr]::Zero) { throw '托盘窗口不存在。' }
New-Item -ItemType Directory -Force -Path $Output | Out-Null
$Output = (Resolve-Path $Output).Path
$originalCursor = [MeterUi+Point]::new()
[MeterUi]::GetCursorPos([ref]$originalCursor) | Out-Null
$foreground = [MeterUi]::GetForegroundWindow()
$rect = [MeterUi+Rect]::new()
[MeterUi]::GetWindowRect($window, [ref]$rect) | Out-Null
$scale = [MeterUi]::GetDpiForWindow($window) / 96.0
function Capture([string]$Name) {
    $bitmap = [Drawing.Bitmap]::new($rect.Right - $rect.Left, $rect.Bottom - $rect.Top)
    $graphics = [Drawing.Graphics]::FromImage($bitmap)
    try {
        $graphics.CopyFromScreen($rect.Left, $rect.Top, 0, 0, $bitmap.Size)
        $bitmap.Save((Join-Path $Output "$Name.png"), [Drawing.Imaging.ImageFormat]::Png)
    } finally { $graphics.Dispose(); $bitmap.Dispose() }
}
function Click([int]$X, [int]$Y) {
    [MeterUi]::SetCursorPos($X, $Y) | Out-Null
    [MeterUi]::mouse_event(2, 0, 0, 0, [UIntPtr]::Zero)
    [MeterUi]::mouse_event(4, 0, 0, 0, [UIntPtr]::Zero)
    Start-Sleep -Milliseconds 350
}
function Key([byte]$Code, [bool]$Up = $false) {
    [MeterUi]::keybd_event($Code, 0, $(if ($Up) { 2 } else { 0 }), [UIntPtr]::Zero)
}
function PassThrough {
    [MeterUi]::PostMessage($tray, 0x8001, [UIntPtr]::Zero, [IntPtr]0x205) | Out-Null
    Start-Sleep -Milliseconds 350
    $menu = [MeterUi]::FindWindow('#32768', '')
    $menuPid = [uint32]0
    [MeterUi]::GetWindowThreadProcessId($menu, [ref]$menuPid) | Out-Null
    if ($menu -eq [IntPtr]::Zero -or $menuPid -ne $meter.Id) { throw '未找到本程序的托盘菜单。' }
    $menuRect = [MeterUi+Rect]::new()
    [MeterUi]::GetWindowRect($menu, [ref]$menuRect) | Out-Null
    Click ([int](($menuRect.Left + $menuRect.Right) / 2)) ($menuRect.Bottom - [int](12 * $scale))
    Start-Sleep -Milliseconds 350
    if (([MeterUi]::GetWindowLongPtr($window, -20).ToInt64() -band 0x20) -eq 0) { throw '未进入鼠标穿透；快捷键可能被其他程序占用。' }
}
try {
    [MeterUi]::PostMessage($tray, 0x8002, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    [MeterUi]::SetForegroundWindow($window) | Out-Null
    [MeterUi]::SetCursorPos($rect.Left - 20, $rect.Top + 50) | Out-Null
    Start-Sleep -Milliseconds 1200
    Capture 'compact'
    $x = $rect.Right - [int](78 * $scale)
    $y = $rect.Top + [int](17 * $scale)
    [MeterUi]::SetCursorPos($x, $y) | Out-Null
    Start-Sleep -Milliseconds 150
    Capture 'hover'
    Click $x $y
    Capture 'fixed'
    if (([MeterUi]::GetWindowLongPtr($window, -20).ToInt64() -band 0x20) -ne 0) { throw '固定位置错误地启用了鼠标穿透。' }
    Click $x $y
    Capture 'unlocked'
    PassThrough
    Key 0x11; Key 0x12
    Start-Sleep -Milliseconds 150
    Key 0x4D
    Start-Sleep -Milliseconds 150
    Key 0x4D $true; Key 0x12 $true; Key 0x11 $true
    Start-Sleep -Milliseconds 1000
    if (([MeterUi]::GetWindowLongPtr($window, -20).ToInt64() -band 0x20) -ne 0) { throw 'Ctrl+Alt+M 未恢复穿透。' }
    PassThrough
    [MeterUi]::PostMessage($tray, 0x8001, [UIntPtr]::Zero, [IntPtr]0x202) | Out-Null
    Start-Sleep -Milliseconds 350
    if (([MeterUi]::GetWindowLongPtr($window, -20).ToInt64() -band 0x20) -ne 0) { throw '托盘单击未恢复穿透。' }
    # Deliver the same native menu command used by the tray's restore action.
    [MeterUi]::PostMessage($tray, 0x8002, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    Start-Sleep -Milliseconds 300
    if (([MeterUi]::GetWindowLongPtr($window, -20).ToInt64() -band 0x20) -ne 0) { throw '托盘恢复未清除穿透。' }
    [pscustomobject]@{ pid=$meter.Id; executable=$meter.Path; dpiScale=$scale; width=$rect.Right-$rect.Left;
        height=$rect.Bottom-$rect.Top; left=$rect.Left; top=$rect.Top; fixedDoesNotPassThrough=$true; trayRestoreClearsPassThrough=$true;
        hotkeyRestoresPassThrough=$true; trayClickRestoresPassThrough=$true;
        note='固定/解锁按钮切换见 fixed 与 unlocked 截图；已实际进入穿透后分别验证快捷键和托盘恢复。' } |
        ConvertTo-Json | Set-Content (Join-Path $Output 'ui-smoke.json') -Encoding utf8
} finally {
    Key 0x4D $true; Key 0x12 $true; Key 0x11 $true
    [MeterUi]::PostMessage($tray, 0x1F, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    [MeterUi]::PostMessage($tray, 0x8002, [UIntPtr]::Zero, [IntPtr]::Zero) | Out-Null
    [MeterUi]::SetCursorPos($originalCursor.X, $originalCursor.Y) | Out-Null
    if ($foreground -ne [IntPtr]::Zero) { [MeterUi]::SetForegroundWindow($foreground) | Out-Null }
}
