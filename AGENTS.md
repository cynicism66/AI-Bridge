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
| `bridge.py`、`src/bridge_mcp/` | Python | **现役版本**，用户的全局配置正在使用。进入维护模式：只修 bug，不加功能 |
| `crates/` | Rust | 新核心：数据库、工具、MCP 服务器、命令行 |
| `app/` | Tauri + React + TypeScript | 桌面软件（T08 开始） |
| `tests/conformance/` | Python | **一致性测试**：只通过子进程和 stdio 做黑盒测试，Python 版和 Rust 版必须同时通过 |

## 硬性约束
- **Windows 优先**：路径、编码、换行都要在 Windows 上正确。MCP 的 stdio 一律按 UTF-8 字节读写，每行一条 JSON。
- **不能打断正在使用的配置**：`D:\Bridge\bridge.py` 被用户的全局配置直接引用。在 T05 切换到 Rust 版之前，这个入口必须一直能用，并且它的位置不能移动。
- **数据库兼容**：Python 版和 Rust 版读写同一个数据库（默认 `~/.bridge/bridge.db`），表结构必须互相兼容，两者要能同时运行。
- 面向 AI 的工具说明和返回内容都用中文。
- 测试不许读写真实数据库，必须通过 `BRIDGE_DB` 指向临时文件。
- **Python 部分**：只用标准库，最低支持 3.10；测试命令 `python -X dev -W error -m unittest discover -s tests -v`。
- **Rust 部分**：stable 工具链；提交前 `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace` 都必须通过。
- **依赖要克制**：Rust 依赖只允许用任务书里列出的。确实需要新增时，在完成报告里写明理由，由 Claude 审查决定是否保留。
