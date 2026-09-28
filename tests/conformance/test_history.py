import json
import sqlite3

from .support import ContractCase, ROOT, normalized


class HistoryTests(ContractCase):
    def test_events_filters_renewal_release_and_expiry(self):
        self.enable()
        server = self.session()
        server.call("update_status", project=self.project, task="任务", progress="进度", blockers="卡点", next_step="下一步")
        server.call("update_status", project=self.project, task="新任务")
        server.call("send_message", project=self.project, content="你好")
        for note in ("重构", "续期"):
            server.call("claim_files", project=self.project, files=["src/a.py"], note=note)
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "claim")),
                         "[<时间>] codex 认领：src/a.py（重构），到期 <时间>\n"
                         "[<时间>] codex 认领：src/a.py（续期），到期 <时间>\n")
        self.assertEqual(self.cli("history", self.project, "--kind", "release"), "\n")
        server.call("release_files", project=self.project)
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "release")),
                         "[<时间>] codex 释放认领：src/a.py\n")
        past = self.session(clock="2000-01-01 00:00:00")
        past.call("claim_files", project=self.project, files=["expired.py"])
        server.call("bridge_overview", project=self.project)
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "expire")),
                         "[<时间>] codex 的认领已到期：expired.py\n")
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "status")),
                         "[<时间>] codex 更新状态：任务 任务｜进度 进度｜卡点 卡点｜下一步 下一步\n"
                         "[<时间>] codex 更新状态：任务 新任务\n")
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "message")),
                         "[<时间>] codex → 所有人 留言：你好\n")
        self.cli("off", self.project)
        before = self.cli("history", self.project)
        for tool in ("bridge_overview", "update_status", "send_message", "claim_files", "release_files", "read_messages"):
            server.call(tool, project=self.project)
        self.assertEqual(self.cli("history", self.project), before)
        self.cli("off", "--global")
        self.cli("on", "--global")
        self.assertEqual(normalized(self.cli("history", self.project, "--agent", "human", "--kind", "switch")),
                         "[<时间>] human 开启项目\n[<时间>] human 关闭项目\n"
                         "[<时间>] human 关闭全局总开关\n[<时间>] human 开启全局总开关\n")
        all_lines = self.cli("history", self.project).splitlines()
        self.assertEqual(all_lines, sorted(all_lines, key=lambda line: line[:21]))
        self.assertEqual(self.cli("history", self.project, "--limit", "2").splitlines(), all_lines[-2:])
        self.assertEqual(self.cli("history", self.project, "--limit", "0"), "\n")
        self.assertIn("limit 不能小于 0", self.cli("history", self.project, "--limit", "-1", ok=False))
        self.assertEqual(self.cli("history", self.project, "--agent", "claude"), "\n")
        self.assertEqual(self.cli("history", self.project, "--agent", "human", "--kind", "message"), "\n")

    def test_recent_fifty_and_project_isolation(self):
        self.enable()
        server = self.session()
        for index in range(55):
            server.call("send_message", project=self.project, content=f"消息{index}")
        lines = self.cli("history", self.project).splitlines()
        self.assertEqual(len(lines), 50)
        self.assertTrue(lines[0].endswith("消息5"))
        self.assertTrue(lines[-1].endswith("消息54"))
        self.assertEqual(self.cli("history", self.project + "/elsewhere"), "\n")

    def test_legacy_v0_and_v1_upgrade_preserves_data_and_backfills_once(self):
        for version in (0, 1):
            with self.subTest(version=version):
                self.database = self.directory / f"legacy-{version}.db"
                self.env["BRIDGE_DB"] = str(self.database)
                connection = sqlite3.connect(self.database)
                try:
                    connection.executescript((ROOT / "migrations/001.sql").read_text(encoding="utf-8"))
                    connection.execute(f"PRAGMA user_version={version}")
                    connection.execute("INSERT INTO settings VALUES (?, 1)", (self.project,))
                    for content in ("旧消息", "旧消息"):
                        connection.execute("INSERT INTO messages VALUES (NULL, ?, 'claude', 'all', ?, '2020-01-01 00:00:00')",
                                           (self.project, content))
                    connection.execute("INSERT INTO status VALUES (?, 'claude', '旧状态', '', '', '', '2020-01-01 00:00:00')",
                                       (self.project,))
                    connection.execute("INSERT INTO claims VALUES (?, 'a.py', 'claude', '', '2020-01-01 00:00:00', '2100-01-01 00:00:00')",
                                       (self.project,))
                    connection.execute("INSERT INTO reads VALUES (1, 'codex')")
                    connection.commit()
                    snapshot = {table: connection.execute(f"SELECT * FROM {table}").fetchall()
                                for table in ("status", "messages", "reads", "claims", "settings")}
                finally:
                    connection.close()
                text = normalized(self.cli("history", self.project))
                self.assertEqual(text, "[<时间>] claude → 所有人 留言：旧消息\n" * 2
                                 + "[<时间>] claude 更新状态：任务 旧状态\n")
                self.assertEqual(normalized(self.cli("history", self.project)), text)
                connection = sqlite3.connect(self.database)
                try:
                    self.assertEqual(connection.execute("PRAGMA user_version").fetchone()[0], 8)
                    for table, expected in snapshot.items():
                        actual = connection.execute(f"SELECT * FROM {table}").fetchall()
                        self.assertEqual([row[:len(expected[0])] for row in actual], expected)
                    self.assertEqual(connection.execute("SELECT count(*) FROM events").fetchone()[0], 3)
                    self.assertEqual(json.loads(connection.execute("SELECT json_object('ok', 1)").fetchone()[0]), {"ok": 1})
                finally:
                    connection.close()
                self.initialize()
                server = self.session()
                self.check(server.call("read_messages", project=self.project),
                           "未读消息（1 条，已标为已读）：\n  #2 [<时间>] claude → 所有人：旧消息")
                self.check(server.call("send_message", project=self.project, content="升级后"), "消息 #3 已发送给 所有人。")

    def test_future_version_rejected_in_already_running_server(self):
        self.enable()
        server = self.session()
        self.check(server.call("update_status", project=self.project, task="原任务"), "状态已更新（codex）。")
        connection = sqlite3.connect(self.database)
        try:
            connection.execute("PRAGMA user_version=99")
        finally:
            connection.close()
        error = "数据库版本 99 高于当前支持的版本 8，请升级 Bridge 后再写入。"
        self.check(server.call("update_status", project=self.project, task="不能覆盖"), "出错：" + error, True)
        self.assertIn(error, self.cli("on", self.project, ok=False))
