import json
import sqlite3
from contextlib import closing
from .support import ContractCase, ROOT, STAMP
from .test_sessions import content


class MigrationV5Tests(ContractCase):
    def test_v4_preserved_and_via_constraints(self):
        with closing(sqlite3.connect(self.database)) as db:
            for number in range(1, 5):
                db.executescript((ROOT / f"migrations/{number:03}.sql").read_text(encoding="utf-8"))
            db.execute("PRAGMA user_version=4")
            db.execute("INSERT INTO messages VALUES(1,?,'claude','all','升级前',?)", (self.project, STAMP))
            db.execute("INSERT INTO reads VALUES(1,'codex')")
            db.commit()
            tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%'")]
            old = {t: db.execute(f"SELECT * FROM {t}").fetchall() for t in tables}
        self.cli("status")
        self.cli("status")
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 5)
            for table, rows in old.items():
                actual = db.execute(f"SELECT * FROM {table}").fetchall()
                self.assertEqual([r[:len(rows[0])] for r in actual] if rows else actual, rows)
            self.assertEqual(db.execute("SELECT via FROM messages").fetchall(), [("mcp",)])
            self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
            with self.assertRaises(sqlite3.IntegrityError):
                db.execute("UPDATE messages SET via='fake'")

    def test_gui_cli_and_mcp_sources_in_board_read_and_history(self):
        self.enable()
        self.cli("post", self.project, "命令行发来", "--to", "codex")
        # 构造 GUI 存储边界；实际 GUI 写入另由 bridge-core 单元测试验证。
        with closing(sqlite3.connect(self.database)) as db:
            db.execute("INSERT INTO messages(project,sender,recipient,content,created_at,via) VALUES(?,'human','codex','界面发来',?,'gui')", (self.project, STAMP))
            db.commit()
        server = self.session()
        board = content(server.call("bridge_overview", project=self.project, mark_read=False))
        self.assertIn("human（命令行） → codex：命令行发来", board)
        self.assertIn("human（软件界面） → codex：界面发来", board)
        read = content(server.call("read_messages", project=self.project))
        self.assertIn("human（软件界面）", read)
        self.assertIn("human（命令行）", read)
        server.call("send_message", project=self.project, to="human", content="AI 回复")
        history = self.cli("history", self.project, "--kind", "message")
        self.assertIn("human（命令行） → codex 留言：命令行发来", history)
        self.assertIn("human（软件界面） → codex 留言：界面发来", history)
        self.assertIn("codex → human 留言：AI 回复", history)
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT via FROM messages ORDER BY id").fetchall(), [("cli",), ("gui",), ("mcp",)])
            detail = json.loads(db.execute("SELECT detail FROM events WHERE kind='message' ORDER BY id DESC LIMIT 1").fetchone()[0])
            self.assertEqual(detail["via"], "mcp")
