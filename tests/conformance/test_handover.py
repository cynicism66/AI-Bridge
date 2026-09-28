import os
import sqlite3
import stat
from contextlib import closing
from .support import normalized
from .transfer_support import TransferCase
from .test_sessions import content


class HandoverTests(TransferCase):
    def test_complete_handover_and_reinitialize(self):
        self.cli("on", self.project)
        self.task_init("--write-rules")
        docs = self.directory / "docs/tasks"
        docs.mkdir(parents=True)
        (docs / "T01.md").write_text("# 第一项任务\n细节", encoding="utf-8")
        (self.directory / "README.md").write_text("# 项目说明\n细节", encoding="utf-8")
        codex, claude = self.session(), self.session("claude")
        codex.call("update_status", project=self.project, task="实施中", progress="一半", blockers="问题", next_step="修复")
        claude.call("update_status", project=self.project, task="审查中")
        codex.call("claim_files", project=self.project, files=["src/owned.rs"])
        self.assertIn("已认领 1", content(claude.call("claim_files", project=self.project, files=["AGENTS.md"])))
        self.assertIn("超出", content(claude.call("claim_files", project=self.project, files=["src/no.rs"])))
        # Message reads remain unchanged; only unread originals are forwarded.
        self.cli("post", self.project, "already-read", "--to", "codex")
        codex.call("read_messages", project=self.project)
        self.cli("post", self.project, "unread-direct password=message-secret", "--to", "codex")
        self.cli("post", self.project, "unread-all")
        claude.call("send_message", project=self.project, to="human", content="human-only-question")
        before = self.snapshot()
        result, _, preview = self.transfer("handover", self.project, "--to", "claude", "--yes")
        self.assertIn("交接完成", result)
        doc = (self.directory / "docs/HANDOVER.md").read_text(encoding="utf-8")
        for text in ("项目中文目标", "模板：任务书流程", "章程版本：v1", "各方职务", "实施中", "审查中", "未完成的事项", "src/owned.rs", "AGENTS.md", "codex 的未读消息", "human-only-question", "docs/tasks/T01.md：第一项任务", "README.md：项目说明", "最近 50 条历史事件", "## 风险", "## 建议的下一步", "（由接手方补全）"):
            self.assertIn(text, doc)
        self.assertNotIn("message-secret", doc + preview)
        after = self.snapshot()
        self.assertEqual(after["reads"], before["reads"])
        self.assertEqual(after["messages"][:len(before["messages"])], before["messages"])
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT template,version FROM project_init").fetchone(), ("独立开发", 2))
            self.assertEqual(db.execute("SELECT agent,slot FROM role_assignments").fetchall(), [("claude", "独立开发")])
            self.assertEqual(db.execute("SELECT agent,path FROM claims").fetchall(), [("claude", "agents.md")])
            new = db.execute("SELECT sender,recipient,content FROM messages WHERE id>?", (len(before["messages"]),)).fetchall()
            self.assertEqual(len(new), 3)
            self.assertTrue(all(row[:2] == ("bridge", "claude") for row in new))
            self.assertIn("human → codex", new[0][2])
            self.assertIn("unread-direct", new[0][2])
            self.assertIn("human → all", new[1][2])
            self.assertIn("请阅读 docs/HANDOVER.md", new[2][2])
        for name in ("AGENTS.md", "CLAUDE.md"):
            self.assertIn("claude 全盘负责规划、实现、自查和提交", (self.directory / name).read_text(encoding="utf-8"))
        self.assertIn("未对你（codex）开启", content(codex.call("bridge_overview", project=self.project)))
        board = content(claude.call("bridge_overview", project=self.project))
        self.assertIn("== 协作章程 v2 ==", board)
        self.assertIn("职务：独立开发", board)
        self.assertIn("已认领", content(claude.call("claim_files", project=self.project, files=["src/new.rs"])))
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "handover")),
                         "[<时间>] human 交接给 claude：独立开发\n")
        self.task_init()
        board = content(codex.call("bridge_overview", project=self.project))
        self.assertIn("== 协作章程 v3 ==", board)
        self.assertIn("职务：执行", board)
        self.assertIn("已认领", content(codex.call("claim_files", project=self.project, files=["src/back.rs"])))
        self.assertIn("codex：AI 开关 已开启", self.cli("status"))

    def test_solo_direct_initialization_and_cancel(self):
        self.cli("on", self.project)
        self.cli("init", self.project, "--template", "独立开发", "--role", "独立开发=codex", "--goal", "独立目标")
        board = content(self.session().call("bridge_overview", project=self.project))
        self.assertIn("职务：独立开发", board)
        self.assertIn("请阅读 docs/HANDOVER.md", board)
        before = self.snapshot()
        self.transfer("handover", self.project, "--to", "claude", answer="n\n")
        self.assertEqual(before, self.snapshot())
        self.assertFalse((self.directory / "docs").exists())
        self.transfer("handover", self.project, "--to", "human", "--yes", ok=False)
        self.assertEqual(before, self.snapshot())

    def test_failure_rolls_back_files_and_database(self):
        self.cli("on", self.project)
        self.task_init("--write-rules")
        # A Windows read-only second rules file allows preview reads but rejects replacement.
        # HANDOVER and AGENTS have already been replaced when CLAUDE fails.
        target = self.directory / "CLAUDE.md"
        originals = {name: (self.directory / name).read_bytes() for name in ("AGENTS.md", "CLAUDE.md")}
        if os.name != "nt":
            self.skipTest("Windows read-only replacement semantics")
        target.chmod(stat.S_IREAD)
        try:
            for existing in (False, True):
                document = self.directory / "docs/HANDOVER.md"
                if existing:
                    document.parent.mkdir(exist_ok=True)
                    document.write_bytes(b"original handover")
                before = self.snapshot()
                _, error, _ = self.transfer("handover", self.project, "--to", "claude", "--yes", ok=False)
                self.assertIn("原子替换失败", error)
                self.assertEqual(before, self.snapshot())
                for name, data in originals.items():
                    self.assertEqual((self.directory / name).read_bytes(), data)
                if existing:
                    self.assertEqual(document.read_bytes(), b"original handover")
                else:
                    self.assertFalse(document.parent.exists())
                self.assertFalse(list(self.directory.rglob(".bridge-*.tmp")))
        finally:
            target.chmod(stat.S_IWRITE | stat.S_IREAD)

    def test_rules_file_failure_during_init_preserves_both_files(self):
        if os.name != "nt":
            self.skipTest("Windows read-only replacement semantics")
        self.enable()
        agents, claude = self.directory / "AGENTS.md", self.directory / "CLAUDE.md"
        agents.write_bytes(b"original agents")
        claude.write_bytes(b"original claude")
        claude.chmod(stat.S_IREAD)
        try:
            before = self.snapshot()
            result = self.cli("init", self.project, "--template", "任务书流程", "--role", "规划审查=claude",
                              "--role", "执行=codex", "--goal", "new", "--write-rules", ok=False)
            self.assertIn("写入章程失败，已恢复原规则文件", result)
            self.assertEqual(self.snapshot(), before)
            self.assertEqual(agents.read_bytes(), b"original agents")
            self.assertEqual(claude.read_bytes(), b"original claude")
            self.assertFalse(list(self.directory.rglob(".bridge-*.tmp")))
        finally:
            claude.chmod(stat.S_IWRITE | stat.S_IREAD)
