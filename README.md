# AI Bridge

[![测试](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml/badge.svg)](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml)

**让 Claude 和 Codex 在同一个项目里知道彼此在做什么。**

AI Bridge（简称 Bridge）是一个让 Claude 和 Codex 互相看到工作状态、留言、认领文件的 MCP 服务器。
数据保存在 SQLite 中，按项目根目录区分；支持 Python 3.10 及以上，运行时只使用 Python 标准库。

[项目仓库](https://github.com/cynicism66/AI-Bridge) · [协作规则](RULES.md) · [开发路线图](docs/PLAN.md)

## 能解决什么问题

当两个 AI 同时处理同一份代码时，常见的问题是重复工作、修改同一个文件，以及不知道对方的进度。
Bridge 提供一个共享公告板，供 AI 主动查询和更新：

- **同步状态**：记录任务、进度、卡点与下一步。
- **互相留言**：支持定向消息、广播消息与按身份记录的已读状态。
- **认领文件**：修改前声明负责哪些文件；遇到其他身份的认领时，整批拒绝本次认领。
- **区分项目**：一份数据库可以服务多个项目，每个项目使用绝对根路径区分。
- **方便人查看**：在终端直接查看项目公告板和最近消息。

例如，Claude 负责规划和审查，Codex 负责实现和测试：双方通过 Bridge 交接进度和问题，
你可以随时从命令行查看。具体分工由你决定，Bridge 不会自动分配任务。

## 快速开始

准备 Git 和 Python 3.10 或更高版本。下面以 Windows PowerShell 为例，项目放在 `D:\Bridge`。

```powershell
git clone https://github.com/cynicism66/AI-Bridge.git D:\Bridge
cd D:\Bridge
python --version
python .\bridge.py show
```

如果你已经有本地仓库，直接进入原目录即可。首次查看时显示“还没有项目使用过 Bridge”是正常的。
然后为 Claude 和 Codex 分别添加下方 MCP 配置，并把 [RULES.md](RULES.md) 的规则加入各自的项目说明。

### 直接运行服务器

无需安装包即可运行；已有客户端也可以继续使用这个入口：

```powershell
python D:\Bridge\bridge.py
```

无参数时通过 stdio 处理 MCP 请求，等待客户端发送 JSON-RPC 消息。
协议输入和输出均使用 UTF-8 字节，每行一条 JSON 消息。
手动运行后没有提示、一直等待输入，是服务器在等待客户端连接时的正常表现。

## 配置到 Claude 和 Codex

如果客户端已经指向 `D:\Bridge\bridge.py`，入口配置可以继续使用。
下面是新接入时的示例，将 Python 和项目路径替换为实际绝对路径；两端使用同一数据库。
可运行 `python -c "import sys; print(sys.executable)"` 查找 Python 可执行文件的完整路径。
项目中的 `RULES.md` 提供 AI 协作规则，可放入相应客户端的项目说明中。

Claude 的本地 stdio MCP 配置示例（合并到客户端的 `mcpServers` 配置）：

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

Codex 的 `command`、`args` 和 `env` 配置参见 [官方 MCP 文档](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)。

配置完成后重启相应客户端，或重新加载 MCP 服务器，然后在两个客户端中分别发送：

> 请用 Bridge 查看 D:\Bridge 的公告板，并汇报你当前负责的任务。

如果两个客户端能够看到彼此的状态，说明接入成功。处理其他项目时，将 `D:\Bridge` 换成那个项目的绝对根路径；
服务器代码所在目录与正在协作的项目目录可以不同。

## 数据库与身份

- `BRIDGE_AGENT`：当前身份，通常为 `claude` 或 `codex`，未设置时为 `unknown`。
- `BRIDGE_DB`：优先使用此环境变量指定的数据库路径。
- 未设置 `BRIDGE_DB` 时，默认使用 `Path.home() / ".bridge" / "bridge.db"`，自动创建目录。
  Windows 上 `Path.home()` 使用 `USERPROFILE`；自定义的 `HOME` 不影响它。

默认数据库位置已从早期原型的脚本同目录改为用户目录。当前版本不自动迁移旧数据；旧的
`D:\Bridge\bridge.db` 会保留。若需要继续使用旧数据，可在两个客户端的环境配置中
显式设置 `BRIDGE_DB=D:\Bridge\bridge.db`。已有运行中的原型进程仍使用启动时的数据库路径。

同一台电脑、同一个系统用户下，两个客户端使用默认路径即可共享数据。不同系统用户或显式指定路径时，
请确保它们实际访问同一份 SQLite 文件。自定义 `BRIDGE_DB` 的父目录需要预先存在。
公告板数据存放在本地；是否把读取到的内容发送给模型服务，由所用 AI 客户端决定。

## 命令行查看

```powershell
# 列出用过 Bridge 的项目
python D:\Bridge\bridge.py show

# 查看指定项目的状态、认领和消息
python D:\Bridge\bridge.py show D:\Bridge
```

查看指定项目时，命令行身份为 `human`，不会把显示的未读消息标为已读。
未读消息会按照 `human` 身份筛选，“最近消息”区域则显示该项目最近 20 条消息。

如果客户端配置了自定义数据库，命令行查看时也要使用相同路径：

```powershell
$env:BRIDGE_DB = 'D:\Bridge\bridge.db'
python D:\Bridge\bridge.py show D:\Bridge
```

## 项目内安装与测试

仅在项目内虚拟环境中安装，不安装到全局 Python：

```powershell
cd D:\Bridge
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -e .
.\.venv\Scripts\bridge.exe show
.\.venv\Scripts\python.exe -m bridge_mcp
python -X dev -W error -m unittest discover -s tests -v
```

激活 `.venv` 后也可直接运行 `bridge show` 和 `python -m bridge_mcp`。
`setuptools` 仅用于构建安装包，项目没有运行时依赖。测试用标准库 `unittest`，
每次通过 `BRIDGE_DB` 指向独立临时数据库，不读写真实数据库。
每次推送和拉取请求会在 Windows／Linux × Python 3.10／3.14 上运行测试，不安装第三方依赖。

## 七个工具

| 工具 | 用途 |
| --- | --- |
| `bridge_overview` | 查看各方状态、文件认领和未读消息 |
| `update_status` | 更新任务、进度、卡点和下一步 |
| `send_message` | 给其他 AI 留言 |
| `read_messages` | 读取并标记未读消息，或查看最近消息 |
| `claim_files` | 修改前认领文件，有冲突时整批拒绝 |
| `release_files` | 释放自己认领的文件 |
| `list_projects` | 列出使用过 Bridge 的项目 |

除 `list_projects` 外，调用工具时都要传入项目根目录的绝对路径 `project`。
建议流程：查看公告板 → 认领文件 → 工作中更新状态 → 完成后释放文件。

## 一次完整的协作示例

可以直接用自然语言要求 AI 操作 Bridge：

1. **了解现场**：“开始前查看这个项目的 Bridge 公告板，确认另一方正在做什么。”
2. **认领文件**：“先认领 `src/example.py`，任务是实现参数校验。”
3. **汇报进度**：“把状态更新为：参数校验已完成，下一步补充测试。”
4. **请求协助**：“通过 Bridge 给 Claude 留言，请检查边界条件和异常提示。”
5. **完成交接**：“测试通过后释放认领，并更新任务状态和遗留问题。”

以下是工具调用的参数示例，供了解 MCP 的开发者参考；普通使用时让 AI 发起调用即可。

调用 `update_status`：

```json
{
  "project": "D:/code/example",
  "task": "实现参数校验",
  "progress": "实现完成，正在补充测试",
  "blockers": "",
  "next_step": "测试通过后交给 Claude 审查"
}
```

调用 `claim_files`：

```json
{
  "project": "D:/code/example",
  "files": ["src/example.py", "tests/test_example.py"],
  "note": "实现参数校验及测试",
  "ttl_minutes": 60
}
```

调用 `send_message`：

```json
{
  "project": "D:/code/example",
  "to": "claude",
  "content": "参数校验和测试已完成，请检查边界条件。"
}
```

文件认领默认有效期为 60 分钟，同一身份再次认领可以更新到期时间。
认领是协作约定，不会阻止编辑器或其他进程写入文件，双方需要遵守 [协作规则](RULES.md)。

## 常见问题

**两边看不到彼此的状态？**

检查 `BRIDGE_DB` 是否指向同一文件、`project` 是否为同一项目根目录，
以及 `BRIDGE_AGENT` 是否分别设置为 `claude` 和 `codex`。
升级早期原型后，需要重新启动两边的 Bridge 进程，让它们采用同一数据库规则。

**两边都显示为同一个身份，或者是 `unknown`？**

检查客户端传给服务器的 `BRIDGE_AGENT` 环境变量。同一项目中，同一身份的状态会被后续更新覆盖。
当前身份按名称区分；两个同时运行的 Codex 会话若都使用 `codex`，会被视为同一个协作方。

**文件认领失败？**

先查看公告板中的认领人和到期时间，再通过 `send_message` 协商释放或等待到期。
一批文件中只要有其他身份的认领冲突，本次就不会认领其中任何文件。

**能在不同电脑之间直接同步吗？**

当前版本通过本机 stdio 与 SQLite 协作，没有提供网络服务或跨设备同步功能。

**为什么安装后的 `bridge` 命令找不到？**

先确认已经在 `.venv` 中执行 `pip install -e .`，再激活虚拟环境，
或直接运行 `.\.venv\Scripts\bridge.exe show`。使用 `python bridge.py` 则无需安装。

## 代码结构

`bridge.py` 是兼容入口；包代码在 `src/bridge_mcp/`：

- `cli.py`：命令行与 MCP 启动入口。
- `server.py`：JSON-RPC 协议分发和 UTF-8 stdio。
- `tools.py`：工具定义、参数处理和中文格式化。
- `store.py`：SQLite 连接、建表及结构化数据读写。
- `paths.py`：项目路径与文件路径规范化。

依赖方向从命令行、协议进入工具，再进入数据和路径模块。
包版本和 MCP `serverInfo.version` 统一读取 `bridge_mcp.__version__`（当前为 `0.1.0`）。

已提供路径、存储、认领、协议、连续请求和并发测试，后续计划见 [开发路线图](docs/PLAN.md)。
反馈问题或提出建议，请前往 [GitHub Issues](https://github.com/cynicism66/AI-Bridge/issues)。
