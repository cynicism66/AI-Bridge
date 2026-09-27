# T04：Rust 核心、MCP 服务器、一致性测试与交互历史

## 背景
用户决定把 Bridge 做成公开发布的 Tauri 桌面软件（见 `docs/PLAN.md`、`docs/ARCHITECTURE.md`）。
第一步是用 Rust 重写核心，写出一个**行为和 Python 版完全一致**的 `bridge-mcp.exe`。
怎么证明"完全一致"：先写一套和语言无关的黑盒**一致性测试**，Python 版先通过，然后 Rust 版也必须通过。

同时顺手修掉 T02、T03 审查时留下的四个可靠性问题。**Python 版也要同步修复**，保持两个版本行为一致，因为用户现在用的就是 Python 版。

## 前置条件（用户完成）
本机要先装好 Rust：`winget install Rustlang.Rustup`，然后执行 `rustup default stable`。VS C++ 编译工具本机已经有了。
开工前先确认 `cargo --version` 能正常输出；如果不能，停下来找用户。

## 一、一致性测试 `tests/conformance/`
- 只通过子进程测试：MCP 走 stdio，人用命令走命令行参数。**不许 import `bridge_mcp`**。
- 要测的程序由 `BRIDGE_CMD` 环境变量指定：设置了就当作一个可执行文件的路径；没设置就默认用 `[sys.executable, bridge.py]`。
- 每个测试使用独立的临时 `BRIDGE_DB`。
- **时间控制**：两个版本都支持测试用的环境变量 `BRIDGE_FAKE_NOW`（格式 `YYYY-MM-DD HH:MM:SS`）。设置了就用它作为当前时间。在 README 里注明它只用于测试。
- 包含时间的输出，比较前先用正则替换成统一的占位符；其余文字要**逐字相同**。

至少覆盖：
1. `initialize`（回显 protocolVersion、serverInfo.name、instructions 全文）、`ping`、未知方法、非法 JSON、通知不回复、同一进程连续处理多个请求
2. `tools/list`：和黄金文件 `tests/conformance/golden/tools_list.json`（由 Python 版生成）**完全一致**，包括名称、说明、参数 schema、顺序
3. 7 个工具的正常路径和错误路径：返回文字，以及 `isError` 有没有设置
4. 开关：默认关闭、项目开关、全局关闭优先、关闭时的规定文字、`list_projects` 标注开关状态、切换后正在运行的服务器马上生效
5. 认领：整批冲突、续期、到期（用 `BRIDGE_FAKE_NOW`）、只能释放自己的
6. 消息：收件人可见性、已读按身份分别记录、human 收发
7. 路径：大小写、反斜杠、`..`、项目内的绝对路径、相对 project 报错，以及下面第三部分的新规则
8. 人用命令 `on`、`off`（含 `--global`）、`status`、`show`、`post`、`read` 的输出文字；省略项目参数时取当前目录
9. 数据库互通：Python 版写入的数据 Rust 版能读，反过来也一样（设置了 `BRIDGE_CMD` 时才跑这一项）
10. 首次并发建库：全新数据库上，4 个进程同时执行 `on`，开启不同的项目，全部成功，最后 4 个项目都是开启状态

现有的 Python 单元测试保留不动，它们测的是 Python 的内部实现。

## 二、Rust 实现
按 `docs/ARCHITECTURE.md` 建立工作区：`crates/bridge-core`（库）和 `crates/bridge-mcp`（可执行文件）。

- **允许的依赖**：`rusqlite`（开启 `bundled` 特性）、`serde`、`serde_json`、`chrono`、`clap`（derive）、`anyhow` 或 `thiserror`。其他依赖要先在完成报告里说明理由。
- `bridge-core` 不做输入输出，只提供函数。中文文字格式化也放在这里，因为桌面软件以后也要用。
- 路径规范化要**完全复刻** Python 版的语义：Windows 上转小写、斜杠统一、`..` 消除、项目内的绝对路径转成相对路径。要逐条对照 `src/bridge_mcp/paths.py` 和 `tests/test_paths.py`。
- 时间使用本地时间，格式和 Python 版一样：`%Y-%m-%d %H:%M:%S`。
- 人用命令行包括 `on`、`off`、`status`、`show`、`post`、`read`、`history`（见第四部分）。**不做 `watch`**，以后由桌面软件代替。
- MCP 的 stdio 按 UTF-8 字节读写，每行一条 JSON；stdout 只能输出协议内容，日志一律写到 stderr。
- Rust 单元测试至少覆盖路径规范化和数据库迁移。
- 单个文件不超过 300 行。

## 三、可靠性修复（Python 和 Rust 两边都要做，一致性测试要覆盖）
1. **Windows 项目路径必须明确**（T02、T03 审查遗留）：
   - 允许：带盘符的路径（`C:\x`、`C:/x`）和 UNC 路径（`\\server\share\x`）
   - Git Bash 风格的 `/d/x` 转换成 `d:/x`
   - 其他没有盘符的路径，比如 `\x`、`/x`、`/abc/x`（第一段不是单个字母），一律报错
   - AI 工具和人用命令都要按这个规则处理。命令行一侧要在 `abspath` **之前**先做这个转换
   - 非 Windows 系统保持原来的规则
2. **初始化只做一次**：每个进程对同一个数据库路径只做一次 WAL 设置、建表和迁移，之后的连接直接使用。
3. **首次并发建库**：多个进程同时初始化一个全新数据库时不能失败。在初始化之前先设置 busy timeout，建表和迁移放在 `BEGIN IMMEDIATE` 事务里；切换 WAL 模式时如果遇到 busy，要有限次重试。
4. **数据库版本号**：用 `PRAGMA user_version` 记录版本，当前表结构定为 1。旧库（版本号为 0，但已经有表）要能平滑升级到 1。迁移步骤按编号依次执行，每一步都要幂等。Python 版也要写入版本号，并且在遇到比自己更新的版本号时拒绝写入，给出中文提示。

## 四、交互历史（用户新增需求，数据库版本 2）
用户希望能**按项目查看 AI 交互的历史**。现在只有消息保留了历史：状态每次更新都会被覆盖，认领释放后直接删除。
做法：新增一张事件表，用 **SQLite 触发器自动记录**。这样 Python 版和 Rust 版都不用改业务代码，谁写入数据都会留下历史，两个版本也天然一致。

### 迁移步骤 2（紧接在第三部分第 4 条的版本 1 之后）
- 新表 `events(id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, agent TEXT NOT NULL, kind TEXT NOT NULL, detail TEXT NOT NULL, created_at TEXT NOT NULL)`，并建立索引 `(project, id)`。`detail` 用 `json_object(...)` 存成 JSON。
- 触发器：

| 触发时机 | kind | agent | created_at | detail |
|---|---|---|---|---|
| `status` 插入后 | `status` | NEW.agent | NEW.updated_at | task、progress、blockers、next_step |
| `messages` 插入后 | `message` | NEW.sender | NEW.created_at | recipient、content |
| `claims` 插入后（包括续期） | `claim` | NEW.agent | NEW.claimed_at | path、note、expires_at |
| `claims` 删除后 | 如果 `OLD.expires_at <= datetime('now','localtime')` 就是 `expire`，否则是 `release` | OLD.agent | `datetime('now','localtime')` | path |
| `settings` 插入后 | `switch` | `human` | `datetime('now','localtime')` | scope、enabled。全局开关事件的 project 记为 `global` |

- `INSERT OR REPLACE` 会先删后插，但只要 `recursive_triggers` 保持默认的关闭状态，删除那一步就不会触发删除触发器，所以续期只会记一条 `claim`。要有测试证明这一点。
- **导入旧数据**：迁移时把已有的 `messages` 按 id 顺序导入成 `message` 事件，把 `status` 的现有记录导入成 `status` 事件。
- 两个版本的迁移 SQL 必须完全相同。建议把 SQL 放进一个共享的 `.sql` 文件，两边都从这里读取；Rust 用 `include_str!`。Python 3.10 自带的 SQLite 要确认支持 `json_object`，并写进报告。

### 人用命令 `history`
`bridge history [项目] [--limit N] [--agent 身份] [--kind 类型]`，按时间正序显示，默认显示最近 50 条。显示某个项目的历史时，**同时包含全局开关事件**。每种事件的输出格式如下，两个版本必须逐字一致：
```
[时间] codex 更新状态：任务 xxx｜进度 yyy｜卡点 zzz｜下一步 www    （空的字段省略）
[时间] codex → claude 留言：内容                                   （收件人是 all 时显示"所有人"）
[时间] codex 认领：src/a.py（重构），到期 2026-09-27 21:00:00       （没有备注时省略括号）
[时间] codex 释放认领：src/a.py
[时间] codex 的认领已到期：src/a.py
[时间] human 开启项目 / 关闭项目 / 开启全局总开关 / 关闭全局总开关
```
- 这个功能**不做成 AI 工具**，`tools/list` 的黄金文件不变。桌面软件（T08）会直接读事件表来显示时间线。
- 暂时不做历史清理。

### 一致性测试补充
每种事件都会被记录；续期只记一条；到期和释放能区分开；关闭的项目里 AI 调用工具不会产生事件；导入旧数据的结果正确；从 T03 版本的数据库升级到版本 2 之后，数据完整而且能继续读写；`history` 的输出和各个筛选参数都正确。

## 五、CI
- 现有的 Python 任务继续保留，一致性测试以 Python 版作为默认目标。
- 新增 Rust 任务（`windows-latest`、`ubuntu-latest`）：依次执行 `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`、`cargo build --release -p bridge-mcp`，最后用 `BRIDGE_CMD` 指向编译出来的程序跑一致性测试。

## 不要做
- 不做 Tauri 应用（T08）、安装器（T09）、打包发布（T10）。
- **不要修改用户的全局配置**，不要把用户切换到 Rust 版（T05）。`bridge.py` 不要移动。
- 不改工具名称、参数、返回文字。唯一的例外是第三部分第 1 条新增的路径报错文字。

## 验收标准
- [ ] 一致性测试对 Python 版全部通过；对 Rust 版（`BRIDGE_CMD=target/release/bridge-mcp.exe`）也全部通过
- [ ] `tools/list` 黄金文件由 Python 版生成，Rust 版和它逐字节一致（JSON 规范化以后比较）
- [ ] 数据库互通测试通过：Python 写、Rust 读，以及 Rust 写、Python 读
- [ ] 首次并发建库测试在两个版本上都稳定通过：本机连续跑 20 次，一次都不失败
- [ ] Python 单元测试全部通过（严格模式）；`cargo fmt`、`clippy`、`test` 全部通过
- [ ] CI 的 Python 任务和 Rust 任务都是绿的，完成报告里附上运行链接
- [ ] 手动验证（**要先征得用户同意**，因为这一步会临时修改全局配置）：把 `bridge-mcp.exe` 临时配置成一个**额外的** MCP 服务器（不要替换现有的 bridge），在 Codex 里调用一次 `bridge_overview` 能正常返回。验证完删掉这个临时配置，恢复原样，并在报告里说明。用户不同意就跳过，并在报告里写明跳过了
- [ ] `/d/x`、`\x` 等路径在 AI 工具和命令行两侧的表现都符合第三部分第 1 条
- [ ] 交互历史：数据库版本升到 2，5 种事件都会自动记录，续期只记一条，旧数据导入正确，`history` 命令在两个版本上的输出逐字一致
- [ ] 用用户真实数据库的**副本**（不要碰原件）演练一次从 T03 升级到版本 2，把升级前后的数据条数写进报告
- [ ] Python 单个文件不超过 250 行，Rust 单个文件不超过 300 行

## 完成报告
### 实现与本机验证（2026-09-27）

- 开工前确认 `cargo 1.98.1 (797e8a9bc 2026-08-05)` 可用；读取 Claude 的 #8–12 留言。
  首个提交 `29fbf5f` 收录 Claude 交接的 AGENTS、路线图、架构、用户决策、设计、许可证及任务／审查文档。
- 新建 Rust workspace：`bridge-core` 提供数据库、路径、七个工具和中文格式化；`bridge-mcp`
  提供 UTF-8 stdio MCP 与 `on/off/status/show/post/read/history`。Python 的原入口及客户端配置保持可用。
- 一致性测试通过子进程运行，不导入 Python 实现；工具清单和 instructions 黄金文件由原 Python MCP 响应生成。
  覆盖协议、七个工具、运行中开关变化、消息与身份、原子认领与续期到期、路径、CLI、历史、迁移、四进程初始化及双向互通。
  SQLite 仅用于构造旧版／未来版测试数据库及核对迁移后的持久化数据。
- 两端修复明确的 Windows 项目路径、按数据库路径缓存初始化、busy timeout／有限 WAL 重试、事务迁移和版本保护。
  已运行进程在每次写连接重新检查版本，避免缓存初始化后忽略其他进程的升级。
- 两端共用 `src/bridge_mcp/migrations/001.sql` 和 `002.sql`；迁移到版本 2，回填旧消息与当前状态，
  通过触发器记录状态、消息、认领、释放／到期和开关。历史支持项目加全局事件、最近 N 条、身份及类型筛选。
  旧表允许空值，历史迁移／触发器将缺失的必填文本转为空串，兼容既有 Python 单元测试和早期不完整记录。
- 中文 README 补充 Rust 构建、测试、history、路径规则、数据库版本和测试时钟说明；CI 保留四个 Python 任务并新增两个 Rust 任务。
- 本机严格 Python 测试：75 项，74 通过、1 项仅 Rust 目标的双向互通按条件跳过；原有 61 项测试文件未修改。
  Rust 目标：14 项一致性测试全部通过，含双向互通。`cargo fmt --check`、Clippy（warnings 视为错误）、
  4 项 Rust 单元测试及 release 构建通过。Python／Rust 文件分别在 250／300 行以内。
- 首次并发建库：Python 连续 20 轮通过（8.829 秒），Rust 连续 20 轮通过（3.562 秒）；
  每轮独立临时数据库、4 个进程同时开启不同项目，共 160 次进程启动，零失败。

### 真实数据库副本迁移

通过 SQLite 只读连接的 backup API 取得一致快照（包含 WAL 中已提交内容），再复制出两个临时数据库分别由 Python／Rust 升级。
没有对原数据库执行迁移，也没有输出消息内容。两份副本结果一致：

| 项目 | 升级前 | Python 升级后 | Rust 升级后 |
|---|---:|---:|---:|
| user_version | 0 | 2 | 2 |
| status | 2 | 2 | 2 |
| messages | 13 | 13 | 13 |
| reads | 12 | 12 | 12 |
| claims | 23 | 23 | 23 |
| settings | 1 | 1 | 1 |
| 回填 message 事件 | — | 13 | 13 |
| 回填 status 事件 | — | 2 | 2 |

本机 Python 3.14.7 / SQLite 3.50.4 的 `json_object('ok',1)` 返回 `{"ok":1}`。
Python 3.10 的兼容性由 CI 的两种操作系统任务验证；测试会显式执行 json_object 并通过迁移与全部触发器场景。

### 依赖与边界说明

- 直接依赖仅使用允许清单：rusqlite 0.32（bundled SQLite，独立运行）、chrono 0.4（本地时间）、
  serde_json 1（JSON 协议／事件）、clap 4 derive（CLI）、anyhow 1（错误传播）；精确版本提交在 Cargo.lock。
  未新增清单外直接依赖，Python 仍只用标准库。
- `BRIDGE_FAKE_NOW` 控制业务时间及认领清理比较；开关与删除认领触发器严格使用任务书指定的
  `datetime('now','localtime')`，使用真实本地时间。测试用过去／未来时间区分到期与释放，比较时归一化时间。
- 空历史输出空行；`--limit 0` 不返回事件、负数报错。其余既有成功输出及工具定义保持一致。
- 手动额外 MCP 验收：程序就绪后已向用户请求同意，等待答复；尚未修改全局配置。
- CI 运行链接与最终验收状态：提交后补充。
