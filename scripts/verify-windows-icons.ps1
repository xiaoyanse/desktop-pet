param(
    [Parameter(Mandatory)][string]$Executable,
    [Parameter(Mandatory)][string]$IconFile,
    [string]$Installer
)
$ErrorActionPreference = 'Stop'
if (-not ('DesktopMeterIconResources' -as [type])) {
    Add-Type -TypeDefinition @'
using System;
using System.Collections.Generic;
using System.Runtime.InteropServices;
public static class DesktopMeterIconResources {
    [DllImport("kernel32.dll", CharSet=CharSet.Unicode, SetLastError=true)]
    static extern IntPtr LoadLibraryExW(string path, IntPtr file, uint flags);
    [DllImport("kernel32.dll")] static extern bool FreeLibrary(IntPtr module);
    [DllImport("kernel32.dll")] static extern IntPtr FindResourceW(IntPtr module, IntPtr name, IntPtr type);
    [DllImport("kernel32.dll")] static extern IntPtr LoadResource(IntPtr module, IntPtr resource);
    [DllImport("kernel32.dll")] static extern IntPtr LockResource(IntPtr resource);
    [DllImport("kernel32.dll")] static extern uint SizeofResource(IntPtr module, IntPtr resource);
    delegate bool EnumName(IntPtr module, IntPtr type, IntPtr name, IntPtr parameter);
    [DllImport("kernel32.dll")] static extern bool EnumResourceNamesW(IntPtr module, IntPtr type, EnumName callback, IntPtr parameter);
    static byte[] Read(IntPtr module, IntPtr type, IntPtr name) {
        var resource = FindResourceW(module, name, type);
        if (resource == IntPtr.Zero) throw new Exception("missing icon resource");
        int size = checked((int)SizeofResource(module, resource));
        var data = new byte[size];
        Marshal.Copy(LockResource(LoadResource(module, resource)), data, 0, size);
        return data;
    }
    public static byte[][] ReadIcons(string path) {
        // Data-file mapping never runs application code or creates a window.
        var module = LoadLibraryExW(path, IntPtr.Zero, 0x22);
        if (module == IntPtr.Zero) throw new Exception("cannot map executable resources");
        try {
            var images = new List<byte[]>();
            EnumName callback = (m, type, name, parameter) => {
                var group = Read(m, type, name);
                if (group.Length < 6) throw new Exception("invalid icon group");
                int count = BitConverter.ToUInt16(group, 4);
                if (group.Length != 6 + count * 14) throw new Exception("invalid icon entries");
                for (int i=0; i<count; i++) {
                    int id = BitConverter.ToUInt16(group, 6 + i*14 + 12);
                    images.Add(Read(m, new IntPtr(3), new IntPtr(id)));
                }
                return true;
            };
            if (!EnumResourceNamesW(module, new IntPtr(14), callback, IntPtr.Zero))
                throw new Exception("executable has no icon group");
            GC.KeepAlive(callback);
            return images.ToArray();
        } finally { FreeLibrary(module); }
    }
}
'@
}
$icon = [IO.File]::ReadAllBytes((Get-Item -LiteralPath $IconFile).FullName)
if ($icon.Length -lt 6 -or [BitConverter]::ToUInt16($icon, 2) -ne 1 -or [BitConverter]::ToUInt16($icon, 4) -ne 6) { throw '品牌 ICO 不包含六个尺寸。' }
$expected = for ($index=0; $index -lt 6; $index++) {
    $entry = 6 + 16*$index
    $length = [BitConverter]::ToUInt32($icon, $entry+8)
    $offset = [BitConverter]::ToUInt32($icon, $entry+12)
    $bytes = [byte[]]::new($length)
    [Array]::Copy($icon, $offset, $bytes, 0, $length)
    [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($bytes))
}
$reports = foreach ($path in @($Executable, $Installer) | Where-Object { $_ }) {
    $full = (Get-Item -LiteralPath $path).FullName
    $actual = foreach ($image in [DesktopMeterIconResources]::ReadIcons($full)) {
        [Convert]::ToHexString([Security.Cryptography.SHA256]::HashData($image))
    }
    foreach ($hash in $expected) {
        if ($hash -notin $actual) { throw "产物未嵌入完整品牌图标：$full" }
    }
    [pscustomobject]@{Path=$full;BrandIconSizes=@(16,32,48,64,128,256);Verified=$true;ProductVersion=[Diagnostics.FileVersionInfo]::GetVersionInfo($full).ProductVersion}
}
$reports | ConvertTo-Json -Depth 3
