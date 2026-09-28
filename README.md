# AI Bridge

[![测试](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml/badge.svg)](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml)

**让 Claude 和 Codex 在同一个项目里知道彼此在做什么，让人也能参与协作。**

AI Bridge 是一个本地协作公告板，提供任务状态、定向／广播留言、文件认领和交互历史。
Rust 核心通过 stdio MCP 供 AI 调用，通过命令行供用户操作；数据保存在本机 SQLite 中。
采用 MIT 许可证，后续桌面软件规划见 [路线图](docs/PLAN.md) 和 [架构](docs/ARCHITECTURE.md)。

## 安装与从源码编译

Windows 需要 Git、stable Rust 的 MSVC 工具链及 Visual Studio C++ 编译工具。
安装位置为用户主目录下的 `.bridge/bin/`，和数据库共用 `.bridge/` 父目录。
MSIX 对 AppData 的私有目录转存不会影响这个位置，因此可以在 Windows Terminal、Claude 或 Codex 的 shell 中运行安装脚本，无需专门检查宿主包身份。

```powershell
git clone https://github.com/cynicism66/AI-Bridge.git D:\Bridge
cd D:\Bridge
cargo --version
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-local.ps1
```

安装脚本先执行 `cargo build --release -p bridge-mcp`，然后安装到固定位置：

```text
%USERPROFILE%\.bridge\bin\bridge-mcp.exe
```

脚本支持 Windows PowerShell 5.1 和 PowerShell 7，可重复执行来更新。
复制先写入同目录临时文件、校验 SHA256，再原子替换旧文件；复制或替换失败时保留原 exe。
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
      "command": "C:\\Users\\wangq\\.bridge\\bin\\bridge-mcp.exe",
      "args": [],
      "env": { "BRIDGE_AGENT": "claude" }
    }
  }
}
```

Codex 的 `~/.codex/config.toml` 中：

```toml
[mcp_servers.bridge]
command = 'C:\Users\wangq\.bridge\bin\bridge-mcp.exe'
args = []

[mcp_servers.bridge.env]
BRIDGE_AGENT = "codex"
```

Codex 的字段说明见 [官方 MCP 文档](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)。
修改前备份配置，修改后必须完全退出并重新启动 Claude app 和 Codex app，再分别调用 `bridge_overview` 验证。Claude 会缓存启动时的 MCP 配置，仅在 `/mcp` 中重连或重启服务器进程不会加载新的 command。验证安装来源使用 `GetMappedFileNameW` 查询实际映像文件，不能仅依赖可能显示虚拟路径的 `Get-CimInstance`。
两端应访问同一数据库，并把 [RULES.md](RULES.md) 加入各自的协作说明。

## 开关与人用命令

全局总开关默认开启，**每个项目默认关闭**，项目里的 AI 开关默认开启。三级开关按“全局 → 项目 → AI”判断，只有全部开启才允许该 AI 使用；开关由用户控制，AI 工具不能改变开关。

```powershell
$bridge = Join-Path $env:USERPROFILE '.bridge\bin\bridge-mcp.exe'
& $bridge on D:\code\example
& $bridge status
& $bridge show D:\code\example
```

下面用 `bridge-mcp` 简写安装好的程序；PowerShell 中可用上面的 `& $bridge` 代替。

| 命令 | 用途 |
|---|---|
| `bridge-mcp on [项目]` / `off [项目]` | 开启／关闭项目 |
| `bridge-mcp on --global` / `off --global` | 开启／关闭全局总开关 |
| `bridge-mcp agent [项目] <agent> on\|off` | 设置项目内某个 AI 的开关；省略项目时使用当前目录 |
| `bridge-mcp status` | 全局和各项目开关、有效状态、最近活动 |
| `bridge-mcp show [项目]` | 状态、认领、未读消息和最近 20 条消息；不标记已读 |
| `bridge-mcp post [项目] "内容" [--to all\|claude\|codex]` | 以 human 身份发消息 |
| `bridge-mcp read [项目]` | 读取并标记给 human 的未读消息 |
| `bridge-mcp history [项目] [--limit N] [--agent 身份] [--kind 类型]` | 按时间正序查看最近 50 条交互历史 |

省略项目参数时使用当前目录，命令行接受相对路径；AI 工具必须传项目根目录的绝对路径。
Windows 接受 `D:\code\example`、`D:/code/example`、UNC 路径，以及转换为 `d:/code/example` 的 `/d/code/example`。
`\example`、`/example`、`/abc/example` 等缺少盘符的路径会报错。

三级开关判断集中在核心的 `access_state`；修改开关后下一次调用立即生效。AI 关闭时工具只读取开关，不创建或更新会话、状态、消息、认领或已读记录。`list_projects` 不展示对当前 AI 关闭的项目；人用 `status`、`show` 仍展示全部项目及已知 AI 的开关和有效状态。
关闭时人用命令仍能操作；AI 收到“未开启／已关闭”提示后，应停止在本次会话调用 Bridge。
Rust 不提供 `watch`，后续由桌面软件显示持续变化。

`history` 的类型包括 `status`、`message`、`claim`、`release`、`expire`、`switch`、`agent_switch`。
AI 开关事件显示为 `[时间] human 对 codex 关闭 Bridge` 或 `开启 Bridge`，由数据库触发器写入。
项目历史也包含全局开关事件；支持身份与类型组合筛选，目前不自动清理历史。

## 项目识别与窗口会话

传入路径会向上查找 `.git`。Bridge 直接读取 `.git` 目录或文件里的 `gitdir:`，并读取 `commondir` 找到公共 Git 目录，不调用 git 命令。公共目录名为 `.git` 时用它的上级目录作为项目键，否则用公共目录本身；bare 仓库也按其公共目录识别。主目录和 worktree 因此共用公告板，子模块按自己的 Git 目录识别。

没有 Git 信息时仍按原路径识别；`.git` 格式损坏或指向不存在的目录时，也回退原路径，并只在 stderr 写一行诊断。旧的 worktree 路径历史数据不自动合并。文件认领中的绝对路径相对于当前 worktree 根目录处理，以便两棵工作树的同一相对文件互相识别。

一个 MCP 服务器进程对应一个随机会话 ID。在每个项目里首次调用时，按 AI 分配未被两小时内活跃会话占用的最小正整数，显示为 `codex #1`、`codex #2`；同一进程进入不同项目时分别编号。编号过期后可以复用，内部仍用随机 ID 校验认领归属，避免新进程释放旧进程的文件。

公告板的身份行和状态显示分支及 worktree 根目录。正常分支显示名称，游离 HEAD 显示前七位；超过 24 小时未活动的状态折叠为“较早的会话”。状态按项目、AI 和会话编号分别保存；不同窗口认领同一文件会冲突，释放只作用于自己的会话。

**消息始终按 agent 投递，已读记录也按 agent 共享。** 给 codex 的消息可由任意 codex 窗口读取；一个窗口读过后，另一个窗口不会重复收到未读提示。人用 CLI 不创建 MCP 会话。

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

当前数据库版本为 3；迁移在事务中完成，旧消息与当前状态会回填到历史表。
版本 3 保留原消息、已读、开关和历史；旧状态/认领迁为会话 #1，并建立迁移来源会话（PID 0），避免新窗口冒领仍有效的旧认领。旧版 Rust 和冻结版 Python 会拒绝写入版本 3 数据库，因此部署时需先退出两端、安装新版后再完整启动两端。
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

## T05b 安装位置切换与回滚

当前有效安装位置为 `%USERPROFILE%\.bridge\bin\bridge-mcp.exe`。T05b 切换前的完整配置备份为 `~/.claude.json.bak-t05b` 和 `~/.codex/config.toml.bak-t05b`；这次只修改各自 `bridge.command`，不修改 args、身份或其他设置。

如需撤销本次配置变更，先退出两端，再恢复备份：

```powershell
Copy-Item -LiteralPath "$env:USERPROFILE\.claude.json.bak-t05b" -Destination "$env:USERPROFILE\.claude.json" -Force
Copy-Item -LiteralPath "$env:USERPROFILE\.codex\config.toml.bak-t05b" -Destination "$env:USERPROFILE\.codex\config.toml" -Force
```

备份引用的是旧 AppData 路径，而 T05 事故中该路径在用户的普通 PowerShell 中不存在；**恢复 `.bak-t05b` 只撤销配置，不能保证恢复连接**。应优先保留新的 `.bridge/bin/` 安装路径修复问题。恢复完整配置也会覆盖备份之后的其他设置，执行前应核对。

Claude 的旧私有目录 `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\AI Bridge\` 由用户在 Claude 已切换新路径、重连成功后自行清理，安装脚本不删除它。更早的 `.bak-t05` 备份继续保留。

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
此历史回滚只适用于数据库版本 2。T06a 升级到版本 3 后，冻结版会拒绝写入，不能只恢复旧配置继续使用。

## 代码结构

- `crates/bridge-core/`：数据库、路径、开关、工具、中文格式化。
- `crates/bridge-mcp/`：stdio MCP 与命令行入口。
- `migrations/`：中立位置的编号 SQL 迁移。
- `scripts/`：本地安装及安装集成验证。
- `tests/conformance/`：Rust 黑盒行为规格。


Python 版已冻结，仅作为历史参考保留在 [legacy/](legacy/README.md)，其历史单元测试继续在 CI 中运行。
