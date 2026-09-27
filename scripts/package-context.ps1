#Requires -Version 5.1
# 用 Windows 包身份 API 检测打包宿主，不依赖进程名称或可伪造的环境变量。
function Assert-BridgeInstallNotRedirected {
    param([string]$LocalAppData, [datetime]$StartedUtc)
    $packages = Join-Path $LocalAppData 'Packages'
    if (-not (Test-Path -LiteralPath $packages)) { return }
    foreach ($package in Get-ChildItem -LiteralPath $packages -Directory -ErrorAction Stop) {
        $copy = Join-Path $package.FullName 'LocalCache\Local\AI Bridge\bin\bridge-mcp.exe'
        if (Test-Path -LiteralPath $copy -PathType Leaf) {
            $file = Get-Item -LiteralPath $copy -ErrorAction Stop
            if ($file.LastWriteTimeUtc -ge $StartedUtc) {
                throw "检测到本次安装期间更新的 MSIX 私有副本：$copy。安装路径验证失败，请用户从开始菜单打开普通 PowerShell 后重新安装；未自动删除任何副本。"
            }
        }
    }
}

function Get-BridgePackageName {
    if (-not ('BridgeInstall.PackageIdentity' -as [type])) {
        Add-Type -TypeDefinition @'
using System;
using System.ComponentModel;
using System.Runtime.InteropServices;
using System.Text;
namespace BridgeInstall {
    public static class PackageIdentity {
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        private static extern int GetCurrentPackageFullName(ref uint length, StringBuilder name);
        [DllImport("kernel32.dll", CharSet = CharSet.Unicode)]
        private static extern int GetPackageFullName(IntPtr process, ref uint length, StringBuilder name);
        [DllImport("kernel32.dll", SetLastError = true)]
        private static extern IntPtr OpenProcess(uint access, bool inherit, uint processId);
        [DllImport("kernel32.dll")]
        private static extern bool CloseHandle(IntPtr handle);
        public static string GetName() {
            uint length = 0;
            int code = GetCurrentPackageFullName(ref length, null);
            if (code == 15700) return null; // APPMODEL_ERROR_NO_PACKAGE
            if (code != 122) throw new Win32Exception(code); // ERROR_INSUFFICIENT_BUFFER
            var name = new StringBuilder((int)length);
            code = GetCurrentPackageFullName(ref length, name);
            if (code != 0) throw new Win32Exception(code);
            return name.ToString();
        }
        public static string GetProcessName(uint processId) {
            IntPtr process = OpenProcess(0x1000, false, processId);
            if (process == IntPtr.Zero) throw new Win32Exception(Marshal.GetLastWin32Error());
            try {
                uint length = 0;
                int code = GetPackageFullName(process, ref length, null);
                if (code == 15700) return null;
                if (code != 122) throw new Win32Exception(code);
                var name = new StringBuilder((int)length);
                code = GetPackageFullName(process, ref length, name);
                if (code != 0) throw new Win32Exception(code);
                return name.ToString();
            } finally { CloseHandle(process); }
        }
    }
}
'@
    }
    $name = [BridgeInstall.PackageIdentity]::GetName()
    if ($name) { return $name }
    # Codex 的中间 CLI/PowerShell 可能没有包身份，仍需检查打包的祖先宿主。
    $cursor = [uint32]$PID
    $seen = New-Object 'Collections.Generic.HashSet[uint32]'
    while ($cursor -ne 0 -and $seen.Add($cursor)) {
        $process = Get-CimInstance Win32_Process -Filter "ProcessId=$cursor" -ErrorAction Stop
        if (-not $process) { throw '安装环境检测时父进程已退出，请在普通 PowerShell 中重试。' }
        if ($process.Name -in @('explorer.exe', 'services.exe', 'wininit.exe')) { break }
        $name = [BridgeInstall.PackageIdentity]::GetProcessName($cursor)
        if ($name) { return $name }
        $cursor = [uint32]$process.ParentProcessId
    }
    return $null
}
