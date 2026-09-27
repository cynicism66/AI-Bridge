# Bridge

Bridge 是一个让 Claude 和 Codex 在同一项目中互相看到工作状态、留言、认领文件的 MCP 服务器。
数据保存在 SQLite 中，按项目根目录区分；支持 Python 3.10 及以上，运行时只使用 Python 标准库。

## 直接运行

现有兼容入口保持可用，无需安装包：

```powershell
python D:\Bridge\bridge.py
```

无参数时通过 stdio 处理 MCP 请求，等待客户端发送 JSON-RPC 消息。
协议输入和输出均使用 UTF-8 字节，每行一条 JSON 消息。

## 配置到 Claude 和 Codex

如果客户端已经指向 `D:\Bridge\bridge.py`，入口配置可以继续使用。
下面是新接入时的示例，将 Python 和项目路径替换为实际绝对路径；两端使用同一数据库。
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

## 数据库与身份

- `BRIDGE_AGENT`：当前身份，通常为 `claude` 或 `codex`，未设置时为 `unknown`。
- `BRIDGE_DB`：优先使用此环境变量指定的数据库路径。
- 未设置 `BRIDGE_DB` 时，默认使用 `Path.home() / ".bridge" / "bridge.db"`，自动创建目录。
  Windows 上 `Path.home()` 使用 `USERPROFILE`；本机自定义的 `HOME` 不影响它。

默认数据库位置已从原型的脚本同目录改为用户目录。本任务不自动迁移旧数据；旧的
`D:\Bridge\bridge.db` 会保留。若需要继续使用旧数据，可在两个客户端的环境配置中
显式设置 `BRIDGE_DB=D:\Bridge\bridge.db`。已有运行中的原型进程仍使用启动时的数据库路径。

## 命令行查看

```powershell
# 列出用过 Bridge 的项目
python D:\Bridge\bridge.py show

# 查看指定项目的状态、认领和消息
python D:\Bridge\bridge.py show D:\Bridge
```

查看指定项目时，命令行身份为 `human`，不会把显示的未读消息标为已读。

## 项目内安装与测试

仅在项目内虚拟环境中安装，不安装到全局 Python：

```powershell
cd D:\Bridge
python -m venv .venv
.\.venv\Scripts\python.exe -m pip install -e .
.\.venv\Scripts\bridge.exe show
.\.venv\Scripts\python.exe -m bridge_mcp
python -m unittest discover -s tests
```

激活 `.venv` 后也可直接运行 `bridge show` 和 `python -m bridge_mcp`。
`setuptools` 仅用于构建安装包，项目没有运行时依赖。冒烟测试用 `unittest` 和子进程，
每次通过 `BRIDGE_DB` 指向独立临时数据库，不读写真实数据库。

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

## 代码结构

`bridge.py` 是兼容入口；包代码在 `src/bridge_mcp/`：

- `cli.py`：命令行与 MCP 启动入口。
- `server.py`：JSON-RPC 协议分发和 UTF-8 stdio。
- `tools.py`：工具定义、参数处理和中文格式化。
- `store.py`：SQLite 连接、建表及结构化数据读写。
- `paths.py`：项目路径与文件路径规范化。

依赖方向从命令行、协议进入工具，再进入数据和路径模块。包版本为 `0.1.0`；
MCP `serverInfo.version` 保留原型的 `1.0.0`，遵守 T01 的接口兼容要求。
