# AI Bridge

[![测试](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml/badge.svg)](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml)

**让 Claude 和 Codex 在同一个项目里知道彼此在做什么，让人也能参与协作。**

AI Bridge（简称 Bridge）是一个共享状态、留言与文件认领的 MCP 服务器。
数据按项目保存在本地 SQLite 中。现役 Python 版支持 Python 3.10 及以上，运行时只使用标准库；
T04 新增行为兼容的 Rust 核心与 `bridge-mcp` 可执行文件，两个版本可以同时访问同一数据库。
当前客户端仍使用 Python 入口，正式切换在 T05 进行。

[项目仓库](https://github.com/cynicism66/AI-Bridge) · [协作规则](RULES.md) · [开发路线图](docs/PLAN.md)

## 能解决什么问题

多个 AI 同时处理代码时，容易重复工作、修改同一个文件，或者不知道对方的进度。
Bridge 提供一个由用户决定是否启用的协作公告板：

- **同步状态**：记录任务、进度、卡点和下一步。
- **互相留言**：支持定向消息、广播消息与按身份记录的已读状态。
- **认领文件**：提前声明负责的文件，遇到其他身份的认领冲突时整批拒绝。
- **区分项目**：多个项目共享一份数据库，各自拥有独立数据和开关。
- **人参与协作**：在终端查看公告板、发消息、收消息或持续观察变化。

例如，Claude 规划和审查，Codex 实现和测试，用户通过命令行发出补充要求。
具体分工由你决定，Bridge 不会自动分配任务。

## 快速开始

准备 Git 和 Python 3.10 或更高版本。以下示例使用 Windows PowerShell，项目位于 `D:\Bridge`：

```powershell
git clone https://github.com/cynicism66/AI-Bridge.git D:\Bridge
cd D:\Bridge
python --version
python .\bridge.py on
python .\bridge.py status
python .\bridge.py show
```

已有本地仓库时直接进入原目录。然后配置下面的 MCP 客户端，并将 [RULES.md](RULES.md)
加入各自的项目说明。服务器代码所在目录和实际协作项目可以不同。

无需安装包即可启动服务器：

```powershell
python D:\Bridge\bridge.py
```

无参数时仍通过 stdio 处理 MCP 请求，每行一条 UTF-8 JSON 消息；等待输入且没有终端提示是正常的。
原有指向 `D:\Bridge\bridge.py` 的客户端入口配置可以继续使用。

## 开关

**全局总开关默认开启，每个项目默认关闭。全局和项目都开启时，AI 才能使用该项目的协作工具。**
开关由用户通过命令行控制，不提供给 AI 的开关工具。设置保存在数据库中，修改后下一次工具调用
立即生效，无需重启已运行的 T03 服务器。

```powershell
python D:\Bridge\bridge.py on D:\code\example
python D:\Bridge\bridge.py off D:\code\example
python D:\Bridge\bridge.py off --global
python D:\Bridge\bridge.py on --global
python D:\Bridge\bridge.py status
```

`on`、`off` 省略项目时使用当前目录，也接受相对路径。目录尚未创建时会提示，但仍保存设置。
`status` 显示全局开关、各项目的开关和有效状态，以及最近活动时间。
全局关闭不会清除项目设置，重新开启全局后继续采用原来的项目开关。

项目未开启或全局关闭时，AI 工具返回普通说明，不返回错误，也不读取或修改状态、消息和认领数据；
只读检查开关设置。AI 收到提示后，本次会话应停止调用 Bridge 并照常工作。
`list_projects` 在全局开启时仍可列出项目并标注“已开启”或“未开启”。

**升级提醒：升级到 T03 后，已经在用的项目（包括 `D:\Bridge` 本身）也需要执行一次 `bridge on`。**
直接运行源码时对应命令为 `python D:\Bridge\bridge.py on D:\Bridge`。
从 T02 升级时需重新加载服务器代码；之后调整开关无需再次重启。旧数据会保留。

## 配置到 Claude 和 Codex

将示例中的 Python 和项目路径替换为实际绝对路径，两端应访问同一数据库。
可运行 `python -c "import sys; print(sys.executable)"` 查找 Python 可执行文件。

Claude 的本地 stdio MCP 配置示例，合并到客户端的 `mcpServers` 配置：

```json
{
  "mcpServers": {
    "bridge": {
      "command": "C:\\Python314\\python.exe",
      "args": ["D:\\Bridge\\bridge.py"],
      "env": {"BRIDGE_AGENT": "claude"}
    }
  }
}
```

Codex 的 `~/.codex/config.toml` 配置示例：

```toml
[mcp_servers.bridge]
command = 'C:\Python314\python.exe'
args = ['D:\Bridge\bridge.py']

[mcp_servers.bridge.env]
BRIDGE_AGENT = "codex"
```

Codex 配置字段见 [官方 MCP 文档](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)。
配置完成后重新加载对应客户端的 MCP 服务器，先通过命令行开启项目，再分别要求两个 AI：

> 请用 Bridge 查看 D:\Bridge 的公告板，并汇报你当前负责的任务。

能看到彼此的状态即说明接入成功。处理其他项目时，将项目参数换成实际项目的绝对根路径。

## 人参与协作

安装命令行入口后可使用 `bridge`；未安装时，把它替换为 `python D:\Bridge\bridge.py`。
**以下项目参数均可省略，默认使用当前目录；命令行接受相对路径，AI 工具仍要求绝对路径。**

| 命令 | 作用 |
| --- | --- |
| `bridge status` | 查看全局和所有项目的开关及最近活动 |
| `bridge show [项目]` | 查看项目启用状态、各方状态、认领和最近 20 条消息 |
| `bridge post [项目] "内容" [--to all\|claude\|codex]` | 以 `human` 身份发消息，默认广播 |
| `bridge read [项目]` | 读取给 `human` 或 `all` 的未读消息，读后标记已读 |
| `bridge history [项目] [--limit N] [--agent 身份] [--kind 类型]` | 按时间正序查看最近 50 条交互历史，包含全局开关事件 |
| `bridge watch [项目] [--interval 秒]` | 首次显示公告板，之后仅在内容变化时输出；默认每 2 秒检查 |

```powershell
python D:\Bridge\bridge.py post D:\code\example "先完成测试再改接口" --to codex
python D:\Bridge\bridge.py post "请双方同步当前进度"
python D:\Bridge\bridge.py read
python D:\Bridge\bridge.py watch --interval 1
python D:\Bridge\bridge.py show | Select-Object -First 3
```

消息有空格时请加引号。`watch` 按 Ctrl+C 正常退出，不打印 traceback；没有变化时不会持续刷屏。
关闭 Bridge 后，人仍可以使用这些命令，输出开头会提示关闭状态。
`show`、`watch` 使用 `human` 身份但不标记已读；`read` 才会修改 human 的已读记录。
AI 可用 `send_message` 并指定 `to="human"` 给你留言。控制台走文本输出，重定向的程序输出为 UTF-8。

`history` 可筛选 `status`、`message`、`claim`、`release`、`expire`、`switch`。
状态更新、留言、认领和续期、释放和到期清理、开关调整都会留下记录；目前不清理历史。
关闭项目后也可查看历史。历史是人用命令，没有新增 AI 工具。

```powershell
python D:\Bridge\bridge.py history D:\code\example --limit 20
python D:\Bridge\bridge.py history --agent codex --kind status
```

Windows 项目路径接受 `D:\code\example`、`D:/code/example`、UNC 路径和 Git Bash 的 `/d/code/example`。
`/d/code/example` 会转换成 `d:/code/example`；`\example`、`/example`、`/abc/example` 等缺少盘符的路径会报错。
命令行仍可使用 `.`、`..` 等相对路径；AI 工具必须提供明确的绝对路径。

## 数据库与身份

- `BRIDGE_AGENT`：AI 身份，通常为 `claude` 或 `codex`，未设置时为 `unknown`；人用命令固定使用 `human`。
- `BRIDGE_DB`：指定数据库路径，优先于默认路径；自定义路径的父目录需要预先存在。
- `BRIDGE_FAKE_NOW`：**只用于测试**，格式 `YYYY-MM-DD HH:MM:SS`，控制业务时间和认领到期检查。
  迁移 SQL 中开关、删除认领事件的时间与到期分类按任务书使用 SQLite 的 `datetime('now','localtime')`；
  这些触发器使用系统本地时间。测试通过过去／未来时间构造到期场景，并归一化输出时间后比较。
- 默认数据库：`Path.home() / ".bridge" / "bridge.db"`，写入时自动创建目录。
  Windows 上使用 `USERPROFILE`，不受自定义 `HOME` 影响。

同一系统用户的两个客户端使用默认路径即可共享数据。如果指定了自定义数据库，命令行也要使用同一路径：

```powershell
$env:BRIDGE_DB = 'D:\Bridge\bridge.db'
python D:\Bridge\bridge.py status
```

早期原型使用脚本同目录的 `bridge.db`，当前版本不自动搬迁旧数据。
要继续使用旧库，请让两端和命令行显式指定相同的 `BRIDGE_DB`。
公告板数据保存在本地；读取内容是否发送给模型服务，取决于 AI 客户端。

数据库通过 `PRAGMA user_version` 管理版本。第一次写连接将 T03 的版本 0／1 数据库迁移到版本 2，
保留原表数据，并把已有消息和当前状态导入事件表。两种实现共用 `src/bridge_mcp/migrations/` 中的 SQL。
每个进程对同一路径只初始化一次；更高版本的数据库会被拒绝写入，并提示升级 Bridge。
迁移在事务中完成，多个进程首次同时启动时会等待写锁，并对 WAL 切换进行有限重试。

## 项目内安装与测试

只在项目虚拟环境中安装，不安装到全局 Python：

```powershell
cd D:\Bridge
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -e .
.\.venv\Scripts\bridge.exe status
.\.venv\Scripts\python.exe -m bridge_mcp
python -X dev -W error -m unittest discover -s tests -v
```

激活 `.venv` 后可以直接使用 `bridge`。`setuptools` 仅用于构建安装包，运行时没有第三方依赖。
测试使用标准库 `unittest` 和独立临时 `BRIDGE_DB`，不接触真实数据库。
每次 push 和 pull_request 都会在 Windows／Linux × Python 3.10／3.14 上运行严格测试。

### Rust 构建与一致性测试

需要 stable Rust 工具链；Windows 使用 MSVC 工具链及 Visual Studio C++ 编译工具。
Rust 支持 `on`、`off`、`status`、`show`、`post`、`read`、`history`；无参数启动 MCP，`watch` 仅保留在 Python 版。

```powershell
cargo --version
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p bridge-mcp

# 默认测试 Python；全套 Python unittest 也会自动包含这组黑盒测试
python -X dev -W error -m unittest discover -s tests/conformance -t . -v

# BRIDGE_CMD 是一个可执行文件路径，不是带参数的命令字符串
$env:BRIDGE_CMD = (Resolve-Path .\target\release\bridge-mcp.exe).Path
python -X dev -W error -m unittest discover -s tests/conformance -t . -v
Remove-Item Env:BRIDGE_CMD
```

Linux 对应的二进制路径是 `target/release/bridge-mcp`。CI 另有 Windows／Linux 两个 Rust 任务，
执行格式检查、Clippy、单元测试、release 构建及上述黑盒测试（包含 Python/Rust 双向数据库互通）。
`tests/conformance/golden/` 来自 Python MCP 子进程的真实响应；工具清单按规范化 JSON 比较，说明文本全文比较，
其他输出仅替换时间后逐字比较。测试只使用临时数据库，不读取用户数据。

## 七个 AI 工具

| 工具 | 用途 |
| --- | --- |
| `bridge_overview` | 查看各方状态、认领和未读消息 |
| `update_status` | 更新任务、进度、卡点和下一步 |
| `send_message` | 留言给 `claude`、`codex`、`human` 或 `all` |
| `read_messages` | 读取并标记未读消息，或查看最近消息 |
| `claim_files` | 修改前认领文件，有冲突时整批拒绝 |
| `release_files` | 释放自己认领的文件 |
| `list_projects` | 列出项目并标注项目开关状态 |

除 `list_projects` 外，工具必须传项目根目录的绝对路径 `project`。
正常协作流程是：查看公告板 → 认领文件 → 更新进度 → 留言交接 → 释放认领。
文件认领默认有效期 60 分钟，同一身份再次认领可以续期；它是协作约定，不会锁住文件阻止其他程序写入。

例如，让 AI“给 Claude 留言，请审查边界条件”，对应的 `send_message` 参数是：

```json
{
  "project": "D:/code/example",
  "to": "claude",
  "content": "实现和测试已完成，请检查边界条件。"
}
```

更多协作要求见 [RULES.md](RULES.md)。收到未开启或已关闭的提示后，本次会话停止调用 Bridge。

## 常见问题

- **两边看不到彼此？** 先运行 `status` 检查开关，再确认数据库路径、项目根路径和两个 AI 身份一致。
- **身份是 `unknown`，或任务相互覆盖？** 检查 `BRIDGE_AGENT`。同一身份的多个会话被视为同一个协作方。
- **文件认领失败？** 查看认领人和到期时间，通过 `send_message` 协商；一批里有冲突时不会部分认领。
- **找不到 `bridge` 命令？** 先在 `.venv` 内可编辑安装并激活，或使用 `.\.venv\Scripts\bridge.exe`。
- **不同电脑能直接同步吗？** 当前版本只提供本机 stdio 与 SQLite 协作，没有网络服务或跨设备同步。

## 代码结构

`bridge.py` 保留为兼容入口，包代码位于 `src/bridge_mcp/`：

- `cli.py` / `output.py`：人用命令、观察变化与控制台和重定向输出。
- `server.py`：JSON-RPC 协议及 UTF-8 stdio。
- `tools.py`：工具定义、开关拦截与中文格式化。
- `store.py`：SQLite 连接、业务数据及开关设置。
- `database.py` / `migrations/`：数据库初始化和共享版本迁移。
- `history.py`：交互历史查询和中文格式化。
- `paths.py`：项目和文件路径规范化。

Rust 工作区中，`crates/bridge-core/` 提供数据库、路径、七个工具及中文格式化，
`crates/bridge-mcp/` 负责 stdio 和命令行。核心库不向终端输出，后续桌面软件可直接调用。
协议说明的 Rust 静态资源来自 Python 黄金响应，变更协议时须同步验证两端。

包与 MCP 版本统一使用 `bridge_mcp.__version__`（当前 `0.1.0`）。
后续计划见 [路线图](docs/PLAN.md)，问题和建议请提交到 [GitHub Issues](https://github.com/cynicism66/AI-Bridge/issues)。
