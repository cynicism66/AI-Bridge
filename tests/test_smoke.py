"""通过真实 stdio 子进程验收兼容入口；数据库始终位于临时目录。"""

import json
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest

ROOT = Path(__file__).resolve().parents[1]
TOOL_NAMES = [
    "bridge_overview", "update_status", "send_message", "read_messages",
    "claim_files", "release_files", "list_projects",
]


class SmokeTests(unittest.TestCase):
    def setUp(self):
        self.directory = tempfile.TemporaryDirectory(prefix="bridge-smoke-")
        self.addCleanup(self.directory.cleanup)
        self.env = os.environ.copy()
        self.env["BRIDGE_DB"] = str(Path(self.directory.name) / "bridge.db")
        # 刻意使用非 UTF-8 文本编码，确保协议走二进制流。
        self.env["PYTHONIOENCODING"] = "ascii"
        self.project = str(Path(self.directory.name) / "中文项目")

    def rpc(self, method, params=None, agent="codex"):
        request = {"jsonrpc": "2.0", "id": "中文请求", "method": method}
        if params is not None:
            request["params"] = params
        payload = (json.dumps(request, ensure_ascii=False) + "\n").encode("utf-8")
        self.assertIn("中文请求".encode("utf-8"), payload)
        result = subprocess.run(
            [sys.executable, "-X", "dev", "-W", "error", str(ROOT / "bridge.py")],
            input=payload, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
            env={**self.env, "BRIDGE_AGENT": agent}, cwd=self.directory.name,
            timeout=15, check=True,
        )
        self.assertEqual(result.stderr, b"")
        self.assertIn("中文请求".encode("utf-8"), result.stdout)
        response = json.loads(result.stdout.decode("utf-8"))
        self.assertEqual(response["id"], request["id"])
        self.assertNotIn("error", response)
        self.assertNotIn("isError", response["result"])
        return response["result"], result.stdout

    def test_initialize(self):
        result, raw = self.rpc("initialize", {"protocolVersion": "2025-06-18"})
        self.assertEqual(result["serverInfo"]["name"], "bridge")
        self.assertEqual(result["protocolVersion"], "2025-06-18")
        self.assertTrue(result["instructions"])
        self.assertIn("协作的公告板".encode("utf-8"), raw)

    def test_tool_names(self):
        result, _ = self.rpc("tools/list")
        self.assertEqual([tool["name"] for tool in result["tools"]], TOOL_NAMES)

    def test_agents_share_chinese_status(self):
        task = "拆分模块，保留中文输出"
        result, raw = self.rpc("tools/call", {
            "name": "update_status", "arguments": {"project": self.project, "task": task},
        }, agent="claude")
        self.assertEqual(result["content"][0]["text"], "状态已更新（claude）。")
        self.assertIn("状态已更新（claude）。".encode("utf-8"), raw)
        result, raw = self.rpc("tools/call", {
            "name": "bridge_overview", "arguments": {"project": self.project},
        }, agent="codex")
        text = result["content"][0]["text"]
        self.assertIn("你的身份：codex", text)
        self.assertIn("【claude】", text)
        self.assertIn(task, text)
        self.assertIn(task.encode("utf-8"), raw)
        self.assertIn("中文项目".encode("utf-8"), raw)

    def test_multiple_requests_in_one_server_process(self):
        requests = [
            {"id": 1, "method": "initialize"},
            {"method": "notifications/initialized"},
            {"id": 2, "method": "tools/list"},
            {"id": 3, "method": "tools/call", "params": {
                "name": "update_status", "arguments": {"project": self.project, "task": "连续请求"},
            }},
            {"id": 4, "method": "tools/call", "params": {
                "name": "bridge_overview", "arguments": {"project": self.project},
            }},
        ]
        payload = b"".join((json.dumps({"jsonrpc": "2.0", **request}, ensure_ascii=False) + "\n").encode("utf-8")
                           for request in requests)
        result = subprocess.run(
            [sys.executable, "-X", "dev", "-W", "error", str(ROOT / "bridge.py")],
            input=payload, capture_output=True, env={**self.env, "BRIDGE_AGENT": "codex"},
            cwd=self.directory.name, timeout=15, check=True,
        )
        self.assertEqual(result.stderr, b"")
        replies = [json.loads(line.decode("utf-8")) for line in result.stdout.splitlines()]
        self.assertEqual([reply["id"] for reply in replies], [1, 2, 3, 4])
        self.assertEqual(replies[0]["result"]["serverInfo"]["name"], "bridge")
        self.assertEqual([tool["name"] for tool in replies[1]["result"]["tools"]], TOOL_NAMES)
        self.assertEqual(replies[2]["result"]["content"][0]["text"], "状态已更新（codex）。")
        self.assertIn("连续请求", replies[3]["result"]["content"][0]["text"])


if __name__ == "__main__":
    unittest.main()
