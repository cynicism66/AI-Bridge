import sqlite3
from contextlib import closing
from .support import ContractCase, ROOT, STAMP
from .test_initialization import PENDING
from .test_sessions import content

class MigrationV4Tests(ContractCase):
    def test_v3_rows_preserved_pending_then_user_init(self):
        with closing(sqlite3.connect(self.database)) as db:
            for number in (1, 2, 3):
                db.executescript((ROOT / f"migrations/{number:03}.sql").read_text(encoding="utf-8"))
            db.execute("PRAGMA user_version=3")
            db.execute("INSERT INTO settings VALUES (?,1)", (self.project,))
            db.execute("INSERT INTO settings VALUES (?,0)", (self.project + "/off",))
            db.execute("INSERT INTO sessions VALUES ('old',?,'codex',1,?,'main',123,?,?)", (self.project, self.project, STAMP, STAMP))
            db.execute("INSERT INTO status VALUES (?,'codex','旧任务','','','',?,1,'old')", (self.project, STAMP))
            db.execute("INSERT INTO claims VALUES (?,'old.rs','codex','旧认领',?,'2100-01-01 00:00:00',1,'old')", (self.project, STAMP))
            db.execute("INSERT INTO messages VALUES (1,?,'claude','codex','旧消息',?)", (self.project, STAMP))
            db.execute("INSERT INTO reads VALUES (1,'codex')")
            db.execute("INSERT INTO agent_settings VALUES (?,'other',0)", (self.project,))
            db.commit()
            old = {t: db.execute(f"SELECT * FROM {t}").fetchall() for t in ("sessions","status","claims","messages","reads","settings","agent_settings","events")}
        self.assertIn("待初始化", self.cli("status"))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 5)
            for table, rows in old.items():
                self.assertEqual([r[:len(rows[0])] for r in db.execute(f"SELECT * FROM {table}")], rows)
            self.assertEqual(db.execute("SELECT charter_version FROM sessions").fetchall(), [(0,)])
            self.assertEqual(db.execute("SELECT count(*) FROM project_init").fetchone()[0], 0)
        server = self.session()
        for name in ("bridge_overview","update_status","claim_files","send_message"):
            self.check(server.call(name,project=self.project,task="不能写",files=["a"],content="不能写"), PENDING)
        with closing(sqlite3.connect(self.database)) as db:
            for table, rows in old.items():
                self.assertEqual([r[:len(rows[0])] for r in db.execute(f"SELECT * FROM {table}")], rows)
        self.initialize()
        self.assertIn("旧任务", content(server.call("bridge_overview", project=self.project)))
        self.check(server.call("read_messages",project=self.project), "没有未读消息。")
        self.check(server.call("release_files",project=self.project), "已释放 0 个文件。")
        self.cli("status")
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM events WHERE kind='init'").fetchone()[0], 1)
            self.assertEqual(db.execute("PRAGMA integrity_check").fetchone()[0], "ok")
