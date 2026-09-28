import sqlite3
from contextlib import closing
from .support import ContractCase, GLOBAL_OFF, PROJECT_OFF, normalized, display_path
from .test_sessions import content


class AgentSwitchTests(ContractCase):
    def snapshot(self):
        with closing(sqlite3.connect(self.database)) as db:
            return {table: db.execute(f"SELECT * FROM {table}").fetchall() for table in
                    ("sessions", "status", "claims", "messages", "reads", "events", "settings", "agent_settings")}

    def test_disabled_agent_does_not_touch_business_and_live_reenable(self):
        self.enable()
        server = self.session()
        server.call("update_status", project=self.project, task="先前任务")
        server.call("claim_files", project=self.project, files=["held"])
        self.assertEqual(self.cli("agent", self.project, "codex", "off"), f"AI 开关：codex 未开启（{display_path(self.project)}）\n")
        before = self.snapshot()
        expected = "Bridge 在此项目中未对你（codex）开启（由用户控制）。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。"
        for tool in ("bridge_overview", "update_status", "claim_files", "release_files", "send_message", "read_messages"):
            self.check(server.call(tool, project=self.project, task="不能写", files=["x"], content="不能写"), expected)
            self.assertEqual(self.snapshot(), before)
        self.check(self.session().call("bridge_overview", project=self.project), expected)
        self.assertEqual(self.snapshot(), before)
        self.check(server.call("list_projects"), "还没有项目使用过 Bridge。")
        self.assertEqual(self.snapshot(), before)
        # 人用入口仍工作，其他 AI 默认开启。
        self.assertIn("codex：AI 开关 未开启，有效状态 未开启", self.cli("show", self.project))
        self.assertIn("你的身份：claude #1", content(self.session("claude").call("bridge_overview", project=self.project)))
        self.cli("agent", "codex", "on", cwd=self.directory)
        self.check(server.call("update_status", project=self.project, task="恢复"), "状态已更新（codex）。")
        self.assertIn("任务：恢复", content(server.call("bridge_overview", project=self.project)))
        self.assertIn("codex：AI 开关 已开启，有效状态 已开启", self.cli("status"))
        self.assertIn("claude：AI 开关 已开启，有效状态 已开启", self.cli("status"))
        self.cli("agent", self.project, "codex", "on")
        self.assertEqual(normalized(self.cli("history", self.project, "--kind", "agent_switch")),
                         "[<时间>] human 对 codex 关闭 Bridge\n[<时间>] human 对 codex 开启 Bridge\n")

    def test_gate_priority_and_configured_ai_without_sessions(self):
        self.enable()
        server = self.session()
        self.cli("agent", self.project, "codex", "off")
        self.cli("off", self.project)
        self.check(server.call("bridge_overview", project=self.project), PROJECT_OFF)
        self.cli("off", "--global")
        self.check(server.call("bridge_overview", project=self.project), GLOBAL_OFF)
        state = self.cli("status")
        self.assertIn("codex：AI 开关 未开启，有效状态 未开启", state)
        self.assertIn("AI 开关必须是 on 或 off", self.cli("agent", self.project, "codex", "bad", ok=False))
        self.assertIn("agent 不能为空", self.cli("agent", self.project, " ", "on", ok=False))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM sessions").fetchone()[0], 0)
