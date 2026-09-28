#Requires -Version 5.1
[CmdletBinding()]
param()

$ErrorActionPreference = 'Stop'
[Console]::OutputEncoding = New-Object Text.UTF8Encoding($false)
$repoRoot = [IO.Path]::GetFullPath((Join-Path $PSScriptRoot '..'))
$stagedFile = $null
$failure = $null

try {
    if (-not $env:USERPROFILE) { throw '无法确定 USERPROFILE，安装已停止。' }
    $binDirectory = Join-Path $env:USERPROFILE '.bridge\bin'
    $destination = Join-Path $binDirectory 'bridge-mcp.exe'
    Push-Location -LiteralPath $repoRoot
    try {
        & cargo build --release -p bridge-mcp
        if ($LASTEXITCODE -ne 0) { throw 'Rust release 编译失败，已保留原安装文件。' }
    } finally { Pop-Location }
    $source = Join-Path $repoRoot 'target\release\bridge-mcp.exe'
    if (-not (Test-Path -LiteralPath $source -PathType Leaf)) { throw '未找到编译后的 bridge-mcp.exe。' }
    [IO.Directory]::CreateDirectory($binDirectory) | Out-Null

    # 写权限独占打开：运行中的 Windows 映像不能这样打开。
    if (Test-Path -LiteralPath $destination) {
        try {
            $probe = [IO.File]::Open($destination, [IO.FileMode]::Open, [IO.FileAccess]::ReadWrite, [IO.FileShare]::None)
            $probe.Dispose()
        } catch [IO.IOException] {
            throw '请先退出 Claude 和 Codex 再安装'
        } catch [UnauthorizedAccessException] {
            throw '请先退出 Claude 和 Codex 再安装；若已退出，请检查安装目录的写权限。'
        }
    }

    # 同目录暂存并校验，最后原子替换；复制失败不会破坏当前 exe。
    $stagedFile = Join-Path $binDirectory ('.bridge-mcp-' + [guid]::NewGuid().ToString('N') + '.tmp')
    [IO.File]::Copy($source, $stagedFile, $false)
    if ((Get-FileHash -LiteralPath $source).Hash -ne (Get-FileHash -LiteralPath $stagedFile).Hash) {
        throw '复制校验失败，已保留原安装文件。'
    }
    try {
        if (Test-Path -LiteralPath $destination) {
            [IO.File]::Replace($stagedFile, $destination, [NullString]::Value)
        } else {
            [IO.File]::Move($stagedFile, $destination)
        }
    } catch [IO.IOException] {
        throw '请先退出 Claude 和 Codex 再安装'
    } catch [UnauthorizedAccessException] {
        throw '请先退出 Claude 和 Codex 再安装；若已退出，请检查安装目录的写权限。'
    }
    $stagedFile = $null
    Write-Output "已安装：$destination"
} catch {
    $failure = $_.Exception.Message
} finally {
    if ($stagedFile -and (Test-Path -LiteralPath $stagedFile)) {
        Remove-Item -LiteralPath $stagedFile -Force
    }
}
if ($failure) {
    [Console]::Error.WriteLine("安装失败：$failure")
    exit 1
}
