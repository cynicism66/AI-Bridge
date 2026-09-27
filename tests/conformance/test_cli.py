from concurrent.futures import ThreadPoolExecutor
import os
import threading
import unittest

from .support import COMMAND, PYTHON, ContractCase, normalized


class CliTests(ContractCase):
    def test_cli_output_and_default_directory(self):
        off = "提示：Bridge 未在此项目开启；人用命令仍可使用。\n"
        glob = "提示：Bridge 全局已关闭；人用命令仍可使用。\n"
        self.assertEqual(self.cli("status"), "全局开关：已开启\n  （还没有项目）\n")
        self.assertEqual(self.cli("on", cwd=self.directory), f"项目开关：已开启（{self.project}）\n")
        self.assertEqual(self.cli("status"), "全局开关：已开启\n"
                         f"  {self.project}：项目开关 已开启，有效状态 已开启，最近活动 -\n")
        self.assertEqual(self.cli("show", cwd=self.directory), f"启用状态：已开启\n项目：{self.project}"
                         "\n你的身份：human\n\n== 各方状态 ==\n  （还没有人汇报状态）\n\n== 文件认领 =="
                         "\n  （没有文件被认领）\n\n== 给你的未读消息（0 条）==\n  （没有未读消息）"
                         "\n\n== 最近消息 ==\n  （无）\n")
        self.assertEqual(self.cli("off", ".", cwd=self.directory), off + f"项目开关：未开启（{self.project}）\n")
        self.assertEqual(self.cli("post", "广播", cwd=self.directory), off + "消息 #1 已发送给 所有人。\n")
        self.assertEqual(self.cli("off", "--global"), glob + "全局开关：已关闭\n")
        self.assertEqual(self.cli("read", cwd=self.directory), glob + "没有未读消息。\n")
        self.assertEqual(self.cli("on", "--global"), "全局开关：已开启\n")
        missing = self.project + "/missing"
        self.assertEqual(self.cli("on", missing), "提示：项目目录不存在，已按指定路径保存开关。\n"
                         f"项目开关：已开启（{missing}）\n")
        self.assertIn("--global 不能同时指定项目", self.cli("on", self.project, "--global", ok=False))
        self.assertIn("消息内容不能为空", self.cli("post", self.project, " ", ok=False))

    def test_four_process_first_initialization(self):
        barrier = threading.Barrier(4)

        def start(index):
            barrier.wait(timeout=20)
            return self.cli("on", self.project + f"/project-{index}")

        with ThreadPoolExecutor(max_workers=4) as executor:
            results = list(executor.map(start, range(4)))
        self.assertEqual(len(results), 4)
        state = self.cli("status")
        for index in range(4):
            self.assertIn(f"{self.project}/project-{index}：项目开关 已开启，有效状态 已开启，最近活动 -", state)

    @unittest.skipUnless(os.environ.get("BRIDGE_CMD"), "仅 Rust 目标运行双向互通")
    def test_interoperability_both_directions(self):
        self.cli("on", self.project, command=PYTHON)
        python, rust = self.session("claude", command=PYTHON), self.session("codex", command=COMMAND)
        for sender, receiver, identity in ((python, rust, "codex"), (rust, python, "claude")):
            result = sender.call("send_message", project=self.project, content="互通", to=identity)
            self.assertNotIn("isError", result)
            result = receiver.call("read_messages", project=self.project)
            self.assertIn("互通", result["content"][0]["text"])
            self.assertNotIn("isError", result)
            sender.call("update_status", project=self.project, task="共享状态")
            sender.call("claim_files", project=self.project, files=[identity + ".py"])
            overview = receiver.call("bridge_overview", project=self.project)
            self.assertIn("共享状态", overview["content"][0]["text"])
            self.assertIn(identity + ".py", overview["content"][0]["text"])
        self.assertEqual(normalized(self.cli("history", self.project, command=PYTHON)),
                         normalized(self.cli("history", self.project, command=COMMAND)))
        self.cli("off", self.project, command=PYTHON)
        self.assertIn("未在此项目开启", rust.call("bridge_overview", project=self.project)["content"][0]["text"])
        self.cli("on", self.project, command=COMMAND)
        self.assertIn("共享状态", python.call("bridge_overview", project=self.project)["content"][0]["text"])
