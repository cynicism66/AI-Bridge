# T05b：把安装位置移出 AppData（T05 补丁）

## 背景
T05 把 `bridge-mcp.exe` 装在 `%LOCALAPPDATA%\AI Bridge\bin\`，结果出了两次事故：
1. Codex 在 Codex app 里运行安装脚本，文件被 MSIX 转存到了 `Packages\OpenAI.Codex_*\LocalCache\`；
2. Claude 在 Claude app 里又运行了一次，文件又被转存到 `Packages\Claude_pzs8sxrjxfjjc\LocalCache\`。**Claude app 也是 MSIX 应用。**

用户在自己的 PowerShell 里查过，`Test-Path "%LOCALAPPDATA%\AI Bridge\bin\bridge-mcp.exe"` 返回 **False**：真实路径上根本没有 exe，Claude 能用只是因为它读的是自己私有目录里的那份。另外，T05 加的"祖先进程有包身份就拒绝安装"这条检测，在 Windows Terminal（它本身也是 MSIX）里会**误报**。

用户决定（DECISIONS 第 27 条）：**安装位置改为 `%USERPROFILE%\.bridge\bin\`**。MSIX 只转存 AppData，数据库一直放在 `~/.bridge/` 下，从来没出过问题。

## 要做的事
1. **安装脚本**：`scripts/install-local.ps1` 的目标路径改为 `%USERPROFILE%\.bridge\bin\bridge-mcp.exe`。**删掉**祖先进程包身份检测和 LocalCache 后检（以及 `package-context.ps1` 里只为它们服务的代码和测试），因为新位置不会被转存，这些检测已经没用了，还会误报。保留：原子替换、哈希校验、被占用时给出中文提示。
2. **切换配置（用户已同意）**：备份两个配置文件（后缀 `.bak-t05b`），把 `bridge` 的 `command` 改成 `C:\Users\wangq\.bridge\bin\bridge-mcp.exe`，其他内容一个字节都不能动，用 diff 证明。
3. **安装**：新位置不受 MSIX 影响，由谁运行安装脚本都可以。安装后在报告里记录文件的真实大小和哈希。
4. **文档**：README 的安装、回滚两节，`AGENTS.md` 的"不能打断正在使用的配置"那一条，全部改为新路径；删掉"必须由用户在普通 PowerShell 执行"这类已经不需要的限制，并说明原因（MSIX 只转存 AppData）。
5. **清理说明**（不要自己删，写进报告，由用户执行）：
   - `%LOCALAPPDATA%\Packages\Claude_pzs8sxrjxfjjc\LocalCache\Local\AI Bridge\`：Claude 的私有副本。要等 Claude 切换到新路径、重新连接 bridge 之后才能删，在那之前它正在被使用
   - `.bak-t05` 备份保留，不删
6. **报告**：在 T05 的完成报告后面补一节"T05b"，写明两次事故的经过、Windows Terminal 误报的原因、为什么改路径比加检测更可靠。

## 验收标准
- [ ] 安装脚本安装到 `%USERPROFILE%\.bridge\bin\`；用户在自己的 PowerShell（Windows Terminal 里的）中用 `Test-Path` 能看到文件
- [x] 两个配置文件和 `.bak-t05b` 的 diff 只有 `bridge` 的 command 那一行
- [ ] 真实环境验证：Codex 和 Claude 分别重启或重连之后，实际 MCP 调用成功；`Get-CimInstance` 显示两个 bridge-mcp.exe 的路径都是 `C:\Users\wangq\.bridge\bin\bridge-mcp.exe`
- [x] 契约测试、`cargo fmt`、`clippy`、`test`、`legacy` 测试全部通过；CI 全绿
- [x] README、AGENTS 已更新；报告已补 T05b 一节

## 完成报告

代码修改、安装及已授权的配置切换已完成；本地检查全部通过，真实客户端重连和用户侧 Test-Path 验收待完成。详细经过、配置差异证据及清理说明见 [T05 报告的 T05b 节](T05-切换到Rust版.md#t05b安装位置移出-appdata2026-09-28)。实现已提交为 `1af4691`，[CI 六个任务全部通过](https://github.com/cynicism66/AI-Bridge/actions/runs/36364821672)。安装文件的独立诊断确认实际映像位于 `.bridge/bin/`；最后两项用户/客户端验收尚未收到结果，因此本任务尚未最终结项。
