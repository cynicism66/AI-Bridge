#Requires -Version 5.1
# 集成测试只使用临时 USERPROFILE 和 BRIDGE_DB，不碰用户安装或数据库。
$ErrorActionPreference = 'Stop'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('bridge-install-test-' + [guid]::NewGuid().ToString('N'))
$originalUserProfile = $env:USERPROFILE
$originalCargoHome = $env:CARGO_HOME
$originalRustupHome = $env:RUSTUP_HOME
$originalDatabase = $env:BRIDGE_DB
$process = $null
$failure = $null
$installer = Join-Path $PSScriptRoot 'install-local.ps1'
$shell = (Get-Process -Id $PID).Path

function Invoke-Installer {
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $shell
    $info.Arguments = '-NoProfile -ExecutionPolicy Bypass -File "' + $installer + '"'
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardOutput = $true
    $info.RedirectStandardError = $true
    $info.StandardOutputEncoding = New-Object Text.UTF8Encoding($false)
    $info.StandardErrorEncoding = New-Object Text.UTF8Encoding($false)
    $child = [Diagnostics.Process]::Start($info)
    try {
        $out = $child.StandardOutput.ReadToEndAsync()
        $err = $child.StandardError.ReadToEndAsync()
        if (-not $child.WaitForExit(120000)) { $child.Kill(); throw '安装测试超时' }
        return @{ Code = $child.ExitCode; Text = $out.Result + $err.Result }
    } finally { $child.Dispose() }
}

try {
    [IO.Directory]::CreateDirectory($testRoot) | Out-Null
    # 隔离安装位置，但继续使用原工具链和依赖缓存。
    if (-not $env:CARGO_HOME) { $env:CARGO_HOME = Join-Path $originalUserProfile '.cargo' }
    if (-not $env:RUSTUP_HOME) { $env:RUSTUP_HOME = Join-Path $originalUserProfile '.rustup' }
    $env:USERPROFILE = $testRoot
    $env:BRIDGE_DB = Join-Path $testRoot 'test.db'
    $destination = Join-Path $testRoot '.bridge\bin\bridge-mcp.exe'
    foreach ($round in 1..2) {
        $result = Invoke-Installer
        if ($result.Code -ne 0) { throw "第 $round 次安装失败：$($result.Text)" }
    }
    $before = (Get-FileHash -LiteralPath $destination).Hash
    $info = New-Object Diagnostics.ProcessStartInfo
    $info.FileName = $destination
    $info.UseShellExecute = $false
    $info.CreateNoWindow = $true
    $info.RedirectStandardInput = $true
    $info.RedirectStandardOutput = $true
    $info.StandardOutputEncoding = New-Object Text.UTF8Encoding($false)
    # .NET Framework 没有 StandardInputEncoding；启动时会用 Console.InputEncoding
    # 创建并自动刷新 StreamWriter，必须在此时避免写入 BOM。
    $inputEncoding = [Console]::InputEncoding
    try {
        [Console]::InputEncoding = New-Object Text.UTF8Encoding($false)
        $process = [Diagnostics.Process]::Start($info)
    } finally { [Console]::InputEncoding = $inputEncoding }
    # 不使用受宿主 Console.InputEncoding 影响的 StreamWriter，直接写 UTF-8 无 BOM 字节。
    $request = [Text.Encoding]::UTF8.GetBytes('{"jsonrpc":"2.0","id":1,"method":"ping"}' + "`n")
    $process.StandardInput.BaseStream.Write($request, 0, $request.Length)
    $process.StandardInput.BaseStream.Flush()
    $reply = $process.StandardOutput.ReadLineAsync()
    if (-not $reply.Wait(10000)) { throw '测试 MCP 响应超时' }
    if ($reply.Result -notmatch '"result":\{\}') { throw "测试 MCP ping 响应错误：$($reply.Result)" }
    $result = Invoke-Installer
    if ($result.Code -eq 0 -or $result.Text -notmatch '请先退出 Claude 和 Codex 再安装') {
        throw "占用时未正确拒绝安装：$($result.Text)"
    }
    if ((Get-FileHash -LiteralPath $destination).Hash -ne $before) { throw '占用测试改变了原 exe' }
    if (Get-ChildItem -LiteralPath (Split-Path $destination) -Filter '*.tmp' -Force) { throw '遗留暂存文件' }
    $process.StandardInput.Close()
    if (-not $process.WaitForExit(10000)) { throw '测试 MCP 未退出' }
    $process.Dispose()
    $process = $null
    $result = Invoke-Installer
    if ($result.Code -ne 0) { throw "退出后重装失败：$($result.Text)" }
    Write-Output '通过：首次安装、重复覆盖、真实 MCP 进程占用时拒绝且原文件不变、退出后重装；无暂存文件残留。'
} catch {
    $failure = $_.Exception.Message
} finally {
    if ($process) {
        if (-not $process.HasExited) { $process.Kill(); $process.WaitForExit() }
        $process.Dispose()
    }
    $env:USERPROFILE = $originalUserProfile
    $env:CARGO_HOME = $originalCargoHome
    $env:RUSTUP_HOME = $originalRustupHome
    $env:BRIDGE_DB = $originalDatabase
    # 只允许删除本测试在系统临时目录中创建的、名称唯一的目录。
    $resolved = [IO.Path]::GetFullPath($testRoot)
    $temporary = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $resolved.StartsWith($temporary, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path $resolved -Leaf) -notlike 'bridge-install-test-*') { throw '测试清理路径校验失败' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
if ($failure) { throw $failure }
