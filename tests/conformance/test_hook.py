"""Stop 钩子协议、收件窗口及旧库升级；所有子进程只使用临时库。"""
import json
import sqlite3
import subprocess
from concurrent.futures import ThreadPoolExecutor
from contextlib import closing

from .support import COMMAND, ROOT, STAMP, ContractCase


class HookTests(ContractCase):
    def setUp(self):
        super().setUp()
        (self.directory / ".git").mkdir()
        (self.directory / ".git/HEAD").write_text("ref: refs/heads/main\n", encoding="utf-8")

    def hook(self, session="worker", agent="codex", active=False, clock=STAMP, cwd=None, raw=None):
        data = raw if raw is not None else json.dumps({
            "cwd": cwd or str(self.directory), "session_id": session, "stop_hook_active": active
        }, ensure_ascii=False).encode("utf-8")
        result = subprocess.run([*COMMAND, "hook", "--agent", agent], input=data,
                                env={**self.env, "BRIDGE_FAKE_NOW": clock},
                                capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr)
        if result.stdout:
            self.assertEqual(len(result.stdout.splitlines()), 1)
            decoded = json.loads(result.stdout)
            self.assertEqual(decoded["decision"], "block")
            self.assertIsInstance(decoded["reason"], str)
        return result

    def post(self, text="审查一下", to="codex"):
        self.cli("post", self.project, text, "--to", to)

    def test_no_database_invalid_input_and_non_repository_are_fail_open(self):
        self.assertEqual(self.hook().stdout, b"")
        self.assertFalse(self.database.exists())
        for data in [b"not json", b"{}", b'{"cwd":"x","session_id":""}', b'{"cwd":2}']:
            result = self.hook(raw=data)
            self.assertEqual(result.stdout, b"")
            self.assertTrue(result.stderr)
        self.enable()
        self.post()
        (self.directory / ".git/HEAD").unlink()
        (self.directory / ".git").rmdir()
        self.assertEqual(self.hook().stdout, b"")
        self.assertEqual(self.rows("SELECT * FROM reads"), [])
        self.assertEqual(self.rows("SELECT * FROM hook_receivers"), [])

    def test_active_off_and_empty_do_not_claim_or_consume(self):
        self.enable()
        self.assertEqual(self.hook().stdout, b"")
        self.assertEqual(self.rows("SELECT * FROM hook_receivers"), [])
        self.post()
        self.assertEqual(self.hook(active=True).stdout, b"")
        for args in [("off", self.project), ("off", "--global")]:
            self.cli(*args)
            self.assertEqual(self.hook().stdout, b"")
            self.cli("on", self.project)
            self.cli("on", "--global")
        self.assertEqual(self.rows("SELECT * FROM reads"), [])
        self.assertEqual(self.rows("SELECT * FROM hook_receivers"), [])

    def test_latest_ten_only_are_marked_read_and_not_self_or_wrong_recipient(self):
        self.enable()
        sender = self.session("claude")
        for n in range(13):
            sender.call("send_message", project=self.project, to="codex", content=f"消息{n}")
        self.session().call("send_message", project=self.project, to="all", content="自己发的")
        sender.call("send_message", project=self.project, to="human", content="给别人")
        reason = json.loads(self.hook().stdout)["reason"]
        self.assertIn("还有 3 条，用 read_messages 查看", reason)
        self.assertIn("#4 [claude → codex] 消息3", reason)
        self.assertIn("#13 [claude → codex] 消息12", reason)
        self.assertNotIn("#1 [", reason)
        self.assertNotIn("自己发的", reason)
        self.assertNotIn("给别人", reason)
        self.assertEqual(self.rows("SELECT message_id FROM reads ORDER BY message_id"),
                         [(n,) for n in range(4, 14)])
        self.assertEqual(self.hook(active=True).stdout, b"")
        remaining = self.session().call("read_messages", project=self.project)["content"][0]["text"]
        self.assertIn("3 条", remaining)
        self.assertEqual(self.hook().stdout, b"")

    def test_other_window_cannot_take_messages_and_owner_heartbeat_extends_binding(self):
        self.enable()
        self.post("第一条")
        self.assertTrue(self.hook(session="worker").stdout)
        self.post("第二条")
        self.assertEqual(self.hook(session="question").stdout, b"")
        self.assertEqual(len(self.rows("SELECT * FROM reads")), 1)
        self.assertEqual(self.hook(session="worker", active=True, clock="2090-01-02 04:04:05").stdout, b"")
        self.assertEqual(self.hook(session="question", clock="2090-01-02 06:04:05").stdout, b"")
        self.assertIn("第二条", json.loads(self.hook(session="question", clock="2090-01-02 06:04:06").stdout)["reason"])
        self.assertEqual(self.rows("SELECT session_id FROM hook_receivers"), [("question",)])
        self.post("第三条")
        self.assertEqual(self.hook(session="worker", clock="2090-01-02 06:04:07").stdout, b"")

    def test_expired_binding_without_mail_stays_and_rebind_without_mail_assigns_next(self):
        self.enable()
        self.post()
        self.hook(session="old")
        self.assertEqual(self.hook(session="new", clock="2090-01-02 05:04:06").stdout, b"")
        self.assertEqual(self.rows("SELECT session_id FROM hook_receivers"), [("old",)])
        self.cli("off", self.project)
        self.assertIn("已清除 codex 的收件窗口", self.cli("rebind", self.project, "--agent", "codex"))
        self.assertEqual(self.rows("SELECT session_id,last_active,pending_rebind FROM hook_receivers"),
                         [(None, None, 1)])
        self.assertEqual(self.hook(session="new", active=True).stdout, b"")
        self.assertEqual(self.rows("SELECT session_id,pending_rebind FROM hook_receivers"), [("new", 0)])
        self.cli("on", self.project)
        self.post()
        self.assertEqual(self.hook(session="old").stdout, b"")
        self.assertTrue(self.hook(session="new").stdout)

    def test_agents_and_projects_have_independent_receivers(self):
        self.enable()
        self.post("广播", "all")
        self.assertTrue(self.hook(session="c", agent="codex").stdout)
        self.assertTrue(self.hook(session="a", agent="claude").stdout)
        self.assertEqual(self.rows("SELECT agent,session_id FROM hook_receivers ORDER BY agent"),
                         [("claude", "a"), ("codex", "c")])
        other = self.directory / "other"
        (other / ".git").mkdir(parents=True)
        (other / ".git/HEAD").write_text("ref: refs/heads/main", encoding="utf-8")
        self.cli("rebind", str(other), "--agent", "codex")
        self.hook(session="other", cwd=str(other))
        self.assertEqual(len(self.rows("SELECT * FROM hook_receivers")), 3)

    def test_concurrent_hooks_deliver_once_and_share_worktree_root(self):
        self.enable()
        self.post()
        with ThreadPoolExecutor(max_workers=4) as pool:
            results = list(pool.map(lambda i: self.hook(session=f"window{i}"), range(4)))
        self.assertEqual(sum(bool(r.stdout) for r in results), 1)
        self.assertEqual(len(self.rows("SELECT * FROM reads")), 1)
        self.assertEqual(len(self.rows("SELECT * FROM hook_receivers")), 1)
        # 同一仓库的 worktree 共用收件窗口，路径按原有仓库识别处理。
        private = self.directory / ".git/worktrees/other"
        private.mkdir(parents=True)
        (private / "commondir").write_text("../..", encoding="utf-8")
        wt = self.directory / "worktree"
        wt.mkdir()
        (wt / ".git").write_text(f"gitdir: {private}", encoding="utf-8")
        self.post("给主收件窗口")
        self.assertEqual(self.hook(session="different", cwd=str(wt)).stdout, b"")
        self.cli("rebind", str(wt), "--agent", "codex")
        self.assertTrue(self.hook(session="different", cwd=str(wt)).stdout)
        self.assertEqual(len(self.rows("SELECT * FROM hook_receivers")), 1)

    def test_corrupt_and_future_database_do_not_block_host(self):
        self.database.write_bytes(b"not sqlite")
        result = self.hook()
        self.assertEqual(result.stdout, b"")
        self.assertTrue(result.stderr)
        self.database.unlink()
        self.enable()
        with closing(sqlite3.connect(self.database)) as db:
            db.execute("PRAGMA user_version=999")
        result = self.hook()
        self.assertEqual(result.stdout, b"")
        self.assertTrue(result.stderr)

    def test_missing_home_is_fail_open_before_database_construction(self):
        env = {k: v for k, v in self.env.items() if k.upper() not in {"USERPROFILE", "HOME", "BRIDGE_DB"}}
        result = subprocess.run([*COMMAND, "hook", "--agent", "codex"], input=b"{}",
                                env=env, capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(result.stdout, b"")
        self.assertIn("无法确定用户主目录", result.stderr.decode("utf-8"))

    def test_v8_upgrade_preserves_every_existing_table_and_allows_new_messages(self):
        with closing(sqlite3.connect(self.database)) as db:
            for n in range(1, 9):
                db.executescript((ROOT / f"migrations/{n:03}.sql").read_text(encoding="utf-8"))
            db.execute("PRAGMA user_version=8")
            db.execute("INSERT INTO settings VALUES (?,1)", (self.project,))
            db.execute("INSERT INTO agent_settings VALUES (?, 'codex',0)", (self.project,))
            db.execute("INSERT INTO messages(project,sender,recipient,content,created_at) VALUES(?,?,?,?,?)",
                       (self.project, "claude", "codex", "升级前的消息", STAMP))
            db.commit()
            tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table'")]
            before = {t: db.execute(f'SELECT * FROM "{t}"').fetchall() for t in tables}
        self.cli("status")
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("PRAGMA user_version").fetchone()[0], 9)
            for table, rows in before.items():
                self.assertEqual(db.execute(f'SELECT * FROM "{table}"').fetchall(), rows)
        self.assertIn("升级前的消息", json.loads(self.hook().stdout)["reason"])
        self.post("升级后的消息")
        self.assertIn("升级后的消息", json.loads(self.hook().stdout)["reason"])
