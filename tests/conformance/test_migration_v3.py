import json
import sqlite3
from contextlib import closing
from .support import ContractCase, ROOT, STAMP, normalized
from .test_sessions import content


class MigrationV3Tests(ContractCase):
    def test_v2_preserves_rows_and_recreates_status_and_switch_triggers(self):
        with closing(sqlite3.connect(self.database)) as db:
            for number in (1, 2):
                db.executescript((ROOT / f"migrations/{number:03}.sql").read_text(encoding="utf-8"))
            db.execute("PRAGMA user_version=2")
            db.execute("INSERT INTO settings VALUES (?,1)", (self.project,))
            db.execute("INSERT INTO status VALUES (?, 'codex', '旧任务', '', '', '', ?)", (self.project, STAMP))
            db.execute("INSERT INTO claims VALUES (?, 'old.rs', 'codex', '旧认领', ?, '2100-01-01 00:00:00')",
                       (self.project, STAMP))
            db.execute("INSERT INTO messages VALUES (NULL, ?, 'claude','codex','旧消息',?)", (self.project, STAMP))
            db.execute("INSERT INTO reads VALUES (1,'codex')")
            db.commit()
            original = {t: db.execute(f"SELECT * FROM {t}").fetchall() for t in
                        ("status", "claims", "messages", "reads", "settings", "events")}
        self.cli("status")  # 触发迁移，不创建 MCP 会话或清理历史认领。
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 8)
            for table, before in original.items():
                after = db.execute(f"SELECT * FROM {table}").fetchall()
                self.assertEqual([r[:len(before[0])] for r in after], before)
            self.assertEqual(db.execute("SELECT session_no FROM status").fetchall(), [(1,)])
            self.assertEqual(db.execute("SELECT session_no FROM claims").fetchall(), [(1,)])
            self.assertEqual(db.execute("SELECT pid,session_no FROM sessions").fetchall(), [(0, 1)])
        self.initialize()
        server = self.session()
        board = content(server.call("bridge_overview", project=self.project))
        self.assertIn("你的身份：codex #2", board)
        self.assertIn("旧任务", board)
        self.check(server.call("read_messages", project=self.project), "没有未读消息。")
        self.check(server.call("release_files", project=self.project), "已释放 0 个文件。")
        server.call("update_status", project=self.project, task="新任务")
        server.call("send_message", project=self.project, content="新消息")
        server.call("claim_files", project=self.project, files=["new.rs"])
        server.call("release_files", project=self.project)
        self.cli("agent", self.project, "codex", "off")
        with closing(sqlite3.connect(self.database)) as db:
            events = db.execute("SELECT kind,detail FROM events").fetchall()
            self.assertEqual([kind for kind, _ in events][-5:], ["status", "message", "claim", "release", "agent_switch"])
            statuses = [json.loads(detail) for kind, detail in events if kind == "status"]
            self.assertEqual(statuses[-1]["session_no"], 2)
            self.assertEqual(statuses[-1]["task"], "新任务")
            count = len(events)
        self.cli("status")
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM events").fetchone()[0], count)
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "agent_switch")),
                         "[<时间>] human 对 codex 关闭 Bridge\n")
