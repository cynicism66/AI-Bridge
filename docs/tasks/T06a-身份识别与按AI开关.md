# T06a：身份与项目识别、按 AI 开关（数据库版本 3）

## 背景
T06 的内容很多，拆成两步：**T06a 做底层的身份识别和按 AI 开关**，T06b 再做职务、权限和协作初始化。
依据：DECISIONS 第 11、12、15、16 条，以及 `docs/design/协作初始化与交接.md` 第三节。
**从本任务起，新功能只在 Rust 里实现**，`legacy/` 不改。

## 一、按 git 仓库识别项目（决策 15）
AI 传进来的 `project`，以及命令行里的项目参数，都要先换算成**项目键**：
1. 从给定路径往上逐级查找 `.git`：
   - `.git` 是**目录**：它就是仓库的公共目录；
   - `.git` 是**文件**（worktree 或子模块）：读出其中的 `gitdir: <路径>`（相对路径按 `.git` 文件所在目录解析）。如果那个目录里有 `commondir` 文件，按它找到公共目录；没有的话，`gitdir` 指向的目录本身就是公共目录。
2. 项目键 = 公共目录的**上一级目录**（公共目录名为 `.git` 的常见情况）；如果公共目录不叫 `.git`（比如 bare 仓库），项目键就是公共目录本身。然后用现有的路径规范化规则处理。
3. 找不到 `.git`：和现在一样，按传入的路径识别。
4. **不调用 git 命令**，只读文件。`.git` 文件格式不对、或者指向的目录不存在时，退回按路径识别，**不能报错**，并在 stderr 写一行诊断信息。
5. 同时要读出**当前分支**：`HEAD` 文件内容是 `ref: refs/heads/<名字>` 时，分支就是这个名字；否则视为游离状态，显示 HEAD 的前 7 位。还要记下**当前工作目录**，也就是 worktree 的根目录。

效果：Claude 在 worktree `D:\Bridge-wt1` 里、Codex 在 `D:\Bridge` 里，两边看到的是同一块公告板。

## 二、窗口会话（决策 16）
- **一个 MCP 服务器进程 = 一个会话**：Claude 和 Codex 每开一个会话，就会各自启动一个 bridge 进程。
- 新表 `sessions`：会话 id（进程启动时随机生成）、agent、项目键、编号、所在 worktree、分支、进程 id、开始时间、最后活动时间。每次工具调用都更新最后活动时间。
- **编号**：会话在某个项目里第一次调用工具时分配。取同一个项目、同一个 agent 下**活跃会话**没占用的最小正整数，活跃指最后活动时间在 2 小时以内。显示成 `codex #1`、`codex #2`。
- **状态和认领按会话记录**：
  - `status` 的主键从 `(project, agent)` 改为 `(project, agent, session_no)`；
  - 认领记录多存一列会话编号。**同一个 agent 的不同会话认领同一个文件时，也算冲突**；释放时只能释放自己这个会话的认领（不传 files 时也一样）。
- **消息仍然按 agent 投递**：发给 `codex` 的消息，codex 的任何一个会话都能读到，已读状态按 agent 记录。这样新开的会话不会把历史消息全部重新当成未读。在 README 里说明这条规则。
- 公告板显示：
  ```
  【codex #1】分支 main · 文件夹 d:/bridge · 更新于 …
  【codex #2】分支 feature-x · 文件夹 d:/bridge-wt1 · 更新于 …
  ```
  `你的身份` 那一行也要带上编号、分支和文件夹。超过 24 小时没有活动的会话，状态合并成一行"较早的会话：codex #3（3 天前）…"。

## 三、按 AI 开关（决策 11、12）
- 三级开关：**全局 → 项目 → 这个项目里的这个 AI**。AI 这一级默认开启，只有用户明确关掉才关。
- 所有开关的判断**集中在一个函数里**，T06b 以后还会往里加。
- AI 这一级关闭时，这个 AI 调用工具不读写业务数据，只返回：
  `Bridge 在此项目中未对你（codex）开启（由用户控制）。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。`
  和现在一样，不设置 `isError`。
- 命令行：
  - `bridge-mcp agent [项目] <agent> on|off`
  - `status` 和 `show` 要显示每个项目里各个 AI 的开关状态。已知的 AI 取自：该项目里出现过的会话和状态记录，加上已经有开关设置的 AI。
- 开关变化记入历史：新增事件类型 `agent_switch`，由触发器记录；`history` 命令的输出格式是 `[时间] human 对 codex 关闭 Bridge` 或 `…开启 Bridge`。

## 四、数据库迁移（版本 3）
- 新文件 `migrations/003.sql`：新建 `sessions` 表；`status` 表按新主键重建（新建表、复制数据、删掉旧表、改名），**原来依附在 status 上的触发器要重新创建**；`claims` 加上会话编号列；`settings` 支持 agent 一级的开关（表结构由你设计，要能写成触发器）；新增 `agent_switch` 触发器。
- 旧数据迁移：现有的状态和认领，会话编号一律设为 1。
- 已经按 worktree 路径写入的历史数据**不做合并**（目前实际上没有），在报告里说明。
- `legacy/` 的 Python 版遇到版本 3 会拒绝写入，这是预期内的，不需要处理。

## 五、测试
- **Rust 单元测试**：
  - `.git` 解析：普通仓库、worktree（相对和绝对 gitdir 都要有）、子模块、bare 仓库、格式坏掉的 `.git` 文件、`commondir` 缺失、从子目录往上找；
  - 编号分配：空缺复用、2 小时过期；
  - 三级开关的组合。
- **契约测试**（在临时目录里构造真实的 `.git` 结构，不调用 git 命令）：
  - 主目录和 worktree 看到同一块公告板，分支和文件夹显示正确；
  - 两个 codex 进程分别显示为 `#1`、`#2`，各自的状态互不覆盖；
  - 同一个 agent 的不同会话认领同一个文件时冲突；
  - AI 一级开关：关闭时返回规定文字、不写入数据，打开后马上恢复；
  - `agent` 命令、`status`、`show` 的输出；`agent_switch` 事件和 `history` 输出；
  - 从版本 2 升级到版本 3：数据完整，触发器仍然正常工作。
- 用**真实数据库的副本**演练一次从版本 2 升级到版本 3，把升级前后的数据条数写进报告，**不能碰原件**。

## 六、部署（本任务的最后一步）
升级真实数据库之后，旧版 exe 会拒绝写入。所以：
1. 所有测试和 CI 都通过以后，在公告板上请用户**退出 Claude 和 Codex**；
2. 运行 `scripts/install-local.ps1`，新路径不受 MSIX 影响，你可以自己运行；
3. 请用户**重新打开两个 app**。改的是 exe 文件，不是配置，所以按理只要重新启动 bridge 进程就行；为了稳妥，统一完全重启；
4. 双方各自用真实的 MCP 调用确认：显示编号、分支和文件夹，并用 `GetMappedFileNameW` 确认实际加载的文件在 `.bridge\bin` 下。

## 不要做
- 不做职务、权限、章程、协作初始化（T06b）。
- AI 工具的名称和参数不变。返回文字只允许按本任务书的要求改动（身份行、状态显示、AI 一级关闭时的提示）。
- 不修改全局配置，不改 `legacy/`。

## 验收标准
- [x] 主目录和 worktree 共用一块公告板，契约测试覆盖
- [x] 同一个 AI 的多个会话按 `#编号` 分开显示，状态不互相覆盖；同一个 agent 的不同会话认领同一个文件会冲突
- [x] 三级开关正确；AI 一级关闭时返回规定文字、不写入数据；改动马上生效
- [x] 数据库升到版本 3，旧数据完整，触发器正常；用真实库的副本演练过
- [x] `.git` 格式异常时退回按路径识别，不报错
- [x] `cargo fmt`、`clippy`、`test`、契约测试全部通过；Windows CI 全绿（按决定 28 精简，legacy 仅手动运行）
- [x] 部署完成，双方用真实 MCP 调用验证过
- [x] README 更新：项目识别规则、会话编号、消息按 agent 投递、`agent` 命令
- [x] Rust 单个文件不超过 300 行

## 完成报告
### 2026-09-28：实现、CI 与双方真实部署验收全部完成

- 先完成 T05b 收尾提交 `2662cdf`：补齐用户 Test-Path=True、双方实际调用、GetMappedFileNameW 与完整重启的证据，更新 README/PLAN，并收录 Claude 交接的 T06a 任务和设计修订。
- 新增 `repository.rs`，只读 `.git`/gitdir/commondir/HEAD，解析普通仓库、相对/绝对 gitdir、worktree、子模块、bare 仓库及子目录查找；异常写一行中文 stderr 并退回原路径。不调用 git，也没有增加依赖。
- 一个 MCP 进程生成一个随机 ID（使用现有 SQLite 的 randomblob，在内存数据库生成，不触碰业务库）；`sessions` 按 ID+项目存储，编号在立即事务中取当前活跃会话未使用的最小正整数，避免并发争用。每次有效项目工具调用刷新时间和分支；无 project 参数的 list_projects 刷新本进程已登记且仍启用的项目。
- 状态按 `(project,agent,session_no)` 保存，认领附带编号；额外保存内部 session_id 校验所有权，防止两小时过期后复用编号的新进程误释放旧认领。超 24 小时的状态合并显示；消息与已读仍按 agent 共享。绝对认领路径相对于当前 worktree 根目录归一化。
- 三级开关判断集中到 `access_state`，按全局/项目/AI 顺序拒绝。AI 关闭时只读开关，不更新业务表或会话；无项目参数的 list_projects 不显示对当前 AI 关闭的项目。新增人用 `agent [项目] <agent> on|off`，status/show 枚举会话、状态、开关里的已知 AI；agent_switch 由触发器记录，重复设置同值不产生伪变更事件。
- 迁移 `003.sql` 重建 status 并恢复触发器，保留原列值和已有 events；claims 增加编号与内部 ID；新增 sessions/agent_settings。旧状态和认领均为 #1，建立 PID=0 的迁移来源会话，避免新进程冒领旧认领；后续按两小时活跃期限复用编号。旧 worktree 路径的数据未合并。冻结 Python 不修改，版本 3 后其拒绝写入属于预期行为。
- README 已说明项目识别、窗口编号、共享已读、agent 命令和升级顺序；工具名称/参数保持不变。Rust 最长文件 174 行，所有文件均小于 300 行。

### 本地测试

`cargo fmt --check`、`cargo clippy --workspace --all-targets -- -D warnings`、`cargo test --workspace`（9 项）、release 编译、23 项 Rust 契约测试、61 项 legacy 测试全部通过。契约覆盖四进程并发编号、同 AI 多会话状态/认领隔离、编号复用、消息共享已读、worktree/分支、坏 Git 数据回退、三级开关与关闭时全表快照不变，以及版本 2 数据和触发器迁移。首次新增测试暴露 Python sqlite3 上下文不会关闭连接，已改为 contextlib.closing，严格模式和 Windows 清理均通过。

### 真实数据库副本演练（原库只读）

用 SQLite 只读连接和 backup API 将 `C:\Users\wangq\.bridge\bridge.db` 备份到临时目录；对副本设置 BRIDGE_DB，再运行新版 status 触发迁移。所有原始列逐行比对一致，integrity_check=ok。演练前后原库 user_version 都是 2，副本变为 3；临时副本验证完毕后清理，未上传真实协作内容。

| 表 | 升级前 | 升级后 |
|---|---:|---:|
| status | 2 | 2 |
| messages | 49 | 49 |
| reads | 46 | 46 |
| claims | 9 | 9 |
| settings | 1 | 1 |
| events | 138 | 138 |
| sessions（新增） | — | 2 |
| agent_settings（新增） | — | 0 |

### 偏离与剩余事项

内部 session_id 和 PID=0 的迁移来源会话是为编号复用时的归属安全补充的存储细节，未改变工具参数或消息投递规则。未实施 T06b，未修改全局配置或 legacy。实现及副本演练阶段未升级真实库；用户完成独立终端安装并重启后，真实库已升级至版本 3，见下方部署记录。

短路径修复提交 `aedd14f` 的 CI [36367255616](https://github.com/cynicism66/AI-Bridge/actions/runs/36367255616) 已全部通过。按用户决定 28，工作流精简为 Windows Rust 与两种 PowerShell 安装测试；移除 Ubuntu 和冻结 legacy 的 CI，AGENTS/README 同步说明，精简后的提交 `d639e05` 已通过 [Windows CI 36367508583](https://github.com/cynicism66/AI-Bridge/actions/runs/36367508583)，fmt、clippy、test、release 编译、契约测试、两种 PowerShell 安装测试全部成功。全部 CI 通过后才按任务书请求用户退出两端、安装新版、完整重启，再由双方实际 MCP 调用和 GetMappedFileNameW 验证；双方真实验证现已通过，部署验收已勾选，详见下方最终记录。

首次 CI 的 Windows Rust 测试发现临时目录采用 RUNNER~1 短路径，Git 公共目录解析成 runneradmin 长路径；已统一真实 Git/worktree 目录的规范路径，测试夹具也使用规范目录，并新增 Windows 8.3 长短路径同一身份的契约。本地修复后全部检查通过；修复提交的 Windows CI 已通过。


### CI 范围调整（决定 28）

按用户指示和 Claude #53，`.github/workflows/test.yml` 只保留 windows-latest 的 fmt、clippy -D warnings、test、release 编译、契约测试及 Windows PowerShell 5.1/PowerShell 7 安装测试。移除 Ubuntu 和 legacy Python 作业；冻结代码未改，本轮手动运行的 61 项历史测试仍通过。此决定覆盖原任务验收里的 legacy CI 要求。

### 部署交接点

首次部署尝试前，代码、短路径修复、Windows CI 精简均已提交推送并验证，当时正式安装及双方真实 MCP 验收仍待完成。已在忽略目录 `target/t06a-deploy.ps1` 准备临时后台安装助手：等待 Claude/Codex/Bridge 进程全部退出后调用现有 `scripts/install-local.ps1`，核对安装文件与 release 的 SHA256；等待上限 15 分钟，不主动结束应用，结果写入 `target/t06a-deploy.log`。完整重启后应先检查日志与实际映像，再由双方真实 MCP 调用确认编号、分支和文件夹，最后勾选部署验收。本助手不修改配置，也不主动打开真实数据库。

### 重启后的实际核验：安装未完成，改用独立终端

- 本轮用户重开客户端后，Codex 真实 `bridge_overview` 调用成功，但身份仍只有 `codex`，没有 T06a 的编号、分支和文件夹，因此部署验收不通过。
- `target/t06a-deploy.log` 只有 `2026-09-28 09:54:58 WAITING`，未出现 INSTALLING、SUCCESS 或 FAILED。助手 PID 33292 已不存在。退出原因没有日志证据，可能被宿主进程生命周期清理，不能视为已完成或正常超时。此前“等待约 30 秒”只是时间估计，不是安装成功判据，也未验证后台助手能跨 Codex 退出存活。
- `GetMappedFileNameW` 实测 Claude PID 33008、Codex PID 15292 均加载 `\Device\HarddiskVolume3\Users\wangq\.bridge\bin\bridge-mcp.exe`。位置正确，但内容仍为旧版：安装文件 SHA256 `733632F25E810DEC453D8E5FDFAA4DA4E3BFA03CE824FED1F8D60B48E86BA6B4`；目标 release SHA256 `84EB126E271DF184602746F5294C98B94EA7F9EB47C4A5985B7B5887F32C391E`，二者不同。
- 只读查询真实库 `PRAGMA user_version` 为 2，尚未升级。没有在真实库运行测试，没有修改全局配置。
- 已通过公告板 #57 告知 Claude。后续请用户在独立 Windows Terminal 中完全退出 Claude/Codex 后运行 `powershell -NoProfile -ExecutionPolicy Bypass -File "D:\Bridge\scripts\install-local.ps1"`，看到“已安装”再重开双方；以文件哈希、真实 MCP 返回和实际映像为验收证据，不再仅按等待时间判断成功。后台助手未重新启动。

### 独立终端安装后的真实验证

用户按提示完成安装并重启两端后，本轮核验：

- 安装文件 `C:\Users\wangq\.bridge\bin\bridge-mcp.exe` 与 `target/release/bridge-mcp.exe` 的 SHA256 均为 `84EB126E271DF184602746F5294C98B94EA7F9EB47C4A5985B7B5887F32C391E`，新版已实际安装。
- Codex 本端真实 `bridge_overview` 返回 `codex #2 · 分支 master · 文件夹 d:/bridge`。编号为 #2 是因为迁移来源的 #1 仍在两小时活跃期限内，属于预期分配。
- `GetMappedFileNameW` 核验 Claude PID 33652（父进程 claude.exe，PID 21744）和 Codex PID 24780（父进程 codex.exe，PID 32840），实际映像均为 `\Device\HarddiskVolume3\Users\wangq\.bridge\bin\bridge-mcp.exe`，对应真实安装目录。
- 只读核验真实数据库：`user_version=3`，`integrity_check=ok`；没有对真实库运行自动测试。
- Claude 在公告板 #59（2026-09-28 10:22:52）确认本端真实 `bridge_overview(project=D:/Bridge)` 成功，返回 `claude #2 · 分支 master · 文件夹 d:/bridge`；独立核对双方实际映像、安装哈希与数据库版本 3。其 #2 同样由仍在活跃期限内的迁移 #1 造成，符合编号规则。
- 至此任务书全部验收项满足，T06a 实现与部署完成；没有已知的未完成验收项。首次后台安装助手失败的记录保留，最终安装由用户在独立终端执行现有安装脚本完成。后续由 Claude 进行完整代码审查，T06b 尚未开始。
