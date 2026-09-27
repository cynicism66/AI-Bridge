"""开关默认值、只读拦截及立即生效。"""

from contextlib import ExitStack
import sqlite3
from unittest.mock import patch

from tests.support import DatabaseTestCase
from bridge_mcp import server, store, tools


class SwitchTests(DatabaseTestCase):
    def setUp(self):
        super().setUp()
        self.project = tools.norm_project(self.project)

    def call(self, name):
        return server.handle({"method": "tools/call", "params": {
            "name": name, "arguments": {"project": self.project, "task": "任务", "content": "消息", "files": ["a.py"]},
        }})

    def assert_blocked(self, name, message):
        self.assertEqual(self.call(name), {"content": [{"type": "text", "text": message}]})

    def test_default_project_off_does_not_create_database(self):
        self.assertEqual(store.switch_state(self.project), {
            "global_enabled": True, "project_enabled": False, "enabled": False,
        })
        for name in tools.HANDLERS:
            if name != "list_projects":
                with self.subTest(name=name):
                    self.assert_blocked(name, tools.PROJECT_OFF)
        self.assertFalse(self.database.exists())

    def test_project_and_global_changes_apply_to_next_call(self):
        self.assert_blocked("update_status", tools.PROJECT_OFF)
        store.set_enabled(True, self.project)
        self.assertEqual(self.call("update_status")["content"][0]["text"], "状态已更新（codex）。")
        store.set_enabled(False)
        for name in tools.HANDLERS:
            with self.subTest(name=name):
                self.assert_blocked(name, tools.GLOBAL_OFF)
        self.assertTrue(store.switch_state(self.project)["project_enabled"])
        store.set_enabled(True)
        self.assertTrue(store.switch_state(self.project)["enabled"])
        store.set_enabled(False, self.project)
        self.assert_blocked("update_status", tools.PROJECT_OFF)

    def test_disabled_tools_only_read_settings_and_never_touch_business_data(self):
        store.update_status(self.project, "claude", "原任务")
        store.send_message(self.project, "claude", "all", "保留未读")
        with patch.object(store, "now", return_value="2000-01-01 00:00:00"):
            store.claim_files(self.project, "claude", ["a.py"], 1)
        original_connect = sqlite3.connect
        reads = set()

        def authorizer(action, table, column, database, trigger):
            if action == sqlite3.SQLITE_READ:
                reads.add(table)
                return sqlite3.SQLITE_OK if table in ("settings", "sqlite_master") else sqlite3.SQLITE_DENY
            return sqlite3.SQLITE_OK if action in (sqlite3.SQLITE_SELECT, sqlite3.SQLITE_FUNCTION) else sqlite3.SQLITE_DENY

        def connect(*args, **kwargs):
            self.assertTrue(kwargs.get("uri"))
            self.assertIn("mode=ro", args[0])
            connection = original_connect(*args, **kwargs)
            connection.set_authorizer(authorizer)
            return connection

        for global_off in (False, True):
            if global_off:
                store.set_enabled(False)
            before = self.database.read_bytes()
            with ExitStack() as stack:
                stack.enter_context(patch.object(store.sqlite3, "connect", side_effect=connect))
                for operation in ("overview", "update_status", "send_message", "read_messages", "claim_files", "release_files"):
                    stack.enter_context(patch.object(store, operation, side_effect=AssertionError("禁止业务读写")))
                if global_off:
                    stack.enter_context(patch.object(store, "list_projects", side_effect=AssertionError("禁止业务读写")))
                for name in tools.HANDLERS:
                    if global_off or name != "list_projects":
                        self.assert_blocked(name, tools.GLOBAL_OFF if global_off else tools.PROJECT_OFF)
            self.assertEqual(self.database.read_bytes(), before)
        self.assertEqual(reads, {"settings", "sqlite_master"})

    def test_existing_database_without_settings_defaults_off_without_migration(self):
        store.update_status(self.project, "claude", "历史任务")
        with store.db() as conn:
            conn.execute("DROP TABLE settings")
        before = self.database.read_bytes()
        self.assert_blocked("bridge_overview", tools.PROJECT_OFF)
        self.assertEqual(self.database.read_bytes(), before)

    def test_project_list_includes_configured_and_historical_projects(self):
        other = self.project + "-other"
        old = self.project + "-old"
        store.set_enabled(True, self.project)
        store.set_enabled(False, other)
        store.update_status(old, "claude", "历史项目")
        text = self.call("list_projects")["content"][0]["text"]
        self.assertIn(f"{self.project}（最近活动 -，已开启）", text)
        self.assertIn(f"{other}（最近活动 -，未开启）", text)
        self.assertIn(old, text)
        self.assertEqual(len(store.list_projects()), 3)

    def test_tools_do_not_expose_switches_and_describe_human_recipient(self):
        self.assertEqual(len(tools.tool_list()), 7)
        description = next(item for item in tools.tool_list() if item["name"] == "send_message")
        self.assertEqual(description["inputSchema"]["properties"]["to"]["description"],
                         "收件人：claude / codex / human / all，默认 all")
        self.assertIn("新项目默认未开启", tools.INSTRUCTIONS)
