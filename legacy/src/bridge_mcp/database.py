"""进程内一次初始化；编号迁移与 Rust 共用 SQL。"""

from pathlib import Path
import sqlite3
import threading
import time

VERSION = 2
_initialized = set()
_lock = threading.Lock()


def migration_directory():
    """支持 legacy 源码运行，以及回滚时恢复到原 src 位置。"""
    root = Path(__file__).resolve().parents[2]
    direct = root / "migrations"
    return direct if direct.is_dir() else root.parent / "migrations"


def check_version(conn):
    version = int(conn.execute("PRAGMA user_version").fetchone()[0])
    if version > VERSION:
        raise ValueError(f"数据库版本 {version} 高于当前支持的版本 {VERSION}，请升级 Bridge 后再写入。")
    return version


def statements(sql):
    pending = ""
    for line in sql.splitlines(keepends=True):
        pending += line
        if sqlite3.complete_statement(pending):
            yield pending
            pending = ""
    if pending.strip():
        raise ValueError("迁移 SQL 不完整")


def initialize(conn, path):
    conn.execute("PRAGMA busy_timeout=15000")
    # 每次写连接仍检查版本，防止已运行的旧程序在其他进程升级后继续写入。
    check_version(conn)
    key = str(path.resolve())
    with _lock:
        if key in _initialized:
            return
        for attempt in range(10):
            try:
                conn.execute("PRAGMA journal_mode=WAL")
                break
            except sqlite3.OperationalError as error:
                if not any(word in str(error).lower() for word in ("locked", "busy")) or attempt == 9:
                    raise
                time.sleep(0.02 * (attempt + 1))
        conn.execute("BEGIN IMMEDIATE")
        try:
            version = check_version(conn)
            for number in range(version + 1, VERSION + 1):
                sql = (migration_directory() / f"{number:03}.sql").read_text(encoding="utf-8")
                for statement in statements(sql):
                    conn.execute(statement)
                conn.execute(f"PRAGMA user_version={number}")
            conn.commit()
        except BaseException:
            conn.rollback()
            raise
        _initialized.add(key)
