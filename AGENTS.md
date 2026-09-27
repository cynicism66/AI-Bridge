# Bridge 项目约定

Bridge 让在同一个项目里工作的多个 AI（Claude、Codex）互相看到状态、留言、认领文件。
目标是做成一个**公开发布的 Windows 桌面软件**：托盘常驻加主窗口，安装时自动接入 Claude 和 Codex。

## 分工
- **Claude**：规划、拆任务、写验收标准、审代码。任务书在 `docs/tasks/`，路线图在 `docs/PLAN.md`，架构在 `docs/ARCHITECTURE.md`，**用户的决定在 `docs/DECISIONS.md`（冲突时以它为准）**。
- **Codex**：按任务书实现代码和测试。
- **用户**：拍板。任务书没写清楚的地方，先问用户，不要自己猜。

## 工作流程
1. 只做当前指定的任务书（如 `docs/tasks/T04-*.md`），不要顺手做其他任务的内容。
2. 完成后逐条对照任务书里的「验收标准」自查，全部满足再提交。
3. 在任务书末尾的「完成报告」里写：做了什么、偏离任务书的地方及原因、遗留问题。
4. 提交信息格式：`T04: 简短说明`。

## 代码组成
| 目录 | 语言 | 状态 |
|---|---|---|
| `legacy/` | Python | **已冻结**，仅作为历史参考；历史单元测试继续运行 |
| `crates/` | Rust | **现役版本**：数据库、工具、MCP 服务器、命令行；新功能只在 Rust 实现 |
| `app/` | Tauri + React + TypeScript | 桌面软件（T08 开始） |
| `tests/conformance/` | Python | **Rust 黑盒契约测试**：通过子进程和 stdio 验证，不再测试冻结版 |
| `migrations/` | SQL | 数据库迁移的共享来源 |

## 硬性约束
- **Windows 优先**：路径、编码、换行都要在 Windows 上正确。MCP 的 stdio 一律按 UTF-8 字节读写，每行一条 JSON。
- **不能打断正在使用的配置**：用户的全局配置引用 `%LOCALAPPDATA%\AI Bridge\bin\bridge-mcp.exe`，不得改为编译目录。更新前退出两端，由用户在普通 PowerShell 执行安装脚本，避免 MSIX 文件重定向。
- **数据库兼容**：默认 `~/.bridge/bridge.db`，迁移必须保留已有数据。T05 的版本 2 与冻结版兼容；后续功能只在 Rust 实现，不再要求冻结版通过新契约。
- 面向 AI 的工具说明和返回内容都用中文。
- 测试不许读写真实数据库，必须通过 `BRIDGE_DB` 指向临时文件。
- **Python 部分**：只用标准库，最低支持 3.10；历史测试在 `legacy/` 下运行 `python -X dev -W error -m unittest discover -s tests -v`；Rust 契约测试在根目录运行 `python -X dev -W error -m unittest discover -s tests/conformance -t . -v`。
- **Rust 部分**：stable 工具链；提交前 `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 都必须通过。
- **依赖要克制**：Rust 依赖只允许用任务书里列出的。确实需要新增时，在完成报告里写明理由，由 Claude 审查决定是否保留。
