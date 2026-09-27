#Requires -Version 5.1
# 集成测试只使用临时 LOCALAPPDATA 和 BRIDGE_DB，不碰用户安装或数据库。
$ErrorActionPreference = 'Stop'
$testRoot = Join-Path ([IO.Path]::GetTempPath()) ('bridge-install-test-' + [guid]::NewGuid().ToString('N'))
$originalLocalAppData = $env:LOCALAPPDATA
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
    $env:LOCALAPPDATA = $testRoot
    $env:BRIDGE_DB = Join-Path $testRoot 'test.db'
    $destination = Join-Path $testRoot 'AI Bridge\bin\bridge-mcp.exe'
    . (Join-Path $PSScriptRoot 'package-context.ps1')
    # 独立后检：旧副本不误报，本轮写入必须拒绝；只模拟临时目录。
    $cacheCopy = Join-Path $testRoot 'Packages\Test.Package\LocalCache\Local\AI Bridge\bin\bridge-mcp.exe'
    [IO.Directory]::CreateDirectory((Split-Path $cacheCopy)) | Out-Null
    [IO.File]::WriteAllText($cacheCopy, 'fixture')
    $startedUtc = [datetime]::UtcNow
    [IO.File]::SetLastWriteTimeUtc($cacheCopy, $startedUtc.AddMinutes(-1))
    Assert-BridgeInstallNotRedirected -LocalAppData $testRoot -StartedUtc $startedUtc
    [IO.File]::SetLastWriteTimeUtc($cacheCopy, $startedUtc.AddSeconds(1))
    $rejected = $false
    try { Assert-BridgeInstallNotRedirected -LocalAppData $testRoot -StartedUtc $startedUtc }
    catch {
        if ($_.Exception.Message -notmatch '普通 PowerShell') { throw }
        $rejected = $true
    }
    if (-not $rejected -or -not (Test-Path -LiteralPath $cacheCopy)) { throw '重定向后检失败或误删副本' }
    [IO.File]::SetLastWriteTimeUtc($cacheCopy, $startedUtc.AddMinutes(-1))
    Write-Output '通过：独立 LocalCache 后检忽略旧副本，拒绝本轮写入，并保留副本。'
    if (Get-BridgePackageName) {
        $result = Invoke-Installer
        if ($result.Code -eq 0 -or $result.Text -notmatch 'MSIX' -or $result.Text -notmatch '普通 PowerShell') {
            throw "打包宿主未正确拒绝安装：$($result.Text)"
        }
        if (Test-Path -LiteralPath (Join-Path $testRoot 'AI Bridge')) { throw '拒绝安装时仍创建了安装目录' }
        Write-Output '通过：真实 MSIX 宿主中拒绝安装并提示普通 PowerShell；未创建安装目录。普通宿主的安装和占用测试由外部 PowerShell 或 Windows CI 执行。'
        return
    }
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
    $env:LOCALAPPDATA = $originalLocalAppData
    $env:BRIDGE_DB = $originalDatabase
    # 只允许删除本测试在系统临时目录中创建的、名称唯一的目录。
    $resolved = [IO.Path]::GetFullPath($testRoot)
    $temporary = [IO.Path]::GetFullPath([IO.Path]::GetTempPath()).TrimEnd('\') + '\'
    if (-not $resolved.StartsWith($temporary, [StringComparison]::OrdinalIgnoreCase) -or
        (Split-Path $resolved -Leaf) -notlike 'bridge-install-test-*') { throw '测试清理路径校验失败' }
    if (Test-Path -LiteralPath $resolved) { Remove-Item -LiteralPath $resolved -Recurse -Force }
}
if ($failure) { throw $failure }
