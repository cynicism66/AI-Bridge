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

## 不要做
- 不做 T06 及以后的功能（按 AI 开关、职务、初始化等）。
- 不改工具名称、参数和成功时的返回文字；唯一的例外是第二部分第 3 条的错误提示。
- **没有用户明确同意，不许改全局配置，也不许删除旧数据库。**

## 验收标准
- [ ] `scripts/install-local.ps1` 能正常安装；exe 被占用时给出中文提示并安全退出（完成报告里写明你是怎么测试的）
- [ ] 迁移 SQL 在 `migrations/` 下，工具定义只剩一份，错误提示改成了中文，一致性测试已同步更新
- [ ] 两个配置文件和备份的 diff 只有 `bridge` 那几行
- [ ] 真实环境验证：Codex 和 Claude 都通过 Rust 版调用成功，实际运行的进程是 `bridge-mcp.exe`
- [ ] Python 版已移到 `legacy/`，`legacy/` 下的单元测试在 CI 里通过
- [ ] 一致性测试（目标为 Rust）、`cargo fmt`、`clippy`、`test` 全部通过；CI 全绿，完成报告附上链接
- [ ] README 写了安装、回滚、从源码编译的方法
- [ ] 旧数据库：已征得用户同意并删除，或者用户选择保留，两种情况都要在报告里写明

## 完成报告
（Codex 完成后填写：做了什么、偏离任务书的地方及原因、遗留问题、CI 运行链接、真实环境验证结果）
