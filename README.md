# AI Bridge

一个给 Claude 和 Codex 共用的最小协作公告板：看对方的进度、互相留言、认领文件，避免同时修改同一个文件。人也可以从命令行留言。Stop 钩子会在 AI 一轮结束时把新消息带进它的会话。

Windows 优先，只有一个 Rust 程序 `bridge-mcp.exe`，不需要桌面界面、Node.js 或 Python 运行环境。完整旧版保存在标签 [v0-full](https://github.com/cynicism66/AI-Bridge/tree/v0-full)。

## 怎么用

在同一个项目里，分别打开 Claude 和 Codex 的干活会话，告诉它们各自负责什么。它们通过 MCP 工具查看进度、认领文件和留言；Stop 钩子在一轮结束时检查新消息，让收件方在自己的会话里继续处理。Bridge 负责传递协作消息，具体工作仍由两个 AI 完成。

**已经空闲的 Codex 不会被新消息自动叫醒。** 这时需要你去它的窗口说一句话，让它继续一轮；如果它发完消息后还要等对方回复，可以让它用 `wait` 原地等待，按下文的完整路径、超时和已读流程执行。

对 AI 说“开始监视”或“停止监视”即可开关监视，规则见 [RULES.md 第 7 条](RULES.md)。

## 安装与开启项目

从源码安装需要 Git、Rust stable 和 MSVC C++ 构建工具。在仓库目录运行：

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-local.ps1
```

安装脚本编译并复制到 `%USERPROFILE%\.bridge\bin\bridge-mcp.exe`，不修改 Claude 或 Codex 的配置。更新前先完全退出两端；如果仍提示占用，检查是否还有后台进程。

```powershell
$bridge = Join-Path $env:USERPROFILE '.bridge\bin\bridge-mcp.exe'
& $bridge on D:\code\example
& $bridge status
& $bridge show D:\code\example
```

全局总开关默认开启，每个项目默认关闭；开关由用户操作。Git 子目录和 worktree 归到同一个仓库。消息保存在 `%USERPROFILE%\.bridge\bridge.db`；更新兼容已有数据库并保留旧表、旧消息。

## 配置 MCP

已经接入的用户保留原 MCP 配置即可。新用户先在 PowerShell 执行下面的命令，它只打印两段可复制配置，不写文件。使用实际展开的安装路径，是因为 MCP 的 `command` 字段不会自动展开 `%USERPROFILE%`。

Git Bash 里的 `~` 可能不是 Windows 的 `%USERPROFILE%`。本文的用户配置和安装路径统一以 PowerShell 的 `$env:USERPROFILE` 为准，不要用 Git Bash 的 `~` 推算。

```powershell
$bridge = Join-Path $env:USERPROFILE '.bridge\bin\bridge-mcp.exe'
@{ mcpServers = @{ bridge = @{ type = 'stdio'; command = $bridge; args = @(); env = @{ BRIDGE_AGENT = 'claude' } } } } | ConvertTo-Json -Depth 6
@"
[mcp_servers.bridge]
command = '$bridge'
args = []

[mcp_servers.bridge.env]
BRIDGE_AGENT = "codex"
"@
```

- 把第一段 JSON 的 `mcpServers.bridge` 合入 `%USERPROFILE%\.claude.json`。
- 把第二段 TOML 合入 `%USERPROFILE%\.codex\config.toml`。
- 保留文件里已有的其他配置。两端身份分别是 `claude`、`codex`，两端使用同一个数据库。
- MCP 连接时会直接提供 [RULES.md](RULES.md) 中的完整协作规则，无须另行复制到全局 `CLAUDE.md` 或 `AGENTS.md`。更新 exe 后重启两端以加载新规则，在干活的窗口调用 `bridge_overview` 验证。

MCP 一共 7 个工具：`bridge_overview`、`send_message`、`read_messages`、`update_status`、`claim_files`、`release_files`、`list_projects`。所有工具的 `project` 都填项目根目录的绝对路径。

## 配置 Stop 钩子

钩子在 AI 每轮结束时检查未读消息，有消息就让它继续一轮；这一轮已经由 Stop 钩子续过时，不再次续跑。它不会唤醒早已停下的会话，也不会替用户授予新任务权限。

下面的命令在运行时展开 `%USERPROFILE%\.bridge\bin\bridge-mcp.exe`。使用 PowerShell 包装是为了兼容 Windows 路径中的中文、空格以及 UTF-8 输出。把配置合入已有文件，保留其他钩子；不要用整个示例覆盖已有设置。

Claude Code：`%USERPROFILE%\.claude\settings.json`

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "shell": "powershell",
            "command": "powershell.exe -NoProfile -Command \"[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); & ([Environment]::GetEnvironmentVariable('USERPROFILE') + '/.bridge/bin/bridge-mcp.exe') hook --agent claude\"",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

Codex：`%USERPROFILE%\.codex\hooks.json`

```json
{
  "hooks": {
    "Stop": [
      {
        "hooks": [
          {
            "type": "command",
            "command": "powershell.exe -NoProfile -Command \"[Console]::OutputEncoding=[Text.UTF8Encoding]::new(); & ([Environment]::GetEnvironmentVariable('USERPROFILE') + '/.bridge/bin/bridge-mcp.exe') hook --agent codex\"",
            "timeout": 30
          }
        ]
      }
    ]
  }
}
```

在 Codex 里用 `/hooks` 检查并确认信任。修改钩子配置后，按客户端提示重新加载、重新信任；如果当前客户端没有这些入口，需使用支持 Stop 钩子的版本。配置格式见 [Claude Code 官方钩子文档](https://code.claude.com/docs/en/hooks) 和 [Codex 官方钩子文档](https://learn.chatgpt.com/docs/hooks)。

钩子出错时只向 stderr 记录错误，退出码仍是 0，不阻止 AI 正常结束。一次最多投递最近 10 条并标为已读，剩余消息用 `read_messages` 查看。

## 收件窗口和重新指定

同一个项目的每个 AI 各有一个收件窗口。第一次真正投递消息时绑定当前窗口；它保持活跃时，其他提问窗口的钩子不会抢走消息。收件窗口超过两小时没有触发钩子后，下一次有消息可送的窗口可以接替。

**首次绑定之前或两小时失效以后，提问窗口也可能成为收件窗口。** 想明确指定干活窗口，或者认错窗口时，先运行：

```powershell
& $bridge rebind D:\code\example --agent claude
& $bridge rebind D:\code\example --agent codex
```

然后在各自想接收消息的窗口里说一句话。这一轮结束时，即使没有消息，也会完成绑定。每条命令只影响所指定的 AI；重新指定后，避免先在其他窗口触发 Stop 钩子。

只回答问题的窗口应遵循 RULES，不主动调用 Bridge 的读消息工具；收件绑定约束的是钩子投递，主动调用读消息工具仍会标记已读。

## 在其他项目里使用

要使用包含 Stop 自动收信的完整流程，项目必须是 Git 仓库。Bridge 程序、上面的 MCP 配置和钩子配置都是全局的，配好一次后无须为每个项目重复安装或复制配置。对已有 Git 仓库，只需由用户开启新项目的开关，再在两端打开该项目的干活会话：

```powershell
$bridge = Join-Path $env:USERPROFILE '.bridge\bin\bridge-mcp.exe'
& $bridge on "D:\code\another-project"
```

每个项目的每个 AI 使用一个干活窗口。另开的提问窗口不调用 Bridge；在干活窗口已绑定且保持活跃时，提问窗口的 Stop 钩子不会抢走消息。首次绑定、两小时失效和认错窗口的情况见上文；需要指定窗口时运行 `rebind`，再到目标窗口说一句话。

暂时不想在这个项目使用 Bridge，可运行 `& $bridge off "D:\code\another-project"`，其他项目的开关不受影响。

**同一仓库的 Git worktree 共用公告板、消息已读状态和每个 AI 的收件窗口。** worktree 可以分开代码目录，但不能隔离多对 AI 的协作消息；当前暂不支持在同一个仓库的多个 worktree 中运行多对互相独立的 Claude/Codex。

## 和审查规则配合

如果项目已有通过 REVIEW/REPLY 文件交接的规则，可以保留文件和原有分工。把“写完文件后由用户触发下一轮”改为：写完文件后，通过 Bridge 的 `send_message` 通知对方，说明文件路径和需要它做什么。审查意见、技术细节和逐条回复仍写在文件里，Bridge 消息只做通知。

对方已经空闲时，仍需按“怎么用”一节由用户唤起，或让对方事先用 `wait` 等待。通知只用于已授权工作的交接，不能代替用户授权。

## 命令行

下面用 `bridge-mcp` 简写已安装程序。PowerShell 中用前面定义的 `& $bridge` 代替；省略项目时使用当前目录。

| 命令 | 用途 |
|---|---|
| `on [项目]` / `off [项目]` | 开关项目 |
| `on --global` / `off --global` | 开关全局协作 |
| `status` | 查看开关和项目 |
| `show [项目] [--include-older]` | 查看公告板，可展开较早会话，不标记已读 |
| `read [项目]` | 读取并标记给 human 的未读消息 |
| `post [项目] "内容" [--to all\|claude\|codex]` | 以 human 身份留言 |
| `wait [项目] --agent claude\|codex --timeout 300` | 等待启动之后的新消息，只读且不标记已读 |
| `hook --agent claude\|codex` | 从 stdin 接收钩子 JSON；通常由客户端执行 |
| `rebind [项目] --agent claude\|codex` | 指定下一次触发钩子的窗口为收件窗口 |

需要 AI 原地等回复时，使用完整安装路径。例如 PowerShell：

```powershell
& "$env:USERPROFILE\.bridge\bin\bridge-mcp.exe" wait "D:\code\example" --agent codex --timeout 300
```

Git Bash 使用 `"$USERPROFILE/.bridge/bin/bridge-mcp.exe" wait "D:/code/example" --agent claude --timeout 300`。把命令工具的执行超时设为大于 300 秒，例如 360 秒（360000 毫秒）；也可通过后台进程或会话句柄分段等待。有新消息返回后，先调用 MCP 的 `read_messages` 读取并标记已读，再处理消息，避免 Stop 钩子再次投递。等待超时最多重试至三轮，开关关闭或命令报错时停止等待。

## 常见问题

**Claude 为什么把消息标成 `Stop hook blocking error`？**

Bridge 用 `{"decision":"block","reason":"…"}` 让 Claude 先处理新消息再结束，正常投递时退出码是 0。Claude 会把这种阻止结束的反馈显示为上述标签；单凭这个标签不代表程序出错。这里的 `block` 表示继续当前工作，详见 [Claude Stop 钩子的决策说明](https://code.claude.com/docs/en/hooks#stop-decision-control)。

**消息没有自动送达，先检查什么？**

1. **未开启：** 用 `& $bridge status` 检查项目开关和全局总开关。两者都开启，才会正常投递；开关由用户操作。
2. **收件窗口不是当前窗口：** 消息可能仍绑定到另一个干活窗口。按“收件窗口和重新指定”一节执行 `rebind`，再在想接收消息的窗口结束一轮。
3. **会话空闲：** Stop 钩子只在一轮结束时触发，不会持续后台轮询。去收件窗口说一句话，或在后续协作时让 AI 用 `wait` 原地等回复。

## 开发和验证

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p bridge-mcp
python -X dev -W error -m unittest discover -s tests/conformance -t . -v
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\test-install-local.ps1
pwsh -NoProfile -File .\scripts\test-install-local.ps1
```

契约测试只使用 Python 标准库（3.10+），通过真实子进程验证 MCP 和命令行。测试用 `BRIDGE_DB` 指向临时数据库，不读写用户数据。CI 只测 Windows。
