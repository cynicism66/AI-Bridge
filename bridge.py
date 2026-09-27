"""Bridge：让 Claude 和 Codex 互相看到状态、留言、认领文件的 MCP 服务器。

零依赖（只用 Python 标准库），通过 stdio 与 Claude app / Codex app 通信。
所有数据存在同一个 SQLite 文件里，按项目根目录区分，可同时用于多个项目。

环境变量：
  BRIDGE_AGENT  当前是谁（claude / codex），在各 app 的 MCP 配置里设置
  BRIDGE_DB     数据库路径，默认与本脚本同目录的 bridge.db

命令行查看公告板：
  python bridge.py show [项目路径]
"""

import json
import os
import sqlite3
import sys
from datetime import datetime, timedelta

AGENT = os.environ.get("BRIDGE_AGENT", "unknown").strip().lower() or "unknown"
DB_PATH = os.environ.get("BRIDGE_DB") or os.path.join(os.path.dirname(os.path.abspath(__file__)), "bridge.db")
SERVER_INFO = {"name": "bridge", "version": "1.0.0"}
TIME_FMT = "%Y-%m-%d %H:%M:%S"

INSTRUCTIONS = """Bridge 是你和其他 AI（Claude / Codex）协作的公告板。所有工具都需要 project 参数：填当前项目根目录的绝对路径。
建议流程：开始任务前调用 bridge_overview；改文件前 claim_files；每完成一步 update_status；完成后 release_files。
看到对方留言要及时回复（send_message）。对方认领的文件不要改，先留言协商。"""


# ---------- 数据库 ----------

def db():
    conn = sqlite3.connect(DB_PATH, timeout=15)
    conn.row_factory = sqlite3.Row
    conn.execute("PRAGMA journal_mode=WAL")
    conn.executescript("""
        CREATE TABLE IF NOT EXISTS status (
            project TEXT, agent TEXT, task TEXT, progress TEXT, blockers TEXT, next_step TEXT,
            updated_at TEXT, PRIMARY KEY (project, agent));
        CREATE TABLE IF NOT EXISTS messages (
            id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT, sender TEXT, recipient TEXT,
            content TEXT, created_at TEXT);
        CREATE TABLE IF NOT EXISTS reads (
            message_id INTEGER, agent TEXT, PRIMARY KEY (message_id, agent));
        CREATE TABLE IF NOT EXISTS claims (
            project TEXT, path TEXT, agent TEXT, note TEXT, claimed_at TEXT, expires_at TEXT,
            PRIMARY KEY (project, path));
    """)
    return conn


def now():
    return datetime.now().strftime(TIME_FMT)


def norm_project(project):
    if not project or not str(project).strip():
        raise ValueError("缺少 project 参数：请填当前项目根目录的绝对路径")
    p = os.path.normcase(os.path.abspath(str(project).strip()))
    return p.replace("\\", "/").rstrip("/")


def norm_file(project, path):
    """统一成相对项目根目录、正斜杠、小写（Windows 不区分大小写）的写法。"""
    p = str(path).strip().replace("\\", "/")
    if os.path.isabs(p):
        full = os.path.normcase(os.path.abspath(p)).replace("\\", "/")
        if full.startswith(project + "/"):
            p = full[len(project) + 1:]
        else:
            p = full
    p = os.path.normcase(p).replace("\\", "/")
    while p.startswith("./"):
        p = p[2:]
    return p


def purge_expired(conn):
    conn.execute("DELETE FROM claims WHERE expires_at < ?", (now(),))


def unread_messages(conn, project):
    return conn.execute("""
        SELECT * FROM messages m
        WHERE project = ? AND sender != ? AND recipient IN ('all', ?)
          AND NOT EXISTS (SELECT 1 FROM reads r WHERE r.message_id = m.id AND r.agent = ?)
        ORDER BY id""", (project, AGENT, AGENT, AGENT)).fetchall()


# ---------- 格式化 ----------

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


def overview_text(conn, project, mark_read=False):
    purge_expired(conn)
    out = [f"项目：{project}", f"你的身份：{AGENT}", ""]
    statuses = conn.execute("SELECT * FROM status WHERE project = ? ORDER BY agent", (project,)).fetchall()
    out.append("== 各方状态 ==")
    out += [fmt_status(r) for r in statuses] or ["  （还没有人汇报状态）"]
    claims = conn.execute("SELECT * FROM claims WHERE project = ? ORDER BY agent, path", (project,)).fetchall()
    out += ["", "== 文件认领 =="]
    out += [fmt_claim(r) for r in claims] or ["  （没有文件被认领）"]
    unread = unread_messages(conn, project)
    out += ["", f"== 给你的未读消息（{len(unread)} 条）=="]
    out += [fmt_msg(r) for r in unread] or ["  （没有未读消息）"]
    if mark_read and unread:
        conn.executemany("INSERT OR IGNORE INTO reads VALUES (?, ?)", [(r["id"], AGENT) for r in unread])
        out.append("  （以上消息已标为已读）")
    return "\n".join(out)


# ---------- 工具实现 ----------

def t_overview(args):
    project = norm_project(args.get("project"))
    with db() as conn:
        return overview_text(conn, project, mark_read=args.get("mark_read", True))


def t_update_status(args):
    project = norm_project(args.get("project"))
    with db() as conn:
        conn.execute("INSERT OR REPLACE INTO status VALUES (?, ?, ?, ?, ?, ?, ?)", (
            project, AGENT, args.get("task", ""), args.get("progress", ""),
            args.get("blockers", ""), args.get("next_step", ""), now()))
        n = len(unread_messages(conn, project))
    tip = f"\n提示：你有 {n} 条未读消息，请用 read_messages 查看。" if n else ""
    return f"状态已更新（{AGENT}）。{tip}"


def t_send_message(args):
    project = norm_project(args.get("project"))
    content = (args.get("content") or "").strip()
    if not content:
        raise ValueError("消息内容不能为空")
    to = (args.get("to") or "all").strip().lower()
    with db() as conn:
        cur = conn.execute("INSERT INTO messages (project, sender, recipient, content, created_at) VALUES (?, ?, ?, ?, ?)",
                           (project, AGENT, to, content, now()))
    return f"消息 #{cur.lastrowid} 已发送给 {'所有人' if to == 'all' else to}。"


def t_read_messages(args):
    project = norm_project(args.get("project"))
    limit = int(args.get("limit", 20))
    with db() as conn:
        if args.get("include_read"):
            rows = conn.execute("""
                SELECT * FROM (SELECT * FROM messages WHERE project = ? AND (recipient IN ('all', ?) OR sender = ?)
                               ORDER BY id DESC LIMIT ?) ORDER BY id""",
                                (project, AGENT, AGENT, limit)).fetchall()
            return "\n".join(["最近的消息："] + [fmt_msg(r) for r in rows]) if rows else "还没有任何消息。"
        rows = unread_messages(conn, project)[:limit]
        if not rows:
            return "没有未读消息。"
        conn.executemany("INSERT OR IGNORE INTO reads VALUES (?, ?)", [(r["id"], AGENT) for r in rows])
    return "\n".join([f"未读消息（{len(rows)} 条，已标为已读）："] + [fmt_msg(r) for r in rows])


def t_claim_files(args):
    project = norm_project(args.get("project"))
    files = [norm_file(project, f) for f in args.get("files") or [] if str(f).strip()]
    if not files:
        raise ValueError("files 不能为空")
    ttl = max(1, int(args.get("ttl_minutes", 60)))
    expires = (datetime.now() + timedelta(minutes=ttl)).strftime(TIME_FMT)
    with db() as conn:
        purge_expired(conn)
        conflicts = []
        for f in files:
            r = conn.execute("SELECT * FROM claims WHERE project = ? AND path = ?", (project, f)).fetchone()
            if r and r["agent"] != AGENT:
                conflicts.append(r)
        if conflicts:
            return "认领失败，以下文件已被别人认领（本次一个都没认领）：\n" + "\n".join(fmt_claim(r) for r in conflicts) \
                + "\n请先用 send_message 和对方协商，或等对方释放。"
        conn.executemany("INSERT OR REPLACE INTO claims VALUES (?, ?, ?, ?, ?, ?)",
                         [(project, f, AGENT, args.get("note", ""), now(), expires) for f in files])
    return f"已认领 {len(files)} 个文件，到期 {expires}：\n" + "\n".join(f"  {f}" for f in files)


def t_release_files(args):
    project = norm_project(args.get("project"))
    files = args.get("files") or []
    with db() as conn:
        if files:
            paths = [norm_file(project, f) for f in files]
            n = sum(conn.execute("DELETE FROM claims WHERE project = ? AND path = ? AND agent = ?",
                                 (project, p, AGENT)).rowcount for p in paths)
        else:
            n = conn.execute("DELETE FROM claims WHERE project = ? AND agent = ?", (project, AGENT)).rowcount
    return f"已释放 {n} 个文件。"


def t_list_projects(args):
    with db() as conn:
        rows = conn.execute("""
            SELECT project, MAX(t) AS last FROM (
                SELECT project, updated_at AS t FROM status
                UNION ALL SELECT project, created_at FROM messages
                UNION ALL SELECT project, claimed_at FROM claims)
            GROUP BY project ORDER BY last DESC""").fetchall()
    return "\n".join(["用过 Bridge 的项目："] + [f"  {r['project']}（最近活动 {r['last']}）" for r in rows]) \
        if rows else "还没有项目使用过 Bridge。"


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
      "to": {"type": "string", "description": "收件人：claude / codex / all，默认 all"}}, ["project", "content"]),
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


# ---------- MCP 协议（JSON-RPC over stdio） ----------

def tool_list():
    return [{"name": name, "description": desc,
             "inputSchema": {"type": "object", "properties": props, "required": req}}
            for name, _, desc, props, req in TOOLS]


def handle(req):
    method, params = req.get("method"), req.get("params") or {}
    if method == "initialize":
        return {"protocolVersion": params.get("protocolVersion", "2025-06-18"),
                "capabilities": {"tools": {}}, "serverInfo": SERVER_INFO, "instructions": INSTRUCTIONS}
    if method == "ping":
        return {}
    if method == "tools/list":
        return {"tools": tool_list()}
    if method == "tools/call":
        fn = HANDLERS.get(params.get("name"))
        if not fn:
            return {"content": [{"type": "text", "text": f"未知工具：{params.get('name')}"}], "isError": True}
        try:
            return {"content": [{"type": "text", "text": fn(params.get("arguments") or {})}]}
        except Exception as e:
            return {"content": [{"type": "text", "text": f"出错：{e}"}], "isError": True}
    raise LookupError(method)


def send(obj):
    sys.stdout.buffer.write(json.dumps(obj, ensure_ascii=False).encode("utf-8") + b"\n")
    sys.stdout.buffer.flush()


def serve():
    for line in sys.stdin.buffer:
        line = line.strip()
        if not line:
            continue
        try:
            req = json.loads(line.decode("utf-8"))
        except ValueError:
            send({"jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}})
            continue
        if "id" not in req:  # 通知，无需回复
            continue
        try:
            send({"jsonrpc": "2.0", "id": req["id"], "result": handle(req)})
        except LookupError:
            send({"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32601, "message": f"Method not found: {req.get('method')}"}})
        except Exception as e:
            send({"jsonrpc": "2.0", "id": req["id"], "error": {"code": -32603, "message": str(e)}})


def show(project=None):
    sys.stdout.reconfigure(encoding="utf-8")
    global AGENT
    AGENT = "human"
    with db() as conn:
        if project:
            print(overview_text(conn, norm_project(project)))
            print("\n== 最近消息 ==")
            rows = conn.execute("SELECT * FROM (SELECT * FROM messages WHERE project = ? ORDER BY id DESC LIMIT 20) ORDER BY id",
                                (norm_project(project),)).fetchall()
            print("\n".join(fmt_msg(r) for r in rows) or "  （无）")
        else:
            print(t_list_projects({}))


if __name__ == "__main__":
    if len(sys.argv) > 1 and sys.argv[1] == "show":
        show(sys.argv[2] if len(sys.argv) > 2 else None)
    else:
        serve()
