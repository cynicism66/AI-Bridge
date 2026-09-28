import sqlite3
from contextlib import closing
from .support import STAMP
from .transfer_support import TransferCase
from .test_sessions import content


class ExportTests(TransferCase):
    def test_export_preview_redaction_remote_and_confirmation(self):
        self.enable()
        server = self.session()
        server.call("update_status", project=self.project, task="实现导出", progress="password=private-value",
                    blockers="待检查", next_step="继续测试")
        server.call("claim_files", project=self.project, files=["src/main.rs"], note="保留认领")
        server.call("send_message", project=self.project, to="claude", content="Bearer hidden-bearer ghp_abcdef123456")
        git = self.directory / ".git"
        git.mkdir()
        (git / "config").write_text('[remote "origin"]\nurl=https://alice:remote-secret@example.com/a.git\n', encoding="utf-8")
        before = self.snapshot()
        destination = self.directory / "docs/bridge/协作记录.md"
        for answer in ("n\n", "", "Y\n"):
            stdout, _, preview = self.transfer("export", self.project, answer=answer)
            self.assertIn("确认写入？(y/N)", stdout)
            self.assertIn("已取消", stdout)
            self.assertIn("https://[已打码：URL凭据]@example.com/a.git", stdout)
            self.assertIn("提交并推送后内容可能公开", stdout)
            self.assertIn("共打码", stdout)
            self.assertFalse(destination.exists())
            self.assertFalse(destination.parent.exists())
            self.assertEqual(self.snapshot(), before)
            for secret in ("private-value", "hidden-bearer", "ghp_abcdef123456", "remote-secret"):
                self.assertNotIn(secret, stdout + preview)
        stdout, _, preview = self.transfer("export", self.project, answer="y\n")
        self.assertIn("导出完成", stdout)
        exported = destination.read_text(encoding="utf-8")
        for value in ("导出时间：" + STAMP, "章程版本：v1", "各方职务", "各会话最新状态", "实现导出", "待检查", "继续测试", "src/main.rs", "最近 200 条消息", "最近 500 条历史事件", "[已打码："):
            self.assertIn(value, exported)
        self.assertIn(exported, preview)
        self.assertEqual(self.snapshot(), before)
        stdout, _, _ = self.transfer("export", "--out", "导出.md", "--yes", cwd=self.directory)
        self.assertIn("此仓库有远程地址", stdout)
        self.assertNotIn("确认写入？", stdout)
        self.assertTrue((self.directory / "导出.md").exists())
        tools = server.request("tools/list")["result"]["tools"]
        self.assertFalse({"export", "handover"} & {t["name"] for t in tools})

    def test_export_limits_and_path_validation(self):
        self.enable()
        with closing(sqlite3.connect(self.database)) as db:
            db.executemany("INSERT INTO messages(project,sender,recipient,content,created_at) VALUES (?,'human','all',?,?)",
                           [(self.project, f"limit-message-{i:03d}", STAMP) for i in range(210)])
            db.executemany("INSERT INTO events(project,agent,kind,detail,created_at) VALUES (?,'human','message',?,?)",
                           [(self.project, '{"recipient":"all","content":"limit-event-%03d"}' % i, STAMP) for i in range(510)])
            db.commit()
        self.transfer("export", self.project, "--yes")
        text = (self.directory / "docs/bridge/协作记录.md").read_text(encoding="utf-8")
        self.assertNotIn("limit-message-009", text)
        self.assertIn("limit-message-010", text)
        self.assertNotIn("limit-event-009", text)
        self.assertIn("limit-event-010", text)
        before = self.snapshot()
        for path in ("../escape.md", ".git/config", "bridge.db", "BRIDGE.DB", str(self.directory.parent / "escape.md")):
            self.transfer("export", self.project, "--out", path, "--yes", ok=False)
            self.assertEqual(self.snapshot(), before)
