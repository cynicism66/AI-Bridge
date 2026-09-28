import json
import sqlite3
from contextlib import closing
from .support import ContractCase, ROOT, normalized
from .test_sessions import content

PENDING = "此项目的 Bridge 协作尚未初始化（由用户控制）。请提醒用户完成初始化（分配职务和权限），在此之前正常工作，不必再调用 Bridge 工具。"
START = "<!-- AI Bridge 章程开始（由 bridge 生成，请勿手动修改此区块） -->".encode()
END = "<!-- AI Bridge 章程结束 -->".encode()

class InitializationTests(ContractCase):
    def init_args(self, *extra):
        return ("init", self.project, "--template", "任务书流程", "--role", "规划审查=claude",
                "--role", "执行=codex", "--goal", "目标中文", *extra)

    def snapshot(self):
        with closing(sqlite3.connect(self.database)) as db:
            tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
            return {t: db.execute(f'SELECT * FROM "{t}"').fetchall() for t in tables}

    def test_pending_gate_no_business_write_then_live_init(self):
        self.cli("on", self.project)
        before = self.snapshot()
        s = self.session()
        for tool in ("bridge_overview", "update_status", "send_message", "read_messages", "claim_files", "release_files"):
            self.check(s.call(tool, project=self.project, task="不写入", content="不写入", files=["a"]), PENDING)
            self.assertEqual(self.snapshot(), before)
        self.assertIn(self.project, content(s.call("list_projects")))
        self.assertEqual(self.snapshot(), before)
        self.assertIn("待初始化", self.cli("status"))
        self.assertIn("待初始化", self.cli("show", self.project))
        self.cli(*self.init_args("--no-kickoff"))
        self.check(s.call("update_status", project=self.project, task="生效"), "状态已更新（codex）。")
        self.assertIn("任务：生效", content(s.call("bridge_overview", project=self.project)))
        self.cli("off", self.project)
        self.cli("on", self.project)
        self.assertIn("协作中", self.cli("status"))

    def test_invalid_init_is_atomic(self):
        self.assertIn("先执行 on", self.cli(*self.init_args(), ok=False))
        self.assertFalse(self.database.exists())
        self.cli("on", self.project)
        before = self.snapshot()
        common = ("init", self.project, "--template", "任务书流程", "--goal", "目标")
        cases = [(("--role", "执行=codex"), "缺少职务位置：规划审查"),
                 (("--role", "执行=codex", "--role", "规划审查=codex"), "一个 agent 只能担任一个职务位置"),
                 (("--role", "不存在=codex"), "没有职务位置"),
                 (("--role", "执行=human"), "保留的 AI 名称")]
        for extra, expected in cases:
            self.assertIn(expected, self.cli(*common, *extra, ok=False))
            self.assertEqual(self.snapshot(), before)
        self.assertIn("尚未初始化", self.cli("role", self.project, "codex", "执行", ok=False))

    def test_charter_cache_reinit_and_independent_sessions(self):
        self.cli("on", self.project)
        self.cli(*self.init_args("--no-kickoff"))
        s, other = self.session(), self.session()
        first = content(s.call("bridge_overview", project=self.project))
        for expected in ("== 协作章程 v1 ==", "项目目标：目标中文", "claude 负责规划和审查，codex 负责实现", "== 你的职务和权限 ==", "职务：执行", "禁止：修改任务书里的审查意见", "== 各方职务 ==", "codex：执行 ← 你", "不得使用 `git add -A`"):
            self.assertIn(expected, first)
        second = content(s.call("bridge_overview", project=self.project))
        self.assertIn("协作章程 v1（本会话已显示过，未变化）", second)
        self.assertNotIn("== 你的职务和权限 ==", second)
        self.assertIn("== 协作章程 v1 ==", content(other.call("bridge_overview", project=self.project)))
        self.cli(*self.init_args("--no-kickoff"))
        self.assertIn("== 协作章程 v2 ==", content(s.call("bridge_overview", project=self.project)))
        self.assertIn("未变化", content(s.call("bridge_overview", project=self.project)))
        for text in (self.cli("status"), self.cli("show", self.project)):
            for expected in ("协作中", "模板：任务书流程", "项目目标：目标中文", "claude：职务 规划审查", "codex：职务 执行"):
                self.assertIn(expected, text)

    def test_write_permission_batch_and_unassigned_agent(self):
        self.cli("on", self.project)
        self.cli(*self.init_args("--no-kickoff"))
        s = self.session("claude")
        self.check(s.call("claim_files", project=self.project, files=["docs/good.md", "src/bad.rs"]),
                   "认领失败：以下文件超出你（claude，职务：规划审查）的可写范围：src/bad.rs。你的可写范围：docs/**。如确需修改，请先用 send_message 和规划方或用户协商。")
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM claims").fetchone()[0], 0)
            self.assertEqual(db.execute("SELECT count(*) FROM events WHERE kind='claim'").fetchone()[0], 0)
        self.assertIn("已认领 1", content(s.call("claim_files", project=self.project, files=[self.project + "/docs/ok.md"])))
        for name in ("../escape.md", self.project + "/../escape.md"):
            self.assertIn("超出", content(s.call("claim_files", project=self.project, files=[name])))
        unknown = self.session("unknown")
        self.assertIn("职务：未分配", content(unknown.call("claim_files", project=self.project, files=["a"])))

    def test_kickoff_messages_no_kickoff_and_history(self):
        self.cli("on", self.project)
        self.cli(*self.init_args())
        with closing(sqlite3.connect(self.database)) as db:
            rows = db.execute("SELECT sender,recipient,content FROM messages ORDER BY id").fetchall()
            self.assertEqual(len(rows), 2)
            self.assertEqual([(r[0],r[1]) for r in rows], [("bridge","claude"),("bridge","codex")])
            self.assertIn("起草 docs/PLAN.md", rows[0][2])
            self.assertIn("请等待 claude", rows[1][2])
        history = normalized(self.cli("history", self.project, "--kind", "message"))
        self.assertIn("bridge → claude 留言：", history)
        self.assertIn("bridge → codex 留言：", history)
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "init")),
                         "[<时间>] human 初始化协作：任务书流程，claude=规划审查，codex=执行\n")
        self.cli(*self.init_args("--no-kickoff"))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM messages").fetchone()[0], 2)
            self.assertEqual(db.execute("SELECT count(*) FROM events WHERE kind='init'").fetchone()[0], 2)

    def test_role_swaps_and_refreshes_permissions_and_charter(self):
        self.cli("on", self.project)
        self.cli(*self.init_args("--no-kickoff"))
        s = self.session()
        s.call("bridge_overview", project=self.project)
        self.assertIn("codex → 规划审查，claude → 执行", self.cli("role", "codex", "规划审查", cwd=self.directory))
        board = content(s.call("bridge_overview", project=self.project))
        self.assertIn("== 协作章程 v2 ==", board)
        self.assertIn("codex 负责规划和审查，claude 负责实现", board)
        self.assertIn("职务：规划审查", board)
        self.assertIn("超出", content(s.call("claim_files", project=self.project, files=["src/x.rs"])))
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "role")),
                         "[<时间>] human 把 codex 的职务改为 规划审查\n[<时间>] human 把 claude 的职务改为 执行\n")
        before = self.snapshot()
        self.assertIn("未变更", self.cli("role", self.project, "codex", "规划审查"))
        self.assertEqual(self.snapshot(), before)
        for agent, slot, expected in (("codex","未知","没有职务位置"),("other","执行","未参与")):
            self.assertIn(expected, self.cli("role", self.project, agent, slot, ok=False))
            self.assertEqual(self.snapshot(), before)

    def test_write_rules_preserves_raw_bytes_and_validates_both_first(self):
        self.cli("on", self.project)
        prefix, suffix = b"\xef\xbb\xbf# user\r\n\xff\r\n", b"\r\noriginal tail\r\n"
        old = prefix + START + b"old" + END + suffix
        agents, claude = self.directory / "AGENTS.md", self.directory / "CLAUDE.md"
        agents.write_bytes(old)
        claude.write_bytes(START + b"broken")
        before = self.snapshot()
        self.assertIn("标记", self.cli(*self.init_args("--write-rules", "--no-kickoff"), ok=False))
        self.assertEqual(agents.read_bytes(), old)
        self.assertEqual(self.snapshot(), before)
        claude.unlink()
        self.cli(*self.init_args("--write-rules", "--no-kickoff"))
        result = agents.read_bytes()
        self.assertTrue(result.startswith(prefix + START))
        self.assertTrue(result.endswith(END + suffix))
        self.assertIn("公共协作规则".encode(), result)
        first_claude = claude.read_bytes()
        self.cli(*self.init_args("--write-rules", "--no-kickoff"))
        self.assertEqual(agents.read_bytes(), result)
        self.assertEqual(claude.read_bytes(), first_claude)
        self.cli("role", self.project, "codex", "规划审查")
        self.assertTrue(agents.read_bytes().startswith(prefix + START))
        self.assertTrue(agents.read_bytes().endswith(END + suffix))
        self.assertIn("codex 负责规划和审查，claude 负责实现".encode(), agents.read_bytes())
        before = self.snapshot()
        saved = agents.read_bytes()
        claude.write_bytes(START + b"broken")
        self.assertIn("标记", self.cli("role", self.project, "codex", "执行", ok=False))
        self.assertEqual(self.snapshot(), before)
        self.assertEqual(agents.read_bytes(), saved)

    def test_custom_template_validation_and_write_patterns(self):
        self.cli("on", self.project)
        template = self.directory / "custom.toml"
        source = (ROOT / "templates/task.toml").read_text(encoding="utf-8").replace('write = []', 'write = ["src/*.rs", "*.md", "Cargo.toml"]')
        template.write_text(source, encoding="utf-8")
        args = list(self.init_args("--no-kickoff", "--template-file", str(template)))
        args[3] = "自定义"
        self.cli(*args)
        s = self.session()
        for name in ("src/a.rs", "deep/a.md", "Cargo.toml"):
            self.assertIn("已认领", content(s.call("claim_files", project=self.project, files=[name])))
        for name in ("src/deep/a.rs", "other.rs", "../a.md"):
            self.assertIn("超出", content(s.call("claim_files", project=self.project, files=[name])))
        before = self.snapshot()
        template.write_text(source.replace('"src/*.rs"','"src/**/a.rs"'), encoding="utf-8")
        self.assertIn("不支持的 write", self.cli(*args, ok=False))
        self.assertEqual(self.snapshot(), before)
        missing = list(self.init_args("--no-kickoff")); missing[3] = "自定义"
        self.assertIn("--template-file", self.cli(*missing, ok=False))
