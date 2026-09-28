# AI Bridge

[![测试](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml/badge.svg)](https://github.com/cynicism66/AI-Bridge/actions/workflows/test.yml)

**让 Claude 和 Codex 在同一个项目里知道彼此在做什么，让人也能参与协作。**

AI Bridge 是一个本地协作公告板，提供任务状态、定向／广播留言、文件认领和交互历史。
Rust 核心通过 stdio MCP 供 AI 调用；用户可以通过 Windows 桌面窗口、托盘和命令行查看状态、留言及控制开关。数据保存在本机 SQLite 中。
采用 MIT 许可证。桌面端已提供公告板、聊天、初始化向导、职务与权限编辑、历史筛选、导出和交接；安装包将在 T10 提供。进度见 [路线图](docs/PLAN.md) 和 [架构](docs/ARCHITECTURE.md)。

## 桌面窗口与托盘

从源码运行需要 Windows 10/11、WebView2 Runtime、Node.js 24、stable Rust 的 MSVC 工具链与 Visual Studio C++ 编译工具。在项目根目录执行：

```powershell
cd app
npm ci
npm run tauri build -- --no-bundle
cd ..
```

生成的程序为 `target/release/ai-bridge.exe`，本阶段没有安装包。升级前请退出 Claude、Codex 和旧的 AI Bridge，在独立终端执行 MCP 安装脚本并编译桌面程序，再启动桌面程序，最后重开两端。数据库会在新版首次访问时自动升到版本 7，保留消息、已读、认领和交互历史，并为新的已读记录保存读取时间；旧版 MCP 不支持版本 7，因此必须先更新 MCP 再开启桌面窗口。

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-local.ps1
if ($LASTEXITCODE -ne 0) { throw "安装失败" }
Start-Process .\target\release\ai-bridge.exe
```

**桌面程序必须由用户从独立的 Windows Terminal / PowerShell 或资源管理器启动，不要从 Claude/Codex 的 shell 启动。** MSIX 的注册表写入虚拟化可能使通知注册仅对宿主包可见，影响 Windows 通知点击激活；详见 [微软 MSIX 虚拟化说明](https://learn.microsoft.com/en-us/windows/msix/desktop/flexible-virtualization)。此前关于安装脚本可在 AI shell 中执行的说明，只涉及复制 MCP 文件，不代表从该 shell 启动桌面程序也能完成系统通知注册。

从 T08a 升级到 T08b **不需要重新初始化**，数据库迁移不改写已保存的章程。从更早版本首次升级到桌面版时，用户才需要重新执行原来的 `init` 命令，保留模板、职务、目标和规则文件输出选项，并加 `--no-kickoff`，将“来自软件界面的 human 消息才是用户本人确认过的指令”纳入章程且不重复派发起步任务。

- 左侧显示 Bridge 项目、未读数和从 Claude/Codex 配置中发现的项目；发现的项目默认关闭，可以开启或隐藏，也可以选择文件夹添加。
- 公告板显示章程、职务、AI 开关、会话和认领；开启未初始化的项目会自动弹出六步向导，也可点击“开始初始化”。向导预览完整章程后再确认，默认写入规则文件并派发起步任务。
- 职务下拉框支持确认后交换双方；每个位置可以编辑权限或恢复模板默认值。设置页提供重新初始化和当前章程全文。
- 历史按时间倒序显示，可同时筛选多个 AI、事件类型、日期和描述关键词；每次加载 100 条，未加载全部时明确提示。
- 设置页可导出协作记录或交接给一个 AI：先看打码后的完整预览、统计和远程仓库警告，再确认写入。交接预览还列出关闭的 AI、释放的认领和转发消息数。
- 聊天显示所有参与者的消息，可发给所有人、Claude 或 Codex。进入聊天页会标记用户的消息为已读，不影响 AI 的已读状态。GUI 发送的消息标注“软件界面”，CLI 发送的标注“命令行”。
- 左键点击托盘图标切换窗口显隐，右键菜单只列出已开启项目，可切换项目或总开关。项目显示文件夹名，同名时加上父目录；自动发现的项目留在主窗口。关闭窗口会隐藏到托盘；彻底退出用托盘菜单的“退出”。
- 新的 AI/Bridge 消息发给用户或所有人时才通知，点击后打开对应项目聊天；首次启动不补弹历史消息，重启不重复提示已处理消息。软件设置可以关闭通知，重新开启时也不补弹；系统通知设置或勿扰模式可能抑制提示，未读角标仍保留。
- 界面跟随系统深浅色。Windows 11 尝试使用 Mica，不支持时回退纯色。

右键“我的项目”可开启／关闭协作、在资源管理器中打开文件夹、从列表移除或彻底删除；“发现的项目”提供开启、打开文件夹、隐藏。默认浏览器右键菜单已关闭，正式版还限制刷新、打印、后退和开发者工具快捷键；输入框保留系统的复制、剪切、粘贴、全选快捷键。显示给人的 Windows 路径使用大写盘符和反斜杠，内部项目标识保持原样。

`bridge-mcp forget [项目]` 关闭并隐藏项目，保留全部协作数据和项目文件；省略项目时使用当前目录。以后在窗口中“+ 添加项目”选择原目录，会取消隐藏并恢复已有协作信息，项目开关仍然关闭，需由用户自行开启。命令行与窗口共用核心逻辑，窗口也会检测命令行造成的隐藏设置变化。

`bridge-mcp purge [项目] --yes` 彻底删除该项目的 Bridge 状态、会话、消息、已读、认领、开关、初始化、职务、权限和历史；操作不可撤销，也不留下删除历史，请先用 `export` 导出。若有两小时内活跃的 Bridge 会话，还需显式传 `--force`。界面采用两步确认，有活跃会话时还需额外勾选确认。

两种移除操作都只影响 AI Bridge，**不修改 Claude/Codex 的配置或项目文件**。Bridge 自己保留一个隐藏标记，避免刚删除的项目又被自动发现；“+ 添加项目”可解除隐藏，彻底删除后的项目需要重新初始化协作。原有 AGENTS.md / CLAUDE.md 章程区块保留，如不需要可手动删除。从 T08b 升级到 T08c 没有数据库迁移，也无需重新初始化现有项目。

软件设置存放在 `~/.bridge/app.json`，包括隐藏的项目、通知开关、首次关闭提示和通知高水位。设置页可以取消隐藏、重置关闭提示，并查看数据库路径和软件版本。自动发现只取 `~/.claude.json` 的 `projects` 键和 `~/.codex/config.toml` 的项目路径，不保存账号信息，不读取凭证或登录文件，也不修改两端配置。Windows 通知会注册 AI Bridge 自己的用户级通知身份和点击处理器，不需要管理员权限。

开发时在 `app/` 运行 `npm run tauri dev`。隔离手动测试应同时设置 `BRIDGE_DB` 为临时数据库、`BRIDGE_APP_HOME` 为临时用户配置目录。自动测试必须使用临时库，不能以真实数据库作为测试输入。

## 安装与从源码编译

Windows 需要 Git、stable Rust 的 MSVC 工具链及 Visual Studio C++ 编译工具。
安装位置为用户主目录下的 `.bridge/bin/`，和数据库共用 `.bridge/` 父目录。
MSIX 对 AppData 的私有目录转存不会影响这个位置，因此可以在 Windows Terminal、Claude 或 Codex 的 shell 中运行安装脚本，无需专门检查宿主包身份。

```powershell
git clone https://github.com/cynicism66/AI-Bridge.git D:\Bridge
cd D:\Bridge
cargo --version
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\install-local.ps1
```

安装脚本先执行 `cargo build --release -p bridge-mcp`，然后安装到固定位置：

```text
%USERPROFILE%\.bridge\bin\bridge-mcp.exe
```

脚本支持 Windows PowerShell 5.1 和 PowerShell 7，可重复执行来更新。
复制先写入同目录临时文件、校验 SHA256，再原子替换旧文件；复制或替换失败时保留原 exe。
运行中的 exe 被占用时会提示“请先退出 Claude 和 Codex 再安装”并退出；退出两端后重新执行即可。
MCP 配置应引用安装路径，编译目录 `target/release/` 会被 `cargo clean` 清理。

只编译、不安装时执行 `cargo build --release -p bridge-mcp`。
Windows 产物为 `target/release/bridge-mcp.exe`，Linux 为 `target/release/bridge-mcp`。
无参数启动时等待 MCP 输入，每行一条 UTF-8 JSON；stdout 只输出协议内容。

## 接入 Claude Code 与 Codex

示例使用用户名 `your-name`，请替换为你自己的完整安装路径。
只调整现有 `bridge` 条目，不要覆盖配置文件中的其他内容。

Claude Code 的 `~/.claude.json` 中：

```json
{
  "mcpServers": {
    "bridge": {
      "type": "stdio",
      "command": "C:\\Users\\your-name\\.bridge\\bin\\bridge-mcp.exe",
      "args": [],
      "env": { "BRIDGE_AGENT": "claude" }
    }
  }
}
```

Codex 的 `~/.codex/config.toml` 中：

```toml
[mcp_servers.bridge]
command = 'C:\Users\your-name\.bridge\bin\bridge-mcp.exe'
args = []

[mcp_servers.bridge.env]
BRIDGE_AGENT = "codex"
```

Codex 的字段说明见 [官方 MCP 文档](https://learn.chatgpt.com/docs/extend/mcp?surface=cli)。
修改前备份配置，修改后必须完全退出并重新启动 Claude app 和 Codex app，再分别调用 `bridge_overview` 验证。Claude 会缓存启动时的 MCP 配置，仅在 `/mcp` 中重连或重启服务器进程不会加载新的 command。验证安装来源使用 `GetMappedFileNameW` 查询实际映像文件，不能仅依赖可能显示虚拟路径的 `Get-CimInstance`。
两端应访问同一数据库，并把 [RULES.md](RULES.md) 加入各自的协作说明。

## 开关与人用命令

全局总开关默认开启，**每个项目默认关闭**，项目里的 AI 开关默认开启。三级开关按“全局 → 项目 → AI”判断，全部开启后，还必须完成项目协作初始化才允许该 AI 使用；开关由用户控制，AI 工具不能改变开关。

```powershell
$bridge = Join-Path $env:USERPROFILE '.bridge\bin\bridge-mcp.exe'
& $bridge on D:\code\example
& $bridge status
& $bridge show D:\code\example
```

下面用 `bridge-mcp` 简写安装好的程序；PowerShell 中可用上面的 `& $bridge` 代替。

| 命令 | 用途 |
|---|---|
| `bridge-mcp on [项目]` / `off [项目]` | 开启／关闭项目 |
| `bridge-mcp on --global` / `off --global` | 开启／关闭全局总开关 |
| `bridge-mcp agent [项目] <agent> on\|off` | 设置项目内某个 AI 的开关；省略项目时使用当前目录 |
| `bridge-mcp status` | 全局和各项目开关、有效状态、初始化状态、模板、目标、职务及最近活动 |
| `bridge-mcp show [项目] [--include-older]` | 章程、职务、权限、状态、认领及消息；不标记已读，可展开较早会话 |
| `bridge-mcp post [项目] "内容" [--to all\|claude\|codex]` | 以 human 身份发消息 |
| `bridge-mcp read [项目]` | 读取并标记给 human 的未读消息 |
| `bridge-mcp history [项目] [--limit N] [--agent 身份] [--kind 类型]` | 按时间正序查看最近 50 条交互历史 |
| `bridge-mcp permission [项目] <职务位置> [--allow 内容] [--deny 内容] [--write 规则] [--reset]` | 编辑项目权限覆盖；列表选项可以重复，`--reset` 恢复模板默认值 |

省略项目参数时使用当前目录，命令行接受相对路径；AI 工具必须传项目根目录的绝对路径。
Windows 接受 `D:\code\example`、`D:/code/example`、UNC 路径，以及转换为 `d:/code/example` 的 `/d/code/example`。
`\example`、`/example`、`/abc/example` 等缺少盘符的路径会报错。

三级开关和第四级初始化状态的判断集中在核心的 `access_state`；修改开关后下一次调用立即生效。AI 关闭时工具只读取开关，不创建或更新会话、状态、消息、认领或已读记录。`list_projects` 不展示对当前 AI 关闭的项目；人用 `status`、`show` 仍展示全部项目及已知 AI 的开关和有效状态。
关闭时人用命令仍能操作；AI 收到“未开启／已关闭”提示后，应停止在本次会话调用 Bridge。
Rust 不提供 `watch`，后续由桌面软件显示持续变化。

`history` 的类型包括 `status`、`message`、`claim`、`release`、`expire`、`switch`、`agent_switch`、`init`、`role`、`handover`、`permission`。
AI 开关事件显示为 `[时间] human 对 codex 关闭 Bridge` 或 `开启 Bridge`，由数据库触发器写入。
项目历史也包含全局开关事件；支持身份与类型组合筛选，目前不自动清理历史。

## 协作初始化、职务和权限

项目状态为：关闭 → 用户 `on` → 待初始化 → 用户 `init` → 协作中。重新开关已经初始化的项目会保留章程；重新执行 `init` 则递增章程版本。尚未初始化时，除 `list_projects` 外，AI 工具只返回提醒，不写会话、状态、消息或认领。

管理命令 `on`、`off`、`agent`、`init`、`role`、`permission`、`handover` 及导出命令 `export` 必须由用户执行。下面示例中的 `$bridge` 指向已安装的程序，项目应先开启：

```powershell
$bridge = Join-Path $env:USERPROFILE '.bridge\bin\bridge-mcp.exe'
& $bridge on D:\code\example
& $bridge init D:\code\example --template 任务书流程 --role 规划审查=claude --role 执行=codex --goal "实现项目目标"
& $bridge show D:\code\example
# 交换 codex 与当前规划审查的职务
& $bridge role D:\code\example codex 规划审查
```

- **任务书流程**：规划审查可写 `docs/**` 和根目录 `AGENTS.md`，执行方负责实现、测试、完成报告及统一提交；执行方的路径规则为空，表示项目内可写范围不限制，仍须遵守章程里的禁止事项。
- **结对流程**：开发甲、开发乙均可写项目文件，完成后相互审查；本轮实现方统一提交。
- **独立开发**：只有一个“独立开发”职务，接手方负责规划、实现、自查和提交，项目内可写范围不限制。支持直接 `init --template 独立开发 --role 独立开发=codex --goal "目标"`。
- **自定义**：用 `--template 自定义 --template-file .\team.toml` 读取用户 TOML。可从 [task.toml](templates/task.toml) 或 [pair.toml](templates/pair.toml) 复制修改。

`init [项目] --template <名字> --role <位置>=<agent> ... --goal "目标"` 要求每个位置恰好分配一个 AI，且一个 AI 只能担任一个位置。`human`、`bridge`、`all` 是保留名称。`role [项目] <agent> <位置>` 在目标位置已占用时交换双方职务（用户已选定的行为），数据库章程版本加一，权限立即生效；新增参与者应重新 `init`。重复设置同职务不产生伪变更事件。

`permission` 将覆盖设置保存到“项目 + 职务位置”，不改模板文件。未指定的列表保留当前值，指定的列表整体替换；传入空字符串可清空列表，空 `write` 表示项目内路径不受此层限制。PowerShell 5.1 传递空参数可能丢失，可传入 `--write ' '`（空白会被清理）或通过界面清空。`--reset` 不能与列表选项合用，它恢复初始化时保存的模板默认权限。职务交换保留位置的覆盖设置；重新初始化或交接按新模板重建权限并清除原覆盖。

```powershell
& $bridge permission D:\code\example 规划审查 --allow "写任务书" --allow "审查代码" --write 'docs/**' --write AGENTS.md
& $bridge permission D:\code\example 规划审查 --reset
```

保存或重置后章程版本递增，记录 `permission` 历史；若启用了规则文件输出，两个标记块同步更新。无效写入规则或标记块损坏会拒绝操作并保持原状态。

默认以系统身份 `bridge` 向 kickoff 职务发送 `first_task`，向其他参与者分别发送 `first_task_others`；任务书流程和结对流程均分配两个 AI，因此产生两条消息；独立开发只产生一条消息。使用 `--no-kickoff` 可省略派发，适合正在进行中的项目。初始化和职务交换由数据库触发器记录，`history --kind init` / `--kind role` 可查看。

`--write-rules` 把生成的章程写到项目键对应根目录的 `AGENTS.md` 与 `CLAUDE.md` 的下列标记内；不存在则新建，已有区块则替换，区块外的原始字节（包括 BOM、换行）保持不变。标记不完整或重复时拒绝写入。启用此选项后，`role` 和 `permission` 也会同步更新两个标记块；重新 `init` 时是否导出由当次 `--write-rules` 决定。桌面向导默认勾选写入规则文件，命令行仍需显式传入该选项。文件先在同目录暂存、刷盘，再用 Windows 原子替换；普通写入失败会回滚数据库并尝试恢复文件；数据库与文件系统无法构成跨系统的断电原子事务。

```markdown
<!-- AI Bridge 章程开始（由 bridge 生成，请勿手动修改此区块） -->
生成的章程
<!-- AI Bridge 章程结束 -->
```

公告板在本会话首次查看、重新初始化、职务或权限变化后完整显示章程、自己的权限和各方职务；之后显示 `协作章程 v2（本会话已显示过，未变化）`。缓存按会话、项目保存，人用 `show` 始终展示完整内容，不消耗 AI 的首次显示。

自定义模板使用 UTF-8 TOML，顶层 `name`、`charter` 必须放在 `[[roles]]` 之前；格式如下：

```toml
name = "我的流程"
charter = "项目目标：{goal}。{规划} 规划，{实现} 执行。"

[[roles]]
slot = "规划"
duties = "拆任务和审查"
allow = ["读代码", "写文档"]
deny = ["修改实现代码", "提交和推送"]
write = ["docs/**"]
kickoff = true

[[roles]]
slot = "实现"
duties = "实现、测试和提交"
allow = ["实现和测试", "提交和推送"]
deny = ["修改审查意见"]
write = ["src/**", "tests/**", "*.md", "Cargo.toml"]

[first_task]
content = "请根据 {goal} 拟定首个任务，与 human 确认。"
[first_task_others]
content = "请等待 {规划} 的首个任务。"
```

每份模板需要且只能有一个 `kickoff = true` 位置；`first_task.to` 可省略，填写时必须是该位置。`{goal}` 和 `{职务位置}` 只展开一次，目标中的同名文字不会再次替换。所有章程自动附上 [公共规则](templates/common.md)，包括统一提交、留言权限和结论落到仓库文档。

`allow`、`deny` 是供人和 AI 阅读的文字；Bridge 对 `claim_files` 强制检查的是 `write`。支持下列四种规则，不支持其他 glob 语法：

| 规则 | 含义 |
|---|---|
| `src/**` | src 目录及其所有后代文件 |
| `*.rs` | 任意层级的 .rs 文件 |
| `src/*.rs` | 仅 src 直接包含的 .rs 文件 |
| `Cargo.toml` | 完整相对路径 |

匹配前进行相对路径规范化，Windows 不区分大小写；空规则允许项目内任意路径。项目外的绝对路径和 `../` 越界路径不允许认领；未分配职务的 AI 不能认领文件，可通过留言联系用户。一个文件越权则整批拒绝。此层只约束 Bridge 认领操作，各 app 的原生权限接入计划在 T09 实现。

## 导出与交接（仅用户操作）

```powershell
# 导出默认写到项目内 docs/bridge/协作记录.md
& $bridge export D:\code\example
# 可指定仓库内的相对路径或绝对路径
& $bridge export D:\code\example --out docs/协作归档.md
# 交给 Claude 全盘接手；请只对真正要转为单人开发的项目操作
& $bridge handover D:\code\example --to claude
```

两个命令都会先把**已经打码**的预览放到仓库外的临时目录，打印预览路径、打码次数及类型、即将写入的文件路径。有 `.git/config` 远程地址时，逐一显示“提交并推送后内容可能公开”的提醒；远程 URL 中的密码也会打码。打开预览检查后，在终端的 `确认写入？(y/N)` 后输入小写 `y` 才会写入，其他输入或输入结束均取消。`--yes` 用于脚本和测试，跳过询问，但仍生成预览、执行打码并显示远程警告。预览保留供检查，用完后可删除打印出的预览文件及其所在的独立临时目录。

导出包含时间、章程版本和全文、职务、各会话最新状态、当前有效认领、最近 200 条消息和最近 500 条历史。`--out` 不能越出项目目录、覆盖 Bridge 数据库或 Git 元数据，也不能经过符号链接或 Windows 目录联接。省略项目使用当前目录；Git 项目按公共项目根目录输出，与公告板的项目识别一致。

打码使用内置扫描规则，不调用 AI，也不需要额外依赖：

- 密钥前缀：`sk-`、`sk-ant-`、`ghp_`、`gho_`、`github_pat_`、`AKIA`、`xoxb-`、`xoxa-`、`xoxp-`、`xoxr-`、`xoxs-`、`AIza`，匹配前缀后至少四个令牌字符。
- `BEGIN … PRIVATE KEY` 至对应 `END … PRIVATE KEY` 的整块私钥；缺失结束标记时打码到文本末尾。
- 赋值符号 `=`／`:` 左侧的完整变量名，只要包含 `password`、`passwd`、`pwd`、`secret`、`token`、`api_key`、`apikey`、`api-key`、`access_key`、`private_key`、`credential` 中任一关键词（不区分大小写），就对值打码。例如 `GITHUB_TOKEN`、`OPENAI_API_KEY`、`AWS_SECRET_ACCESS_KEY`、`DB_PASSWORD`、`client_secret`、`access_token`。`auth` 单独按 `_`、`-`、`.` 和大小写边界切分后匹配完整段：`AUTH`、`x-auth`、`authToken` 命中，`author`、`authority` 不命中。变量名支持字母、数字、`_`、`-`、`.`，可带引号、`export ` 或 `$env:`；赋值两边可有空格，值支持单双引号及非引号格式，带引号时保留引号并完整打码含空格的内容。
- `Bearer ` 后的令牌，以及 URL 中的 `user:password@` 凭据。

匹配内容替换为 `[已打码：类型]`；没有赋值的普通句子（例如“这个 token 很重要”）保持原样；关键词仅出现在普通变量的值中时，不会因此打码。按包含关键词的规则，`passwordless=true` 这类变量也会打码。自动扫描只覆盖这些格式，因此仍需检查预览。打码针对生成到仓库和预览里的内容，数据库原始记录保持原样；已有规则文件标记块之外的用户内容保持原字节。

交接确认后会生成 `docs/HANDOVER.md`，记录原目标、模板、章程、职务、各会话状态、各方及 human 的未读消息、未释放的认领、项目文档索引和最近 50 条历史。“风险”和“建议的下一步”由接手方补全；文档还写明接手方的新职责与首个任务，关闭或卸载 Bridge 后也可依此继续工作。

同时，Bridge 切换为“独立开发”模板、递增章程版本、保留接手方开关并将其开启，关闭其他已知 AI，释放它们的全部认领。其他 AI 的未读消息由 `bridge` 转发给接手方，注明原发送方和收件人，广播按原消息 ID 去重；原消息与已读记录不改。若当前初始化记录启用了 `--write-rules`，还会原子替换 `AGENTS.md`、`CLAUDE.md` 的章程标记块。接手方收到阅读交接文档、补全风险与下一步并向 human 汇报的任务。`history --kind handover` 可查看交接事件。

确认期间若数据库或目标文件变化，命令会要求重新预览。普通中途失败会回滚数据库，并恢复已经替换的文件或移除本次新建的文件与空目录；单个文件不会先被清空。多个文件和 SQLite 之间仍不构成断电／强制结束进程时的整体原子事务；若恢复也因权限变化失败，命令会明确报错。

恢复多 AI 协作时，由用户重新初始化；此次参与者的 AI 开关自动开启，章程版本递增：

```powershell
& $bridge init D:\code\example --template 任务书流程 --role 规划审查=claude --role 执行=codex --goal "继续实现项目目标" --no-kickoff --write-rules
```

原先开启了规则文件输出的项目，恢复时也应带 `--write-rules`，让仓库内的单人章程同步换回多人章程。若全局或项目总开关已关闭，需要用户先执行对应的 `on`。

## 项目识别与窗口会话

传入路径会向上查找 `.git`。Bridge 直接读取 `.git` 目录或文件里的 `gitdir:`，并读取 `commondir` 找到公共 Git 目录，不调用 git 命令。公共目录名为 `.git` 时用它的上级目录作为项目键，否则用公共目录本身；bare 仓库也按其公共目录识别。主目录和 worktree 因此共用公告板，子模块按自己的 Git 目录识别。

没有 Git 信息时仍按原路径识别；`.git` 格式损坏或指向不存在的目录时，也回退原路径，并只在 stderr 写一行诊断。旧的 worktree 路径历史数据不自动合并。文件认领中的绝对路径相对于当前 worktree 根目录处理，以便两棵工作树的同一相对文件互相识别。

一个 MCP 服务器进程对应一个随机会话 ID。在每个项目里首次调用时，按 AI 分配未被两小时内活跃会话占用的最小正整数，显示为 `codex #1`、`codex #2`；同一进程进入不同项目时分别编号。编号过期后可以复用，内部仍用随机 ID 校验认领归属，避免新进程释放旧进程的文件。

公告板的身份行和状态显示分支及 worktree 根目录。正常分支显示名称，游离 HEAD 显示前七位；超过 2 小时未活动的会话折叠为“较早的会话（N）”。界面可直接展开；文字公告板用 `bridge_overview` 的 `include_older=true` 或命令行 `show --include-older` 展开。编号复用时，在同一事务中清理该编号的旧会话和旧状态，保留交互历史与认领的原会话归属。状态按项目、AI 和会话编号分别保存；不同窗口认领同一文件会冲突，释放只作用于自己的会话。

**消息始终按 agent 投递，已读记录也按 agent 共享。** 给 codex 的消息可由任意 codex 窗口读取；一个窗口读过后，另一个窗口不会重复收到未读提示。人用 CLI 不创建 MCP 会话。

## 七个 AI 工具

| 工具 | 用途 |
|---|---|
| `bridge_overview` | 查看协作章程、职务权限、状态、认领和未读消息 |
| `update_status` | 更新任务、进度、卡点和下一步 |
| `send_message` | 给 claude、codex、human 或 all 留言 |
| `read_messages` | 读取并标记消息，或查询最近消息 |
| `claim_files` | 整批认领文件，越权或冲突时整批拒绝 |
| `release_files` | 释放自己的认领 |
| `list_projects` | 查看项目并标注开关状态 |

认领默认有效期 60 分钟，同一身份再次认领会续期；它是协作约定，不是文件系统锁。
典型流程是：查看公告板 → 认领文件 → 更新进度 → 留言交接 → 释放认领。

## 让 AI 自动响应

Bridge 使用拉取模式：发到公告板的消息要等 AI 主动查看才会进入它的对话。聊天页显示消息编号和 human 所发消息的逐个收件人回执；“已读”表示该 AI 读取过公告板消息，不表示任务已完成。广播会列出当前项目已知的 AI，已读按 AI 共享，不区分同一 AI 的多个窗口。

安装新版后，可以在 Claude 的项目对话里说“开始监视 Bridge”。Claude 应先查看当前公告板，处理已有授权范围内的待办，再用其宿主提供的后台监视能力运行：

```powershell
& "$env:USERPROFILE\.bridge\bin\bridge-mcp.exe" wait D:\Bridge --agent claude --follow
```

需要马上处理时，仍可到对应 app 提醒 AI。`wait` 只产生新消息提示，AI 收到提示后还需调用 Bridge 工具读取并处理消息。它不会自行启动 AI 会话。终端里手动运行这条命令也不会让一个已经结束的 AI 回合自动恢复；自动唤醒依赖所在 app 的监视能力。

```powershell
# 单次等待：收到启动后发来的第一条新未读消息就返回
& "$env:USERPROFILE\.bridge\bin\bridge-mcp.exe" wait D:\Bridge --agent codex --timeout 300
# 省略项目时使用当前目录，并按 Git 仓库归一；持续输出
& "$env:USERPROFILE\.bridge\bin\bridge-mcp.exe" wait --agent claude --follow --interval 3
```

- 只匹配发给指定 AI 或所有人、且发送者不是该 AI 的新未读消息；启动前的消息不会触发。所以监视启动前先查看公告板，收到提示后读取完整待办，再继续等待。
- 默认每 3 秒检查一次，`--interval` 最小 1 秒。默认不超时；`--follow` 也可配 `--timeout` 限制运行时长。
- 提示压成一行，正文最多 200 个字符；human 的软件界面/命令行来源与公告板一致。
- 退出码：新消息 `0`、超时 `2`（打印“等待超时”）、协作关闭 `3`（中文原因）。其他参数/数据库错误也使用 `2`，但写入 stderr；不能把它们当成正常超时。
- 等待本身只读，不标已读、不新建会话、不迁移数据库；启动时检查总开关、项目和 AI 开关，运行中在下一次轮询检测到关闭后停止。
- Codex 当前回合可阻塞等待并继续处理；跨回合可用当前聊天的定时监视（heartbeat）。本环境没有证据表明单独的后台进程 stdout 能唤醒已结束的回合。详细调查见 [T08d 完成报告](docs/tasks/T08d-等待命令与已读显示.md)。
- AI 每次读取上下文、判断和处理消息都会消耗对应服务的额度；频繁定时唤醒即使无事可做也有调用开销。`wait` 程序本身只查询本地 SQLite，不调用模型。
- 只处理用户已授权且需要行动的消息，无新情况时保持安静，不对单纯的“收到”“谢谢”再回复，避免两个 AI 互相触发。停止监视时终止后台等待或停用对应定时任务。

本机定时任务需要保持电脑开机、app 运行且项目目录可用；同聊天任务可以保留上下文并设置停止条件。参见 [OpenAI 官方定时任务说明](https://learn.chatgpt.com/docs/automations?surface=app)。例如按用户要求每 5 分钟检查，连续 30 分钟没有新消息且没有相关待办后停用：计时从启动时间和最近一条符合条件消息时间的较晚者开始，停用成功后再通知用户；读取失败不能视为没有新消息。

## 数据与测试

- 默认数据库为用户主目录下 `.bridge/bridge.db`；Windows 使用 `USERPROFILE`。
- `BRIDGE_DB` 可指定其他数据库，自定义路径的父目录需存在。AI 和命令行应使用相同路径。
- `BRIDGE_AGENT` 指定 AI 身份，缺省为 `unknown`；人用命令固定使用 human。
- `BRIDGE_FAKE_NOW` **只用于测试**，格式 `YYYY-MM-DD HH:MM:SS`，控制业务时间和认领清理比较。
  触发器中的开关／删除认领事件按迁移 SQL 使用 SQLite 的真实本地时间。

当前数据库版本为 7；迁移在事务中完成，早期版本的消息与状态会回填到历史表。
版本 7 为 `reads` 新增可空的 `read_at`，记录首次读取时间；历史已读记录保留为空并显示“已读”，不推测过去的读取时间。升级无需重新初始化。
版本 6 新增项目职务权限覆盖表，并清理重复会话编号（保留最近活跃的拥有者，删除其他拥有者的旧状态）；消息、已读、认领和事件历史不变，既有章程版本不变。从 T08a 升级后无需重新初始化。
版本 5 为消息新增 `via` 来源字段（旧数据为 `mcp`），新增消息的历史记录同步保存来源；不会自动重写既有项目章程。新增公共规则会在用户下次初始化时写入章程。

版本 3 保留原消息、已读、开关和历史；旧状态/认领迁为会话 #1，并建立迁移来源会话（PID 0），避免新窗口冒领仍有效的旧认领。版本 4 保留这些数据，新增初始化信息、职务和会话已显示的章程版本；尚未初始化的旧项目需要用户初始化一次。旧版程序会拒绝写入新数据库，因此升级时需完全退出两端和桌面程序，在独立终端安装新版，再重开桌面和两端；不使用后台安装助手。
每个进程对同一数据库路径只初始化一次，首次并发启动有锁等待及有限重试；更高版本的数据库拒绝写入。
测试使用独立临时 `BRIDGE_DB`，不读写用户真实数据库。

```powershell
cargo fmt --check
cargo clippy --workspace --all-targets -- -D warnings
cargo test --workspace
cargo build --release -p bridge-mcp
python -X dev -W error -m unittest discover -s tests/conformance -t . -v
powershell -NoProfile -ExecutionPolicy Bypass -File .\scripts\test-install-local.ps1
```

一致性测试默认使用仓库中的 Rust release 可执行文件；找不到时会提示先编译。
`BRIDGE_CMD` 可指定单个可执行文件的路径，不能包含命令参数。
协议定义的唯一现役来源是 `crates/bridge-core/resources/`；测试直接读这份定义，其他响应仅归一化时间后逐字比较。
CI 仅在 Windows 上运行 Rust 格式检查、Clippy、单元测试、release 编译和黑盒契约测试，以及 `npm ci`、TypeScript 检查、Vitest、Vite 和 Tauri 无安装包构建，并在 Windows PowerShell 5.1 与 PowerShell 7 下验证安装脚本。Linux 和冻结版 Python 不再纳入 CI。

## T05b 安装位置切换与回滚

当前有效安装位置为 `%USERPROFILE%\.bridge\bin\bridge-mcp.exe`。T05b 切换前的完整配置备份为 `~/.claude.json.bak-t05b` 和 `~/.codex/config.toml.bak-t05b`；这次只修改各自 `bridge.command`，不修改 args、身份或其他设置。

如需撤销本次配置变更，先退出两端，再恢复备份：

```powershell
Copy-Item -LiteralPath "$env:USERPROFILE\.claude.json.bak-t05b" -Destination "$env:USERPROFILE\.claude.json" -Force
Copy-Item -LiteralPath "$env:USERPROFILE\.codex\config.toml.bak-t05b" -Destination "$env:USERPROFILE\.codex\config.toml" -Force
```

备份引用的是旧 AppData 路径，而 T05 事故中该路径在用户的普通 PowerShell 中不存在；**恢复 `.bak-t05b` 只撤销配置，不能保证恢复连接**。应优先保留新的 `.bridge/bin/` 安装路径修复问题。恢复完整配置也会覆盖备份之后的其他设置，执行前应核对。

Claude 的旧私有目录 `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\AI Bridge\` 由用户在 Claude 已切换新路径、重连成功后自行清理，安装脚本不删除它。更早的 `.bak-t05` 备份继续保留。

## T05 回滚

切换前的完整配置备份为 `~/.claude.json.bak-t05` 和 `~/.codex/config.toml.bak-t05`。
退出两个 app 后，先恢复旧配置引用的入口和源码，再恢复配置：

```powershell
cd D:\Bridge
# 原路径不存在时，从归档恢复；已有文件时先核对，避免覆盖修改
Copy-Item -LiteralPath .\legacy\bridge.py -Destination .\bridge.py
New-Item -ItemType Directory -Path .\src -Force | Out-Null
Copy-Item -LiteralPath .\legacy\src\bridge_mcp -Destination .\src\bridge_mcp -Recurse

Copy-Item -LiteralPath "$env:USERPROFILE\.claude.json.bak-t05" -Destination "$env:USERPROFILE\.claude.json" -Force
Copy-Item -LiteralPath "$env:USERPROFILE\.codex\config.toml.bak-t05" -Destination "$env:USERPROFILE\.codex\config.toml" -Force
```

然后重启 Claude app 和 Codex app。恢复配置会回到备份时的完整设置；迁移 SQL 保留在根目录 `migrations/`。
此历史回滚只适用于数据库版本 2。T06a 升级到版本 3 后，冻结版会拒绝写入，不能只恢复旧配置继续使用。

## 代码结构

- `crates/bridge-core/`：数据库、路径、开关、工具、中文格式化。
- `crates/bridge-mcp/`：stdio MCP 与命令行入口。
- `migrations/`：中立位置的编号 SQL 迁移。
- `templates/`：编译内置的 TOML 模板和公共章程规则，也可复制为自定义模板。
- `scripts/`：本地安装及安装集成验证。
- `tests/conformance/`：Rust 黑盒行为规格。


Python 版已冻结，仅作为历史参考保留在 [legacy/](legacy/README.md)，其历史单元测试仅手动运行，不再纳入 CI：在 `legacy/` 下执行 `python -X dev -W error -m unittest discover -s tests -v`。


## 项目内协作记录副本

桌面软件可以把协作历史自动写入项目的 `.bridge/协作记录-YYYY-MM.md`。集中数据库仍是主存储；副本保留消息全文与历史事件，不打码，只适合本地使用。公开分享请使用“导出”。桌面关闭时不写，重新运行后补齐；项目或全局关闭期间暂停。

已有项目默认不开启：在项目“设置”的“项目内协作记录副本”中明确开启。新项目初始化向导默认勾选保存副本及加入 `.gitignore`，确认后才修改项目文件。设置页也提供明确的“确认加入”按钮。Git 仓库未受忽略规则保护时会显示隐私提醒；规则检测调用本机 Git，Git 不可用时显示原因，不能据此认定记录已受保护。

进度存放在软件的 `app.json`；记录先刷盘再保存进度。副本的隐藏事件边界用于中断恢复，请勿手工修改。目录不可用或写入失败会在设置页显示原因并重试，不阻塞其他项目。彻底删除 Bridge 项目只清除协作数据及副本偏好，项目里的副本仍保留；交接文档会列出本地副本索引。


## 等你处理提醒

AI 需要你确认、授权或决定时，可以给 `human` 发送消息并设置 `needs_action=true`。这类消息即使在聊天页被自动标记为已读，也会继续显示在项目标题上方的“等你处理”横幅中。点“已处理”，或在该项目给这个 AI 回复一条时间晚于原消息的消息，才会完成；发给“所有人”的回复会完成对应的更早待办。

最近两小时活跃会话的非空卡点也会显示。点“已知道”只隐藏当前这一次卡点；AI 改变卡点内容后会重新出现，清空卡点或会话超过两小时未活跃后不再显示。多个事项可以展开，“查看”会定位到具体消息或公告板。处理和知道操作都写入历史。

有待办的项目显示橙色标记；托盘橙色优先于普通未读红点，并显示待处理数量。新待办使用独立的通知去重记录，不受聊天页已读影响，窗口打开时也会通知。软件通知开关和全局关闭仍然有效；通知关闭时横幅仍保留。通知失败时显示错误，避免每轮重复弹窗，横幅和待办数据不丢失。

T08g 将数据库升级到版本 8，保留原消息、历史及已读时间。统一升级 MCP 与桌面程序后再启动，避免旧版 MCP 不支持新版数据库；不需要重新初始化项目。
