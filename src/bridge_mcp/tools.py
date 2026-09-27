"""七个协作工具及中文文字格式化。"""

import os
from functools import wraps

from . import store
from .paths import norm_file, norm_project


def current_agent():
    return os.environ.get("BRIDGE_AGENT", "unknown").strip().lower() or "unknown"


PROJECT_OFF = "Bridge 未在此项目开启（由用户控制）。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。"
GLOBAL_OFF = "Bridge 已被用户全局关闭。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。"


def disabled_text(state):
    if not state["global_enabled"]:
        return GLOBAL_OFF
    return "" if state["project_enabled"] else PROJECT_OFF


def project_tool(function):
    @wraps(function)
    def checked(args):
        project = norm_project(args.get("project"))
        notice = disabled_text(store.switch_state(project))
        return notice or function(args)
    return checked


def overview_text(project, agent=None, mark_read=False):
    if agent is None:
        agent = current_agent()
    data = store.overview(project, agent, mark_read)
    out = [f"项目：{project}", f"你的身份：{agent}", ""]
    out.append("== 各方状态 ==")
    out += [fmt_status(r) for r in data["statuses"]] or ["  （还没有人汇报状态）"]
    out += ["", "== 文件认领 =="]
    out += [fmt_claim(r) for r in data["claims"]] or ["  （没有文件被认领）"]
    unread = data["unread"]
    out += ["", f"== 给你的未读消息（{len(unread)} 条）=="]
    out += [fmt_msg(r) for r in unread] or ["  （没有未读消息）"]
    if mark_read and unread:
        out.append("  （以上消息已标为已读）")
    return "\n".join(out)


@project_tool
def t_overview(args):
    project = norm_project(args.get("project"))
    return overview_text(project, mark_read=args.get("mark_read", True))


@project_tool
def t_update_status(args):
    project = norm_project(args.get("project"))
    agent = current_agent()
    result = store.update_status(project, agent, args.get("task", ""), args.get("progress", ""),
                                 args.get("blockers", ""), args.get("next_step", ""))
    n = result["unread_count"]
    tip = f"\n提示：你有 {n} 条未读消息，请用 read_messages 查看。" if n else ""
    return f"状态已更新（{agent}）。{tip}"


@project_tool
def t_send_message(args):
    project = norm_project(args.get("project"))
    content = (args.get("content") or "").strip()
    if not content:
        raise ValueError("消息内容不能为空")
    to = (args.get("to") or "all").strip().lower()
    result = store.send_message(project, current_agent(), to, content)
    return f"消息 #{result['id']} 已发送给 {'所有人' if to == 'all' else to}。"


@project_tool
def t_read_messages(args):
    project = norm_project(args.get("project"))
    limit = int(args.get("limit", 20))
    rows = store.read_messages(project, current_agent(), limit, args.get("include_read"))
    if args.get("include_read"):
        return "\n".join(["最近的消息："] + [fmt_msg(r) for r in rows]) if rows else "还没有任何消息。"
    if not rows:
        return "没有未读消息。"
    return "\n".join([f"未读消息（{len(rows)} 条，已标为已读）："] + [fmt_msg(r) for r in rows])


@project_tool
def t_claim_files(args):
    project = norm_project(args.get("project"))
    files = [norm_file(project, f) for f in args.get("files") or [] if str(f).strip()]
    if not files:
        raise ValueError("files 不能为空")
    ttl = max(1, int(args.get("ttl_minutes", 60)))
    result = store.claim_files(project, current_agent(), files, ttl, args.get("note", ""))
    if result["conflicts"]:
        return "认领失败，以下文件已被别人认领（本次一个都没认领）：\n" + "\n".join(fmt_claim(r) for r in result["conflicts"]) \
            + "\n请先用 send_message 和对方协商，或等对方释放。"
    return f"已认领 {len(files)} 个文件，到期 {result['expires']}：\n" + "\n".join(f"  {f}" for f in files)


@project_tool
def t_release_files(args):
    project = norm_project(args.get("project"))
    paths = [norm_file(project, f) for f in args.get("files") or []]
    result = store.release_files(project, current_agent(), paths)
    return f"已释放 {result['count']} 个文件。"


def t_list_projects(args):
    if not store.switch_state()["global_enabled"]:
        return GLOBAL_OFF
    rows = store.list_projects()
    return "\n".join(["用过 Bridge 的项目："] + [
        f"  {r['project']}（最近活动 {r['last'] or '-'}，{'已开启' if r['enabled'] else '未开启'}）" for r in rows]) \
        if rows else "还没有项目使用过 Bridge。"


def show_text(project=None):
    if not project:
        return t_list_projects({})
    project = norm_project(project)
    overview = overview_text(project, agent="human")
    rows = store.recent_messages(project)
    return overview + "\n\n== 最近消息 ==\n" + ("\n".join(fmt_msg(r) for r in rows) or "  （无）")


def fmt_status(r):
    lines = [f"【{r['agent']}】更新于 {r['updated_at']}", f"  任务：{r['task'] or '-'}"]
    for label, key in (("进度", "progress"), ("卡点", "blockers"), ("下一步", "next_step")):
        if r[key]:
            lines.append(f"  {label}：{r[key]}")
    return "\n".join(lines)


def fmt_claim(r):
    note = f"（{r['note']}）" if r["note"] else ""
    return f"  {r['path']} ← {r['agent']}{note}，到期 {r['expires_at']}"


def fmt_msg(r):
    to = "所有人" if r["recipient"] == "all" else r["recipient"]
    return f"  #{r['id']} [{r['created_at']}] {r['sender']} → {to}：{r['content']}"


INSTRUCTIONS = """Bridge 是你和其他 AI（Claude / Codex）协作的公告板。所有工具都需要 project 参数：填当前项目根目录的绝对路径。
建议流程：开始任务前调用 bridge_overview；改文件前 claim_files；每完成一步 update_status；完成后 release_files。
看到对方留言要及时回复（send_message）。对方认领的文件不要改，先留言协商。
Bridge 的开关由用户通过命令行控制，新项目默认未开启，另有全局总开关。
收到“未开启”或“已关闭”的提示后，请忽略协作规则，正常工作；本次会话不要再调用 Bridge。"""


PROJECT = {"type": "string", "description": "当前项目根目录的绝对路径，例如 D:/code/ender-eda"}

TOOLS = [
    ("bridge_overview", t_overview, "查看公告板全貌：各方状态、文件认领情况、给你的未读消息。开始任务前先调用这个。",
     {"project": PROJECT, "mark_read": {"type": "boolean", "description": "是否把显示的未读消息标为已读，默认 true"}}, ["project"]),
    ("update_status", t_update_status, "更新你自己的工作状态，让对方知道你在做什么。每完成一步都应更新。",
     {"project": PROJECT,
      "task": {"type": "string", "description": "正在做的任务"},
      "progress": {"type": "string", "description": "做到哪一步了"},
      "blockers": {"type": "string", "description": "卡点或需要对方帮忙的地方，没有就留空"},
      "next_step": {"type": "string", "description": "接下来打算做什么"}}, ["project", "task"]),
    ("send_message", t_send_message, "给对方留言：提问、交接、通知、回复。",
     {"project": PROJECT,
      "content": {"type": "string", "description": "消息内容"},
      "to": {"type": "string", "description": "收件人：claude / codex / human / all，默认 all"}}, ["project", "content"]),
    ("read_messages", t_read_messages, "读取给你的未读消息（读后标为已读）。include_read=true 时显示最近的全部消息。",
     {"project": PROJECT,
      "include_read": {"type": "boolean", "description": "是否包含已读消息，默认 false"},
      "limit": {"type": "integer", "description": "最多返回多少条，默认 20"}}, ["project"]),
    ("claim_files", t_claim_files, "修改文件前先认领，防止和对方同时改同一个文件。有冲突时一个都不认领。",
     {"project": PROJECT,
      "files": {"type": "array", "items": {"type": "string"}, "description": "文件路径，相对项目根目录或绝对路径均可"},
      "note": {"type": "string", "description": "认领原因，例如 '重构布线模块'"},
      "ttl_minutes": {"type": "integer", "description": "认领时长（分钟），到期自动释放，默认 60"}}, ["project", "files"]),
    ("release_files", t_release_files, "释放你认领的文件。不传 files 则释放你在该项目的全部认领。",
     {"project": PROJECT,
      "files": {"type": "array", "items": {"type": "string"}, "description": "要释放的文件，留空表示全部"}}, ["project"]),
    ("list_projects", t_list_projects, "列出所有用过 Bridge 的项目。", {}, []),
]
HANDLERS = {name: fn for name, fn, *_ in TOOLS}


def tool_list():
    return [{"name": name, "description": desc,
             "inputSchema": {"type": "object", "properties": props, "required": req}}
            for name, _, desc, props, req in TOOLS]
