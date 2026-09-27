"""存储隔离、事务生命周期与消息可见性。"""

import os
from pathlib import Path
import sqlite3
from unittest.mock import MagicMock, patch

from tests.support import DatabaseTestCase
from bridge_mcp import store


class StoreTests(DatabaseTestCase):
    def test_connection_commits_and_closes(self):
        with store.db() as conn:
            conn.execute("INSERT INTO status (project, agent, task) VALUES (?, ?, ?)",
                         (self.project, "claude", "已提交"))
        with self.assertRaises(sqlite3.ProgrammingError):
            conn.execute("SELECT 1")
        rows = store.overview(self.project, "codex")["statuses"]
        self.assertEqual(rows[0]["task"], "已提交")

    def test_connection_rolls_back_and_closes_on_error(self):
        with self.assertRaisesRegex(RuntimeError, "回滚"):
            with store.db() as conn:
                conn.execute("INSERT INTO status (project, agent) VALUES (?, ?)",
                             (self.project, "claude"))
                raise RuntimeError("回滚")
        with self.assertRaises(sqlite3.ProgrammingError):
            conn.execute("SELECT 1")
        self.assertEqual(store.overview(self.project, "codex")["statuses"], [])

    def test_connection_closes_when_setup_or_transaction_exit_fails(self):
        for stage in ("setup", "exit"):
            with self.subTest(stage=stage):
                connection = MagicMock()
                if stage == "setup":
                    connection.execute.side_effect = sqlite3.OperationalError("建表失败")
                else:
                    connection.__exit__.side_effect = sqlite3.OperationalError("提交失败")
                with patch.object(store.sqlite3, "connect", return_value=connection):
                    with self.assertRaises(sqlite3.OperationalError):
                        with store.db():
                            pass
                connection.close.assert_called_once_with()

    def test_database_environment_is_resolved_per_connection(self):
        store.update_status(self.project, "claude", task="原数据库")
        other = self.directory / "other.db"
        with patch.dict(os.environ, {"BRIDGE_DB": str(other)}):
            self.assertEqual(store.db_path(), other)
            self.assertEqual(store.list_projects(), [])
            store.update_status(self.project, "claude", task="新数据库")
        self.assertEqual(store.db_path(), self.database)
        self.assertEqual(store.overview(self.project, "codex")["statuses"][0]["task"], "原数据库")

    def test_default_path_uses_home_and_creates_directory(self):
        fake_home = self.directory / "user"
        with patch.dict(os.environ, {"BRIDGE_DB": ""}), patch.object(Path, "home", return_value=fake_home):
            self.assertEqual(store.db_path(), fake_home / ".bridge" / "bridge.db")
            self.assertTrue((fake_home / ".bridge").is_dir())

    def test_status_replaces_previous_fields_and_timestamp(self):
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            store.update_status(self.project, "claude", "旧任务", "旧进度", "旧卡点", "旧下一步")
        with patch.object(store, "now", return_value="2026-01-01 10:00:00"):
            store.update_status(self.project, "claude", "新任务")
        rows = store.overview(self.project, "codex")["statuses"]
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0], {
            "project": self.project, "agent": "claude", "task": "新任务", "progress": "",
            "blockers": "", "next_step": "", "updated_at": "2026-01-01 10:00:00",
        })

    def test_messages_visible_only_to_recipient_or_all_and_not_sender(self):
        for to, content in [("codex", "定向"), ("all", "广播"), ("human", "人类专用")]:
            store.send_message(self.project, "claude", to, content)
        self.assertEqual(store.overview(self.project, "claude")["unread"], [])
        self.assertEqual([r["content"] for r in store.overview(self.project, "codex")["unread"]],
                         ["定向", "广播"])
        self.assertEqual([r["content"] for r in store.overview(self.project, "human")["unread"]],
                         ["广播", "人类专用"])

    def test_read_receipts_are_separate_for_each_identity(self):
        store.send_message(self.project, "claude", "all", "广播")
        self.assertEqual(len(store.read_messages(self.project, "codex")), 1)
        self.assertEqual(store.read_messages(self.project, "codex"), [])
        self.assertEqual(len(store.overview(self.project, "human")["unread"]), 1)
        store.overview(self.project, "human", mark_read=True)
        self.assertEqual(store.read_messages(self.project, "human"), [])

    def test_include_read_returns_latest_limit_in_ascending_order(self):
        for index in range(5):
            store.send_message(self.project, "claude", "codex", str(index))
        store.send_message(self.project, "human", "claude", "不可见")
        rows = store.read_messages(self.project, "codex", limit=2, include_read=True)
        self.assertEqual([r["content"] for r in rows], ["3", "4"])
        self.assertEqual([r["id"] for r in rows], sorted(r["id"] for r in rows))
        self.assertEqual(len(store.read_messages(self.project, "codex")), 5)
        self.assertEqual(store.read_messages(self.project, "codex", 2, True), rows)
        sent = store.read_messages(self.project, "claude", 20, True)
        self.assertEqual(len(sent), 6)

    def test_unread_limit_marks_only_returned_messages(self):
        for index in range(3):
            store.send_message(self.project, "claude", "codex", str(index))
        self.assertEqual([r["content"] for r in store.read_messages(self.project, "codex", 1)], ["0"])
        self.assertEqual([r["content"] for r in store.read_messages(self.project, "codex")], ["1", "2"])

    def test_projects_isolate_status_messages_and_claims(self):
        other = str(self.directory / "另一个项目")
        for project, name in [(self.project, "甲"), (other, "乙")]:
            store.update_status(project, "claude", name)
            store.send_message(project, "claude", "codex", name)
            store.claim_files(project, "claude", ["same.py"], 60, name)
        first = store.overview(self.project, "codex", mark_read=True)
        second = store.overview(other, "codex")
        for data, name in [(first, "甲"), (second, "乙")]:
            self.assertEqual([r["task"] for r in data["statuses"]], [name])
            self.assertEqual([r["content"] for r in data["unread"]], [name])
            self.assertEqual([r["note"] for r in data["claims"]], [name])
        self.assertEqual(store.release_files(self.project, "claude", []), {"count": 1})
        self.assertEqual(len(store.overview(other, "codex")["claims"]), 1)

    def test_list_projects_orders_latest_activity_across_all_tables(self):
        names = [str(self.directory / name) for name in ("status", "message", "claim")]
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            store.update_status(names[0], "claude", "任务")
        with patch.object(store, "now", return_value="2026-01-01 10:00:00"):
            store.send_message(names[1], "claude", "all", "消息")
        with patch.object(store, "now", return_value="2026-01-01 11:00:00"):
            store.claim_files(names[2], "claude", ["a.py"], 60)
            store.send_message(names[0], "claude", "all", "更新")
        with patch.object(store, "now", return_value="2026-01-01 12:00:00"):
            store.update_status(names[0], "codex", "最新")
        self.assertEqual([r["project"] for r in store.list_projects()], [names[0], names[2], names[1]])
