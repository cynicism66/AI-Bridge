"""SQLite 连接、建表和数据读写；结果仅包含结构化数据。"""

import os
import sqlite3
from datetime import datetime, timedelta
from pathlib import Path

TIME_FMT = "%Y-%m-%d %H:%M:%S"
_CUSTOM_DB = os.environ.get("BRIDGE_DB")
DB_PATH = _CUSTOM_DB or Path.home() / ".bridge" / "bridge.db"


def db():
    if not _CUSTOM_DB:
        Path(DB_PATH).parent.mkdir(parents=True, exist_ok=True)
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
    expires = (datetime.now() + timedelta(minutes=ttl)).strftime(TIME_FMT)
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
                UNION ALL SELECT project, claimed_at FROM claims)
            GROUP BY project ORDER BY last DESC""").fetchall()
    return [dict(r) for r in rows]


def recent_messages(project):
    with db() as conn:
        rows = conn.execute("SELECT * FROM (SELECT * FROM messages WHERE project = ? ORDER BY id DESC LIMIT 20) ORDER BY id",
                            (project,)).fetchall()
    return [dict(r) for r in rows]
