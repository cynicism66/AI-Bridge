# 多 AI 协作规则（Bridge）

你可能和另一个 AI（Claude 或 Codex）在同一个项目里工作。如果有 `bridge` 工具可用：
`project` 参数填当前项目根目录的绝对路径。收到"未开启"或"已关闭"的提示后，本次会话不再调用 Bridge，正常工作。
**只回答问题、不修改项目文件的会话，不要调用 Bridge**，以免读走给干活窗口的消息。

1. 开始任务前调用 `bridge_overview`，看对方在做什么、有没有给你的留言。
2. 改文件前用 `claim_files` 认领，失败就先用 `send_message` 和对方商量；做完用 `release_files` 释放。
3. 每完成一个阶段用 `update_status` 更新状态。
4. 需要对方回复才能继续时：发完消息，用完整安装路径原地等回复，不依赖 PATH。安装位置是 `%USERPROFILE%\.bridge\bin\bridge-mcp.exe`；PowerShell 运行 `& "$env:USERPROFILE\.bridge\bin\bridge-mcp.exe" wait "<项目目录>" --agent <你的名字> --timeout 300`，Git Bash 运行 `"$USERPROFILE/.bridge/bin/bridge-mcp.exe" wait "<项目目录>" --agent <你的名字> --timeout 300`。把命令工具的执行超时设为大于 300 秒（例如 360 秒 / 360000 毫秒）；支持后台进程或会话句柄的工具可先启动再分段等待，避免被默认超时提前终止。有新消息返回后，先调用 `read_messages` 读取并标记已读，再处理，避免本轮结束时 Stop 钩子重复投递。等待超时可再等一轮，最多三轮，然后告诉用户；开关关闭或命令报错时停止等待并说明原因。
5. 需要用户处理时，给 human 发消息，写清楚要用户做什么。
6. 留言和状态一律用中文。只是确认收到的消息不用回复，避免来回空转。
