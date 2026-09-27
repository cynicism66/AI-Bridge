"""SQLite 连接、建表和数据读写；结果仅包含结构化数据。"""

import os
import sqlite3
from contextlib import contextmanager
from datetime import datetime, timedelta
from pathlib import Path

from .database import initialize

TIME_FMT = "%Y-%m-%d %H:%M:%S"


def db_path(create=True):
    custom = os.environ.get("BRIDGE_DB")
    if custom:
        return Path(custom)
    path = Path.home() / ".bridge" / "bridge.db"
    if create:
        path.parent.mkdir(parents=True, exist_ok=True)
    return path


@contextmanager
def db(read_only=False):
    path = db_path(create=not read_only)
    conn = (sqlite3.connect(path.resolve().as_uri() + "?mode=ro", uri=True, timeout=15)
            if read_only else sqlite3.connect(path, timeout=15))
    try:
        with conn:
            conn.row_factory = sqlite3.Row
            if not read_only:
                initialize(conn, path)
            yield conn
    finally:
        conn.close()


def now():
    if os.environ.get("BRIDGE_FAKE_NOW"):
        return datetime.strptime(os.environ["BRIDGE_FAKE_NOW"], TIME_FMT).strftime(TIME_FMT)
    return datetime.now().strftime(TIME_FMT)


def purge_expired(conn):
    conn.execute("DELETE FROM claims WHERE expires_at < ?", (now(),))


def unread_messages(conn, project, agent):
    rows = conn.execute("""
        SELECT * FROM messages m
        WHERE project = ? AND sender != ? AND recipient IN ('all', ?)
          AND NOT EXISTS (SELECT 1 FROM reads r WHERE r.message_id = m.id AND r.agent = ?)
        ORDER BY id""", (project, agent, agent, agent)).fetchall()
    return [dict(r) for r in rows]


def mark_messages_read(conn, rows, agent):
    conn.executemany("INSERT OR IGNORE INTO reads VALUES (?, ?)", [(r["id"], agent) for r in rows])


def overview(project, agent, mark_read=False):
    with db() as conn:
        purge_expired(conn)
        statuses = conn.execute("SELECT * FROM status WHERE project = ? ORDER BY agent", (project,)).fetchall()
        claims = conn.execute("SELECT * FROM claims WHERE project = ? ORDER BY agent, path", (project,)).fetchall()
        unread = unread_messages(conn, project, agent)
        if mark_read and unread:
            mark_messages_read(conn, unread, agent)
    return {"statuses": [dict(r) for r in statuses], "claims": [dict(r) for r in claims], "unread": unread}


def update_status(project, agent, task="", progress="", blockers="", next_step=""):
    with db() as conn:
        conn.execute("INSERT OR REPLACE INTO status VALUES (?, ?, ?, ?, ?, ?, ?)", (
            project, agent, task, progress, blockers, next_step, now()))
        return {"unread_count": len(unread_messages(conn, project, agent))}


def send_message(project, agent, to, content):
    with db() as conn:
        cur = conn.execute("INSERT INTO messages (project, sender, recipient, content, created_at) VALUES (?, ?, ?, ?, ?)",
                           (project, agent, to, content, now()))
    return {"id": cur.lastrowid}


def read_messages(project, agent, limit=20, include_read=False):
    with db() as conn:
        if include_read:
            rows = conn.execute("""
                SELECT * FROM (SELECT * FROM messages WHERE project = ? AND (recipient IN ('all', ?) OR sender = ?)
                               ORDER BY id DESC LIMIT ?) ORDER BY id""",
                                (project, agent, agent, limit)).fetchall()
            return [dict(r) for r in rows]
        rows = unread_messages(conn, project, agent)[:limit]
        if rows:
            mark_messages_read(conn, rows, agent)
    return rows


def claim_files(project, agent, files, ttl, note=""):
    expires = (datetime.strptime(now(), TIME_FMT) + timedelta(minutes=ttl)).strftime(TIME_FMT)
    with db() as conn:
        purge_expired(conn)
        conflicts = []
        for path in files:
            row = conn.execute("SELECT * FROM claims WHERE project = ? AND path = ?", (project, path)).fetchone()
            if row and row["agent"] != agent:
                conflicts.append(dict(row))
        if not conflicts:
            conn.executemany("INSERT OR REPLACE INTO claims VALUES (?, ?, ?, ?, ?, ?)",
                             [(project, path, agent, note, now(), expires) for path in files])
    return {"conflicts": conflicts, "expires": expires}


def release_files(project, agent, paths):
    with db() as conn:
        if paths:
            count = sum(conn.execute("DELETE FROM claims WHERE project = ? AND path = ? AND agent = ?",
                                    (project, path, agent)).rowcount for path in paths)
        else:
            count = conn.execute("DELETE FROM claims WHERE project = ? AND agent = ?", (project, agent)).rowcount
    return {"count": count}


def list_projects():
    with db() as conn:
        rows = conn.execute("""
            SELECT project, MAX(t) AS last FROM (
                SELECT project, updated_at AS t FROM status
                UNION ALL SELECT project, created_at FROM messages
                UNION ALL SELECT project, claimed_at FROM claims
                UNION ALL SELECT scope, NULL FROM settings WHERE scope != 'global')
            GROUP BY project ORDER BY last DESC""").fetchall()
        settings = dict(conn.execute("SELECT scope, enabled FROM settings"))
    return [{**dict(r), "enabled": bool(settings.get(r["project"], False))} for r in rows]


def recent_messages(project):
    with db() as conn:
        rows = conn.execute("SELECT * FROM (SELECT * FROM messages WHERE project = ? ORDER BY id DESC LIMIT 20) ORDER BY id",
                            (project,)).fetchall()
    return [dict(r) for r in rows]


def switch_state(project=None):
    settings = {}
    # 默认关闭时不创建数据库或表；已有数据库只读设置，不访问业务表。
    if db_path(create=False).is_file():
        with db(read_only=True) as conn:
            if conn.execute("SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings'").fetchone():
                scopes = ("global", project) if project is not None else ("global", "global")
                settings = dict(conn.execute("SELECT scope, enabled FROM settings WHERE scope IN (?, ?)", scopes))
    global_enabled = bool(settings.get("global", True))
    project_enabled = bool(settings.get(project, False))
    return {"global_enabled": global_enabled, "project_enabled": project_enabled,
            "enabled": global_enabled and project_enabled}


def set_enabled(enabled, project=None):
    with db() as conn:
        conn.execute("INSERT OR REPLACE INTO settings VALUES (?, ?)",
                     ("global" if project is None else project, int(enabled)))
