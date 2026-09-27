# Bridge 架构

## 组成

```
Claude app ──stdio──▶ bridge-mcp.exe ─┐
Codex app  ──stdio──▶ bridge-mcp.exe ─┼──▶ ~/.bridge/bridge.db（SQLite，WAL）
AI Bridge.exe（托盘 + 主窗口） ───────┘
```

- **bridge-mcp.exe**：控制台程序。不带参数时作为 MCP 服务器，由 Claude 和 Codex 各自启动一份，通过 stdio 通信。带子命令时是人用的命令行（`on`、`off`、`status` 等）。
- **AI Bridge.exe**：Tauri 桌面软件，常驻托盘。它直接调用 Rust 核心库读写同一个数据库，不经过 MCP。
- **数据库是唯一的共享点**。所有进程之间不直接通信，只读写同一个 SQLite 文件。简单、没有端口冲突，任何一个进程崩溃都不影响其他进程。

## Rust 工作区

```
Cargo.toml                 # workspace
crates/
├─ bridge-core/            # 库：路径规范化、数据库（含迁移）、开关、7 个工具及中文文字
└─ bridge-mcp/             # 可执行文件：MCP stdio 服务器 + 人用命令行
app/                       # T08：Tauri 应用（src-tauri 依赖 bridge-core）
```

`bridge-core` 不做任何输入输出，只提供函数，由 MCP 服务器、命令行、桌面软件三处共用。

## 数据库兼容和版本
- 用 `PRAGMA user_version` 记录表结构版本：版本 1 等于 Python 版 T03 的表结构；版本 2 新增交互历史（`events` 表和触发器，T04）；版本 3 新增 AI 管理（T06，只在 Rust 版实现）。
- 以后改表结构只能通过有编号的迁移步骤，每一步都要能在旧数据上幂等执行。版本 1 和 2 的迁移 SQL 由 Python 版和 Rust 版共用同一个文件。
- 两个版本都会写版本号。遇到比自己新的版本时拒绝写入，并给出中文提示，防止旧程序把新数据写坏。
- 项目知识（目标、路线图、任务书、决定）存放在项目仓库里。数据库只存协作过程，并且可以随时导出（T07）。

## 一致性测试
`tests/conformance/` 是一组黑盒测试：用 `BRIDGE_CMD` 环境变量指定要测的命令（默认是 `python bridge.py`），通过子进程的 stdio 和命令行来验证行为。
**Python 版和 Rust 版必须跑同一套一致性测试并且都通过。** 这是判断重写是否正确的唯一标准。
