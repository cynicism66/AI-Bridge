# AI Bridge

[![测试](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml/badge.svg)](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml)

**让 Claude 和 Codex 在同一个项目里知道彼此在做什么，让人也能参与协作。**

AI Bridge 是一个本地协作公告板，提供任务状态、定向／广播留言、文件认领和交互历史。
Rust 核心通过 stdio MCP 供 AI 调用，通过命令行供用户操作；数据保存在本机 SQLite 中。
采用 MIT 许可证，后续桌面软件规划见 [路线图](docs/PLAN.md) 和 [架构](docs/ARCHITECTURE.md)。

## 安装与从源码编译

Windows 需要 Git、stable Rust 的 MSVC 工具链及 Visual Studio C++ 编译工具。
**安装必须由用户从开始菜单打开普通 PowerShell 后亲自执行，不能交给 Codex 或其他打包应用代为安装。**
MSIX 宿主可能将 LOCALAPPDATA 写入重定向到自己的私有目录，导致其他客户端找不到程序；脚本检测到包身份后会在编译、创建安装目录前拒绝安装。
在普通 PowerShell 中执行：

```powershell
git clone https://github.com/cynicism66/AI-Bridge.git D:\Bridge
cd D:\Bridge
cargo --version
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-local.ps1
```

包身份检测使用 Windows 的 [GetCurrentPackageFullName](https://learn.microsoft.com/en-us/windows/win32/api/appmodel/nf-appmodel-getcurrentpackagefullname) 和 [GetPackageFullName](https://learn.microsoft.com/en-us/windows/win32/api/appmodel/nf-appmodel-getpackagefullname) API，同时检查当前进程及其宿主链。

安装脚本先执行 `cargo build --release -p bridge-mcp`，然后安装到固定位置：

```text
%LOCALAPPDATA%\AI Bridge\bin\bridge-mcp.exe
```

脚本支持 Windows PowerShell 5.1 和 PowerShell 7，可重复执行来更新。
复制先写入同目录临时文件、校验 SHA256，再原子替换旧文件；复制或替换失败时保留原 exe。
替换后还会独立检查 MSIX LocalCache 是否出现本轮更新的副本；发现重定向时返回非零退出码并提示普通 PowerShell，不自动删除副本。
运行中的 exe 被占用时会提示“请先退出 Claude 和 Codex 再安装”并退出；退出两端后重新执行即可。
MCP 配置应引用安装路径，编译目录 `target/release/` 会被 `cargo clean` 清理。

只编译、不安装时执行 `cargo build --release -p bridge-mcp`。
Windows 产物为 `target/release/bridge-mcp.exe`，Linux 为 `target/release/bridge-mcp`。
无参数启动时等待 MCP 输入，每行一条 UTF-8 JSON；stdout 只输出协议内容。

## 接入 Claude Code 与 Codex

示例使用用户名 `wangq`，请替换为你自己的完整安装路径。
只调整现有 `bridge` 条目，不要覆盖配置文件中的其他内容。

Claude Code 的 `~/.claude.json` 中：

```json
{
  "mcpServers": {
    "bridge": {
      "type": "stdio",
      "command": "C:\\Users\\wangq\\AppData\\Local\\AI Bridge\\bin\\bridge-mcp.exe",
      "args": [],
      "env": { "BRIDGE_AGENT": "claude" }
    }
  }
}
```

Codex 的 `~/.codex/config.toml` 中：

```toml
[mcp_servers.bridge]
command = 'C:\Users\wangq\AppData\Local\AI Bridge\bin\bridge-mcp.exe'
args = []

[mcp_servers.bridge.env]
BRIDGE_AGENT = "codex"
```

Codex 的字段说明见 [官方 MCP 文档](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)。
修改前备份配置，修改后重启 Claude app 和 Codex app，再分别调用 `bridge_overview` 验证。
两端应访问同一数据库，并把 [RULES.md](RULES.md) 加入各自的协作说明。

## 开关与人用命令

全局总开关默认开启，**每个项目默认关闭**；开关由用户控制，AI 工具不能改变开关。

```powershell
$bridge = Join-Path $env:LOCALAPPDATA 'AI Bridge\bin\bridge-mcp.exe'
& $bridge on D:\code\example
& $bridge status
& $bridge show D:\code\example
```

下面用 `bridge-mcp` 简写安装好的程序；PowerShell 中可用上面的 `& $bridge` 代替。

| 命令 | 用途 |
|---|---|
| `bridge-mcp on [项目]` / `off [项目]` | 开启／关闭项目 |
| `bridge-mcp on --global` / `off --global` | 开启／关闭全局总开关 |
| `bridge-mcp status` | 全局和各项目开关、有效状态、最近活动 |
| `bridge-mcp show [项目]` | 状态、认领、未读消息和最近 20 条消息；不标记已读 |
| `bridge-mcp post [项目] "内容" [--to all\|claude\|codex]` | 以 human 身份发消息 |
| `bridge-mcp read [项目]` | 读取并标记给 human 的未读消息 |
| `bridge-mcp history [项目] [--limit N] [--agent 身份] [--kind 类型]` | 按时间正序查看最近 50 条交互历史 |

省略项目参数时使用当前目录，命令行接受相对路径；AI 工具必须传项目根目录的绝对路径。
Windows 接受 `D:\code\example`、`D:/code/example`、UNC 路径，以及转换为 `d:/code/example` 的 `/d/code/example`。
`\example`、`/example`、`/abc/example` 等缺少盘符的路径会报错。

全局和项目同时开启时 AI 才能使用项目协作工具，修改开关后下一次调用立即生效。
关闭时人用命令仍能操作；AI 收到“未开启／已关闭”提示后，应停止在本次会话调用 Bridge。
Rust 不提供 `watch`，后续由桌面软件显示持续变化。

`history` 的类型包括 `status`、`message`、`claim`、`release`、`expire`、`switch`。
项目历史也包含全局开关事件；支持身份与类型组合筛选，目前不自动清理历史。

## 七个 AI 工具

| 工具 | 用途 |
|---|---|
| `bridge_overview` | 查看状态、认领和未读消息 |
| `update_status` | 更新任务、进度、卡点和下一步 |
| `send_message` | 给 claude、codex、human 或 all 留言 |
| `read_messages` | 读取并标记消息，或查询最近消息 |
| `claim_files` | 整批认领文件，有冲突时整批拒绝 |
| `release_files` | 释放自己的认领 |
| `list_projects` | 查看项目并标注开关状态 |

认领默认有效期 60 分钟，同一身份再次认领会续期；它是协作约定，不是文件系统锁。
典型流程是：查看公告板 → 认领文件 → 更新进度 → 留言交接 → 释放认领。

## 数据与测试

- 默认数据库为用户主目录下 `.bridge/bridge.db`；Windows 使用 `USERPROFILE`。
- `BRIDGE_DB` 可指定其他数据库，自定义路径的父目录需存在。AI 和命令行应使用相同路径。
- `BRIDGE_AGENT` 指定 AI 身份，缺省为 `unknown`；人用命令固定使用 human。
- `BRIDGE_FAKE_NOW` **只用于测试**，格式 `YYYY-MM-DD HH:MM:SS`，控制业务时间和认领清理比较。
  触发器中的开关／删除认领事件按迁移 SQL 使用 SQLite 的真实本地时间。

当前数据库版本为 2；迁移在事务中完成，旧消息与当前状态会回填到历史表。
每个进程对同一数据库路径只初始化一次，首次并发启动有锁等待及有限重试；更高版本的数据库拒绝写入。
测试使用独立临时 `BRIDGE_DB`，不读写用户真实数据库。

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p bridge-mcp
python -X dev -W error -m unittest discover -s tests/conformance -t . -v
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\test-install-local.ps1
```

一致性测试默认使用仓库中的 Rust release 可执行文件；找不到时会提示先编译。
`BRIDGE_CMD` 可指定单个可执行文件的路径，不能包含命令参数。
协议定义的唯一现役来源是 `crates/bridge-core/resources/`；测试直接读这份定义，其他响应仅归一化时间后逐字比较。
Rust 在 Windows／Ubuntu CI 中运行格式、Clippy、单元测试和黑盒契约测试；Windows 另验证本地安装脚本。

## T05 回滚

切换前的完整配置备份为 `~/.claude.json.bak-t05` 和 `~/.codex/config.toml.bak-t05`。
退出两个 app 后，先恢复旧配置引用的入口和源码，再恢复配置：

```powershell
cd D:\Bridge
# 原路径不存在时，从归档恢复；已有文件时先核对，避免覆盖修改
Copy-Item -LiteralPath .\legacy\bridge.py -Destination .\bridge.py
New-Item -ItemType Directory -Path .\src -Force | Out-Null
Copy-Item -LiteralPath .\legacy\src\bridge_mcp -Destination .\src\bridge_mcp -Recurse

Copy-Item -LiteralPath "$env:USERPROFILE\.claude.json.bak-t05" -Destination "$env:USERPROFILE\.claude.json" -Force
Copy-Item -LiteralPath "$env:USERPROFILE\.codex\config.toml.bak-t05" -Destination "$env:USERPROFILE\.codex\config.toml" -Force
```

然后重启 Claude app 和 Codex app。恢复配置会回到备份时的完整设置；迁移 SQL 保留在根目录 `migrations/`。
当前回滚适用于数据库版本 2；后续数据库升级后，应先检查旧实现是否仍然兼容。

## 代码结构

- `crates/bridge-core/`：数据库、路径、开关、工具、中文格式化。
- `crates/bridge-mcp/`：stdio MCP 与命令行入口。
- `migrations/`：中立位置的编号 SQL 迁移。
- `scripts/`：本地安装及安装集成验证。
- `tests/conformance/`：Rust 黑盒行为规格。


Python 版已冻结，仅作为历史参考保留在 [legacy/](legacy/README.md)，其历史单元测试继续在 CI 中运行。
