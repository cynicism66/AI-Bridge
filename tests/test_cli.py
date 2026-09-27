"""人用命令行：使用真实业务层，输出与等待可替换。"""

import io
import os
from contextlib import redirect_stderr, redirect_stdout
from unittest.mock import patch

from tests.support import DatabaseTestCase
from bridge_mcp import cli, store, tools


class CliTests(DatabaseTestCase):
    def setUp(self):
        super().setUp()
        self.project = tools.norm_project(self.project)

    def run_cli(self, *args):
        with io.StringIO() as result, redirect_stdout(result):
            cli.main(list(args))
            return result.getvalue()

    def test_no_arguments_start_mcp_without_human_output(self):
        with patch.object(cli, "serve") as serve:
            self.assertEqual(self.run_cli(), "")
        serve.assert_called_once_with()

    def test_on_off_and_global_priority(self):
        self.assertIn("项目目录不存在", self.run_cli("on", self.project))
        self.assertTrue(store.switch_state(self.project)["enabled"])
        self.assertIn("全局已关闭", self.run_cli("off", "--global"))
        self.assertFalse(store.switch_state(self.project)["enabled"])
        self.assertTrue(store.switch_state(self.project)["project_enabled"])
        self.run_cli("on", "--global")
        self.assertTrue(store.switch_state(self.project)["enabled"])
        self.assertIn("项目目录不存在", self.run_cli("off", self.project))
        self.assertFalse(store.switch_state(self.project)["project_enabled"])

    def test_omitted_projects_use_current_directory_and_relative_paths_work(self):
        previous = os.getcwd()
        try:
            os.chdir(self.directory)
            project = tools.norm_project(str(self.directory))
            self.run_cli("on")
            self.assertTrue(store.switch_state(project)["enabled"])
            self.assertIn(project, self.run_cli("show"))
            self.assertIn("已发送给 所有人", self.run_cli("post", "当前目录留言"))
            self.assertEqual(store.recent_messages(project)[0]["sender"], "human")
            store.send_message(project, "claude", "human", "当前目录回复")
            self.assertIn("当前目录回复", self.run_cli("read"))
            with patch.object(cli.time, "sleep", side_effect=KeyboardInterrupt):
                self.assertIn(project, self.run_cli("watch"))
            self.run_cli("off")
            self.assertFalse(store.switch_state(project)["enabled"])
            self.run_cli("on", "./nested")
            self.assertTrue(store.switch_state(tools.norm_project(str(self.directory / "nested")))["enabled"])
        finally:
            os.chdir(previous)

    def test_status_reports_switches_and_latest_activity(self):
        self.assertIn("全局开关：已开启", self.run_cli("status"))
        self.run_cli("on", self.project)
        with patch.object(store, "now", return_value="2026-01-01 10:00:00"):
            store.update_status(self.project, "claude", "任务")
        store.set_enabled(False)
        text = self.run_cli("status")
        self.assertTrue(text.startswith("提示：Bridge 全局已关闭"))
        self.assertIn("项目开关 已开启，有效状态 未开启", text)
        self.assertIn("最近活动 2026-01-01 10:00:00", text)

    def test_human_commands_work_while_disabled(self):
        for global_off in (False, True):
            with self.subTest(global_off=global_off):
                if global_off:
                    store.set_enabled(False)
                text = self.run_cli("post", self.project, "人类广播")
                self.assertTrue(text.startswith("提示：Bridge"))
                store.send_message(self.project, "claude", "human", "回复人类")
                self.assertIn("回复人类", self.run_cli("read", self.project))
                view = self.run_cli("show", self.project)
                self.assertTrue(view.startswith("提示：Bridge"))
                self.assertIn("启用状态：未开启", view)
                self.assertIn("人类广播", view)

    def test_post_reaches_ai_and_read_marks_only_human_visible_messages(self):
        self.run_cli("on", self.project)
        self.run_cli("post", self.project, "请检查", "--to", "codex")
        self.assertIn("human → codex：请检查", tools.t_read_messages({"project": self.project}))
        tools.t_send_message({"project": self.project, "to": "human", "content": "已检查"})
        store.send_message(self.project, "claude", "all", "广播")
        store.send_message(self.project, "claude", "codex", "仅 AI")
        text = self.run_cli("read", self.project)
        self.assertIn("已检查", text)
        self.assertIn("广播", text)
        self.assertNotIn("仅 AI", text)
        self.assertIn("没有未读消息", self.run_cli("read", self.project))
        self.assertEqual([r["content"] for r in store.overview(self.project, "codex")["unread"]], ["广播", "仅 AI"])

    def test_watch_only_outputs_changes_and_exits_on_interrupt(self):
        calls = 0

        def tick(seconds):
            nonlocal calls
            self.assertEqual(seconds, 0.5)
            calls += 1
            if calls == 2:
                store.update_status(self.project, "claude", "发生变化")
                store.send_message(self.project, "claude", "human", "新消息")
            if calls == 3:
                raise KeyboardInterrupt

        with patch.object(cli.time, "sleep", side_effect=tick), patch.object(cli.output, "write") as write:
            cli.main(["watch", self.project, "--interval", "0.5"])
        self.assertEqual(write.call_count, 2)
        self.assertIn("发生变化", write.call_args.args[0])
        self.assertIn("新消息", write.call_args.args[0])

    def test_watch_default_interval_and_switch_changes(self):
        def tick(seconds):
            self.assertEqual(seconds, 2)
            if store.switch_state(self.project)["enabled"]:
                raise KeyboardInterrupt
            store.set_enabled(True, self.project)

        with patch.object(cli.time, "sleep", side_effect=tick), patch.object(cli.output, "write") as write:
            cli.main(["watch", self.project])
        self.assertEqual(write.call_count, 2)
        self.assertIn("启用状态：已开启", write.call_args.args[0])

    def test_invalid_arguments_are_rejected(self):
        for args in (["on", self.project, "--global"], ["post", ""], ["post", "消息", "--to", "human"],
                     *(["watch", "--interval", value] for value in ("0", "-1", "nan", "inf", "abc"))):
            with self.subTest(args=args), io.StringIO() as errors, redirect_stderr(errors):
                with self.assertRaises(SystemExit) as caught:
                    self.run_cli(*args)
                self.assertEqual(caught.exception.code, 2)
