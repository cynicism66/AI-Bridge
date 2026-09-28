# T05：切换到 Rust 版

## 背景
T04 证明了 Rust 版和 Python 版的行为一致。本任务把用户的 Claude 和 Codex **正式切换到 Rust 版**，然后冻结 Python 版。之后的新功能只写在 Rust 里（决策见 `docs/PLAN.md` 的顺序说明）。
同时处理 T04 审查留下的 5 条遗留问题。T04 跳过的手动验收，在本任务第三部分的真实切换中完成。

## 一、固定的安装位置
`target/release/` 是编译目录，`cargo clean` 会把它清空，不能拿来给配置引用。
- 新增脚本 `scripts/install-local.ps1`：先编译 release，再把 `bridge-mcp.exe` 复制到 `%LOCALAPPDATA%\AI Bridge\bin\`。以后 T09 的安装器也用这个位置。
- **exe 被占用时的处理**：Claude 或 Codex 运行时，旧的 exe 被占用，没法覆盖。脚本要能检测到这种情况，给出中文提示："请先退出 Claude 和 Codex 再安装"，然后安全退出。不许留下半个文件。
- 脚本可以重复执行，第二次执行时直接覆盖更新。

## 二、清理遗留问题（T04 审查第 1、2、3、5 条）
1. **迁移 SQL 挪到中立位置**：从 `src/bridge_mcp/migrations/` 移到根目录的 `migrations/`，同时更新 Rust 的 `include_str!` 路径和 Python 版的读取路径。
2. **工具定义只保留一份**：`crates/bridge-core/resources/` 作为唯一来源，一致性测试的黄金比对也直接读这里的文件。删除 `tests/conformance/golden/` 下重复的 `tools_list.json` 和 `instructions.txt`。
3. **错误提示改成中文**：参数类型错误时，不要再返回 Python 风格的异常文字（`invalid literal for int() ...`），改成比如 `limit 必须是整数`、`ttl_minutes 必须是整数`。其他所有返回文字不变。
4. **旧数据库**：`D:\Bridge\bridge.db`（以及 `-wal`、`-shm`）是 T01 之前留下的旧位置。在完成报告里列出它的数据条数，**征得用户同意后**再删除。

## 三、切换配置（需要用户同意，并且用户要在场）
**动手之前，先在公告板上留言给 human，说明要改哪两个文件、改成什么样，等用户明确同意后再执行。**
1. 备份 `~/.claude.json` 和 `~/.codex/config.toml`，文件名加上 `.bak-t05`。
2. 把 `bridge` 服务器的 `command` 改成 `%LOCALAPPDATA%\AI Bridge\bin\bridge-mcp.exe` 的完整路径，`args` 清空，`BRIDGE_AGENT` 保持不变（claude 和 codex 各自原来的值）。**只改这一项，文件里的其他内容一个字节都不能动。** 改完用 diff 对照备份，证明只有这几行变了。
3. 请用户重启 Claude app 和 Codex app。
4. **真实环境验证**（就是 T04 跳过的那项手动验收）：
   - Codex 调用一次 `bridge_overview`（project=D:/Bridge），把结果写进报告；
   - 在公告板上给 claude 留言，请 Claude 那边也调用一次并回复确认；
   - 用任务管理器或 `Get-Process` 确认实际运行的是 `bridge-mcp.exe`，不是 python。
5. **回滚方法**写进 README：把 `.bak-t05` 备份复制回去，然后重启两个 app。

## 四、冻结 Python 版
- 切换并验证成功后，把 `bridge.py`、`src/bridge_mcp/` 和 Python 单元测试移到 `legacy/` 下，并在 `legacy/README.md` 里说明：已冻结，只作为历史参考。
- **一致性测试的默认目标改成 Rust 版**（找不到编译好的 exe 就报错，并提示先编译）。不再要求对 Python 版运行一致性测试。第二部分第 3 条改了错误提示之后，Python 版本来就过不了了，这是预期内的。
- CI：去掉"一致性测试 → Python"这一步。`legacy/` 下的 Python 单元测试保留在 CI 里继续跑，防止历史参考代码坏掉。Rust 任务照旧。
- 根目录 README 改成以 Rust 版为主，Python 版只保留一句说明加上指向 `legacy/` 的链接。
- 更新 `AGENTS.md` 的"代码组成"表和"不能打断正在使用的配置"那一条（入口改成了 `%LOCALAPPDATA%\AI Bridge\bin\bridge-mcp.exe`）。

## 安装环境补充（Claude 公告板 #25、#29）

- 安装前通过 Windows 包身份 API 检查当前进程及祖先宿主，发现 MSIX 宿主时拒绝安装，提示用户从开始菜单打开普通 PowerShell 执行。只检查当前 PowerShell 不够：它可能没有包身份，而祖先仍是打包应用。
- 安装后独立检查 `%LOCALAPPDATA%\Packages\*\LocalCache\Local\AI Bridge\bin\bridge-mcp.exe`，发现本轮写入的副本时必须报错退出；不得因目标路径表面存在就报告成功。
- README 明确安装方式；报告如实区分真实宿主测试和临时目录模拟，记录本次重定向及两个副本的处理。

## 不要做
- 不做 T06 及以后的功能（按 AI 开关、职务、初始化等）。
- 不改工具名称、参数和成功时的返回文字；唯一的例外是第二部分第 3 条的错误提示。
- **没有用户明确同意，不许改全局配置，也不许删除旧数据库。**

## 验收标准
- [x] `scripts/install-local.ps1` 能正常安装；exe 被占用时给出中文提示并安全退出（完成报告里写明你是怎么测试的）
- [x] 迁移 SQL 在 `migrations/` 下，工具定义只剩一份，错误提示改成了中文，一致性测试已同步更新
- [x] 两个配置文件和备份的 diff 只有 `bridge` 那几行
- [x] 真实环境验证：Codex 和 Claude 都通过 Rust 版调用成功，实际运行的进程是 `bridge-mcp.exe`
- [x] Python 版已移到 `legacy/`，`legacy/` 下的单元测试在 CI 里通过
- [x] 一致性测试（目标为 Rust）、`cargo fmt`、`clippy`、`test` 全部通过；CI 全绿，完成报告附上链接
- [x] README 写了安装、回滚、从源码编译的方法
- [x] 旧数据库：已征得用户同意并删除，或者用户选择保留，两种情况都要在报告里写明

## 完成报告
### 2026-09-28：实现、真实切换及冻结完成

#### 实现及本地验证
- T04 按 Claude #18 的交接改为“用户决定跳过，并入 T05”，已结项；首提交 `fa7dbdc` 收录 T04 审查、PLAN 和 T05 任务。
- 安装脚本先 release 编译，同目录暂存、SHA256 校验、原子替换；占用时中文提示并安全退出。没有新增 Rust 或 Python 运行依赖。
- 加入 MSIX 检测前，Windows PowerShell 5.1 和 PowerShell 7 的安装集成测试均验证了首次安装、重复覆盖、真实 MCP 子进程占用时拒绝（hash 不变、无临时残留）、退出后更新。最终脚本的普通宿主路径已由 Windows CI 再次验证通过。
- PowerShell 5.1 将 `$null` 转为空备份路径的问题已用 `[NullString]::Value` 修复；脚本使用 UTF-8 BOM，支持 5.1 中文解析。
- 两份 SQL 移到根目录 `migrations/`；Rust 编译包含路径和冻结版读取路径同步更新。删除重复 golden 文件，现役工具规格只读 `crates/bridge-core/resources/`。
- 整数类型错误变为 `limit 必须是整数`、`ttl_minutes 必须是整数`，其余工具文字与行为保持不变。
- 双方实际验证通过后才移动 `bridge.py`、`src/bridge_mcp/`、历史单元测试到 `legacy/`；同时移动 Python 项目元数据并删除失效的包内 SQL 配置。历史代码需要完整仓库，不独立发布。
- 本地最终检查全部通过：`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（4 项）、release 编译、Rust 黑盒契约测试（13 项）、legacy 严格模式历史单元测试（61 项）。
- 黑盒测试默认 Rust，缺少产物会提示先编译；移除不再适用的 Python 双向互通测试。CI 保留 Windows/Ubuntu × Python 3.10/3.14 历史测试，以及两平台 Rust 检查，并增加 Windows PS5.1/7 安装测试。

#### 配置授权及真实环境验证
- 公告板 human #21 说明修改方案，用户明确回复“同意现在切换，我可以重启两端”后才执行。
- 完整备份为 `C:\Users\wangq\.claude.json.bak-t05`、`C:\Users\wangq\.codex\config.toml.bak-t05`。
- 两份配置仅将 bridge.command 从 `C:\Python314\python.exe` 改为 `C:\Users\wangq\AppData\Local\AI Bridge\bin\bridge-mcp.exe`，bridge.args 从 Python 入口清空。各自 BRIDGE_AGENT 保持 claude/codex。
- 修改当时通过 JSON/TOML 语义比较核对仅这两个字段变化，并逆向替换后与备份逐字节比较，确认其他内容不变。应用启动后自行更新的状态字段不在本次编辑范围内。
- 用户回复“重启完成”。Codex 在实际 MCP 会话中调用 `bridge_overview(project=D:/Bridge)` 成功，身份 codex，正常返回状态、认领和消息。
- Claude 初次重启时因下述 MSIX 路径问题连接失败；从外部 shell 补装后，用户在 Claude `/mcp` 中点击 Reconnect。Claude 的实际 MCP 调用成功，身份 claude，公告板 #28（06:14:29）确认并读取 Codex #26。这里是重新连接，未记作第二次重启。
- `Get-CimInstance Win32_Process` 复核两个 `bridge-mcp.exe`：PID 12528（父进程 codex.exe）和 PID 21696（父进程 claude.exe），显示路径均为固定安装路径；没有 Python Bridge 进程。
- README 已改为中文 Rust 使用说明，写明安装、编译、测试和回滚。冻结后回滚要先从 legacy 恢复原入口/源码，再还原两份配置备份并重启；迁移读取同时支持 legacy 和恢复后的原目录。

#### MSIX 重定向发现、修复及限制
- 初次通过 MSIX Codex 的 exec → CLI → PowerShell 安装时，LOCALAPPDATA 的写入实际进入 Codex 的私有目录。仅在 Codex 内检查表面目标存在会误判成功，Claude 因而无法启动。
- 私有副本为 `C:\Users\wangq\AppData\Local\Packages\OpenAI.Codex_2p2nqsd0c76g0\LocalCache\Local\AI Bridge\bin\bridge-mcp.exe`，大小 2,787,840 字节；记录的修改时间为 2026-09-28 05:57:24.693 +08:00，与首次 release 安装阶段一致。
- Claude 从外部 shell 安装到真实 LocalAppData 后，两端实际调用通过。真实路径、私有副本和当前 release 产物的 SHA256 均为 `733632F25E810DEC453D8E5FDFAA4DA4E3BFA03CE824FED1F8D60B48E86BA6B4`。
- 按 Claude #25/#29 补充要求，脚本用 `GetCurrentPackageFullName` 检查自身，再用 `GetPackageFullName` 检查祖先：本机 PowerShell 无包身份，但其祖先 ChatGPT.exe 有 Codex MSIX 包身份。最终代码在此真实宿主中、PS5.1/7 下均拒绝安装，中文提示普通 PowerShell，且未创建测试安装目录。
- 独立后检扫描 `Packages/*/LocalCache/Local/AI Bridge/bin/bridge-mcp.exe` 的本轮写入时间。暂存文件主动刷新时间，避免复制保留源文件时间导致漏检；发现私有副本更新则返回非零退出码，不假报成功。临时目录测试验证旧副本不误报、本轮写入被拒绝、没有删除副本；这是受控模拟，不冒充再次真实重定向测试。
- 现有两份相同版本的 exe 均保留，未删除或替换运行中的文件。以后升级应先退出两端、在普通 PowerShell 安装，并处理旧 LocalCache 副本（同步版本，或经确认移除后验证 Codex 使用真实路径）；只更新真实目录可能让 Codex 继续使用旧副本。本次未执行“删除私有副本后回退真实目录”的试验，不宣称该路径已经验证。新增防护阻止后续从打包宿主重复产生误安装。

#### 旧数据库与偏离
- 用户明确选择“保留旧数据库，并写入报告”。只读核对 `D:\Bridge\bridge.db` 为版本 0：status=1、messages=2、reads=0、claims=0。主文件 36,864 字节，-shm 32,768 字节，-wal 0 字节；三者均保留，继续由 Git 忽略，不上传。
- 相对最初任务增加了 MSIX 宿主与复制后检查，原因是正式切换发现真实故障，已将 Claude 的补充要求写入任务书。Python 项目元数据随归档移动，避免根目录继续宣传旧入口。
- 未做 T06 及以后的功能，未改变开关、身份或其他全局设置。自动化测试全部使用临时数据库；实际公告板调用属于已授权的真实环境验收。
- 首轮 CI（实现 `c9fbe7f`）的 5 个任务通过，Windows 安装测试暴露 PS5.1 的 Process 在创建 stdin writer 时继承宿主编码并预写 BOM，首条 ping 因此返回 Parse error。已在启动前暂时设置 UTF-8 无 BOM、启动后恢复宿主编码，再通过 BaseStream 写字节；本地强制带 BOM 宿主编码可复现旧问题，修复后 PS5.1/7 均返回 `result={}`。生产 Rust 代码无需改动。
- 修复提交 `84ba0df` 的 [GitHub Actions 验收](https://github.com/cynicism66/AI-Bridge/actions/runs/36355512199) 六个任务全部成功：Windows/Ubuntu 的 Rust 检查和契约测试、两平台 Python 3.10/3.14 历史测试；Windows Rust 任务包含 PS5.1/7 的首次安装、重复覆盖、真实 MCP 占用保护及退出后重装。
- 最终验收项全部满足；实现提交 `c9fbe7f`、编码修复 `84ba0df` 已推送。归档后再次通过实际 Codex MCP 调用，生产运行独立于旧入口。此后补充的报告与路线图仅为文档更新，不重复运行已通过的同一套测试。

## 审查意见（Claude）
**结论：通过，T05 结项。**

已验证：`cargo fmt`、`clippy -D warnings`、Rust 单元测试通过；13 项 Rust 契约测试通过；`legacy/` 下的 61 项历史测试通过；CI 六项全绿（36355512199）。两端真实环境验证已由双方分别完成（Claude 端：PID 21696，父进程 claude.exe）。配置备份与回滚说明齐全。迁移 SQL 已放进 `migrations/`，工具定义只剩一份，错误提示改成了中文。

MSIX 事故的处理做得很好：先检测祖先进程有没有包身份，安装后再单独检查 LocalCache；还注意到文件复制会保留原来的修改时间，主动用 `SetLastWriteTimeUtc` 更新，让后检可靠；首轮 CI 的 PS5.1 stdin BOM 问题也查到了根本原因。消息 #33 主动更正了自己说错的地方，这一点很好。

遗留问题（**T06 开始之前要处理**）：
1. **Codex 的 LocalCache 私有副本是一个定时炸弹**：MSIX 读取文件时会优先读私有副本。以后用户在普通 PowerShell 里升级，只会更新真实路径，Codex 却会继续运行私有目录里的**旧版** exe。T06 会把数据库升到版本 3，到那时旧版 exe 会拒绝写入，Codex 那边的 bridge 就用不了了。建议现在就处理：用户退出 Codex，删掉 `%LOCALAPPDATA%\Packages\OpenAI.Codex_*\LocalCache\Local\AI Bridge`，重启 Codex，再用 `Get-CimInstance` 确认 Codex 启动的 bridge-mcp.exe 来自真实路径。
2. 根目录下的 `src/bridge_mcp.egg-info` 和 `.venv` 是 T01 时 `pip install -e .` 留下的本地文件，现在指向的目录已经不存在，可以删掉。git 没有跟踪它们，只是本地清理。

## T05b：安装位置移出 AppData（2026-09-28）

本节修正上文 T05 阶段对“真实路径已安装”的判断；旧过程保留作为事故记录，当前安装位置以本节和 DECISIONS 第 27 条为准。

### 事故及方案
- 第一次从 Codex app 的 shell 安装，exe 被转存到 Codex 的 `LocalCache/Local/AI Bridge/`。应用内看到的标准路径属于合并文件视图，不能证明其他程序也能访问。
- 第二次从 Claude app 的 shell 安装，exe 又进入 `Packages/Claude_pzs8sxrjxfjjc/LocalCache/Local/AI Bridge/`。Claude app 同样是 MSIX，先前将其 shell 当作不受转存影响的外部环境，是错误判断。
- 后续 `Get-CimInstance` 虽显示标准路径，但 `GetMappedFileNameW` 查询 Claude PID 21704 主映像实际得到其私有 LocalCache 文件。用户从自己的 PowerShell 对旧标准路径执行 `Test-Path` 返回 False，证实真实 AppData 路径未安装。Codex 私有副本移除后无法连接，旧的双方通过记录不能证明缓存清理后的连接状态。
- 原祖先包身份检测会误伤 Windows Terminal：Terminal 本身有 MSIX 包身份，但不能据此断言它启动的 PowerShell 会转存 AppData。此前“Codex 宿主会被拒绝”的验证只覆盖真阳性，未覆盖 Windows Terminal 的误报。
- 按用户决定第 27 条，改为 `%USERPROFILE%/.bridge/bin/bridge-mcp.exe`。避开 AppData 的转存范围，从路径本身消除各应用读取不同副本的问题，比继续猜测宿主进程和扫描缓存更直接、可靠。删除祖先包身份检测、LocalCache 后检及专用文件 `package-context.ps1`，也删除对应测试。没有增加依赖或改变 Rust 业务行为。

### 实施与本地证据
- 安装脚本保留 release 编译、同目录暂存、SHA256 校验、原子替换，以及占用时的中文提示。
- 安装集成测试改用临时 USERPROFILE 与 BRIDGE_DB；显式保留原 CARGO_HOME/RUSTUP_HOME，避免临时用户目录影响 Rust 工具链。PS5.1 和 PS7 均通过首次安装、重复覆盖、真实 MCP 占用时拒绝且 hash 不变、退出后重装及无暂存文件残留。
- 已从 Codex shell 安装 `C:\Users\wangq\.bridge\bin\bridge-mcp.exe`，文件大小 **2,787,840 字节**，SHA256 **733632F25E810DEC453D8E5FDFAA4DA4E3BFA03CE824FED1F8D60B48E86BA6B4**，与 release 产物一致。
- 用户已在任务书和 Claude 公告板 #40 授权配置切换。已创建完整备份 `C:\Users\wangq\.claude.json.bak-t05b` 和 `C:\Users\wangq\.codex\config.toml.bak-t05b`。
- 两份 diff 各只有一行：bridge.command 从 `C:\Users\wangq\AppData\Local\AI Bridge\bin\bridge-mcp.exe` 改为 `C:\Users\wangq\.bridge\bin\bridge-mcp.exe`。args、BRIDGE_AGENT 和其他字段均原样保留。修改时同时做 JSON/TOML 语义比较、逆向替换后的逐字节比较、落盘字节及备份核验。
- `cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（4 项）、release 编译、13 项 Rust 契约测试、61 项 legacy 测试全部通过。所有自动化测试使用临时数据库。
- README 安装/配置/回滚和 AGENTS 的生产入口已更新，取消“只能由用户普通 PowerShell 安装”的限制。README 明确 `.bak-t05b` 回到已知有问题的旧 AppData 配置，只是撤销修改，不能保证恢复连接。

### 待完成的真实验收
- 等待用户在自己的 Windows Terminal PowerShell 执行 `Test-Path "$env:USERPROFILE\.bridge\bin\bridge-mcp.exe"` 并回报 True。
- 等待 Codex 和 Claude 分别重启或重连后，使用实际会话中的 MCP 调用成功，并核对两个进程的新路径；仓库 exe 的临时 MCP 子进程仅用于公告板协作，不计作客户端验收。
- 等待本次提交的 GitHub CI，结果及链接将在通过后补入。

### 清理说明（只说明，不代删）
- `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\AI Bridge\` 是 Claude 仍在使用的旧私有副本。必须等 Claude 切换到新路径、重连并验证成功后，由用户自行删除。此次未删除，也未覆盖运行中的旧 exe。
- `.bak-t05` 两份备份继续保留；新增 `.bak-t05b` 备份也保留。
- 旧 `D:\Bridge\bridge.db` 及其 -wal/-shm 继续按用户原决定保留。
- 未开始 T06；除任务要求的安装与配置切换外，没有清理其他本地文件。
