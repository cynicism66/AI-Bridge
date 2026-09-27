"""进程内协议测试及工具行为回归。"""

import io
import json
import os
from types import SimpleNamespace
from unittest.mock import patch

from tests.support import DatabaseTestCase
from bridge_mcp import __version__, server, store, tools


class ServerTests(DatabaseTestCase):
    def exchange(self, payload):
        with io.BytesIO(payload) as incoming, io.BytesIO() as outgoing:
            with patch.object(server.sys, "stdin", SimpleNamespace(buffer=incoming)), patch.object(
                server.sys, "stdout", SimpleNamespace(buffer=outgoing)
            ):
                server.serve()
            return [json.loads(line) for line in outgoing.getvalue().splitlines()]

    def call(self, name, **arguments):
        return server.handle({"method": "tools/call", "params": {
            "name": name, "arguments": {"project": self.project, **arguments},
        }})

    def text(self, name, **arguments):
        result = self.call(name, **arguments)
        self.assertNotIn("isError", result)
        return result["content"][0]["text"]

    def test_initialize_echoes_protocol_and_uses_package_version(self):
        result = server.handle({"method": "initialize", "params": {"protocolVersion": "future-version"}})
        self.assertEqual(result["protocolVersion"], "future-version")
        self.assertEqual(result["serverInfo"], {"name": "bridge", "version": __version__})
        self.assertEqual(result["instructions"], tools.INSTRUCTIONS)
        self.assertEqual(result["capabilities"], {"tools": {}})
        self.assertEqual(server.handle({"method": "initialize"})["protocolVersion"], "2025-06-18")

    def test_ping(self):
        self.assertEqual(server.handle({"method": "ping"}), {})

    def test_unknown_method_maps_to_jsonrpc_error(self):
        with self.assertRaises(LookupError):
            server.handle({"method": "不存在"})
        request = {"jsonrpc": "2.0", "id": 7, "method": "不存在"}
        responses = self.exchange((json.dumps(request) + "\n").encode("utf-8"))
        self.assertEqual(responses, [{"jsonrpc": "2.0", "id": 7, "error": {
            "code": -32601, "message": "Method not found: 不存在",
        }}])

    def test_unknown_tool_returns_error_content(self):
        self.assertEqual(self.call("不存在"), {
            "content": [{"type": "text", "text": "未知工具：不存在"}], "isError": True,
        })

    def test_tool_exception_is_reported_in_chinese(self):
        self.assertEqual(self.call("send_message", content="  "), {
            "content": [{"type": "text", "text": "出错：消息内容不能为空"}], "isError": True,
        })
        result = self.call("update_status", project="src", task="任务")
        self.assertTrue(result["isError"])
        self.assertIn("项目根目录的绝对路径", result["content"][0]["text"])
        self.assertEqual(store.list_projects(), [])

    def test_notifications_do_not_reply_or_dispatch(self):
        notifications = [
            {"jsonrpc": "2.0", "method": "notifications/initialized"},
            {"jsonrpc": "2.0", "method": "tools/call", "params": {
                "name": "update_status", "arguments": {"project": self.project, "task": "不应执行"},
            }},
        ]
        payload = b"\n".join(json.dumps(item).encode("utf-8") for item in notifications) + b"\n"
        self.assertEqual(self.exchange(payload), [])
        self.assertEqual(store.list_projects(), [])

    def test_invalid_json_returns_parse_error_and_server_continues(self):
        responses = self.exchange(b'\n{broken json}\n{"jsonrpc":"2.0","id":2,"method":"ping"}\n')
        self.assertEqual(responses[0], {"jsonrpc": "2.0", "id": None, "error": {
            "code": -32700, "message": "Parse error",
        }})
        self.assertEqual(responses[1], {"jsonrpc": "2.0", "id": 2, "result": {}})

    def test_agent_is_resolved_each_call_and_normalized(self):
        for value, expected in [(" CLAUDE ", "claude"), ("codex", "codex"), (" ", "unknown")]:
            with patch.dict(os.environ, {"BRIDGE_AGENT": value}):
                self.assertEqual(tools.current_agent(), expected)
                self.assertEqual(self.text("update_status", task=expected), f"状态已更新（{expected}）。")
                self.assertIn(f"你的身份：{expected}", self.text("bridge_overview"))
        with patch.dict(os.environ):
            os.environ.pop("BRIDGE_AGENT")
            self.assertEqual(tools.current_agent(), "unknown")
        statuses = store.overview(tools.norm_project(self.project), "codex")["statuses"]
        self.assertEqual({r["agent"] for r in statuses}, {"claude", "codex", "unknown"})

    def test_message_and_claim_tools_switch_identity_in_one_process(self):
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            with patch.dict(os.environ, {"BRIDGE_AGENT": "claude"}):
                self.assertEqual(self.text("send_message", to=" CODEX ", content="  请审查  "),
                                 "消息 #1 已发送给 codex。")
                self.assertEqual(self.text("claim_files", files=["./src/../a.py"], note="实现"),
                                 "已认领 1 个文件，到期 2026-01-01 10:00:00：\n  a.py")
                self.assertEqual(self.text("read_messages"), "没有未读消息。")
            with patch.dict(os.environ, {"BRIDGE_AGENT": "codex"}):
                self.assertEqual(self.text("update_status", task="审查"),
                                 "状态已更新（codex）。\n提示：你有 1 条未读消息，请用 read_messages 查看。")
                self.assertEqual(self.text("claim_files", files=["a.py"]),
                                 "认领失败，以下文件已被别人认领（本次一个都没认领）：\n"
                                 "  a.py ← claude（实现），到期 2026-01-01 10:00:00\n"
                                 "请先用 send_message 和对方协商，或等对方释放。")
                self.assertEqual(self.text("release_files", files=["a.py"]), "已释放 0 个文件。")
                self.assertEqual(self.text("read_messages"),
                                 "未读消息（1 条，已标为已读）：\n"
                                 "  #1 [2026-01-01 09:00:00] claude → codex：请审查")
            with patch.dict(os.environ, {"BRIDGE_AGENT": "claude"}):
                self.assertEqual(self.text("release_files"), "已释放 1 个文件。")

    def test_overview_text_and_mark_read_match_existing_behavior(self):
        project = tools.norm_project(self.project)
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            store.update_status(project, "claude", "实现", "进行中", "等待", "测试")
            store.send_message(project, "claude", "all", "交接")
            store.claim_files(project, "claude", ["a.py"], 60)
            expected = (f"项目：{project}\n你的身份：codex\n\n== 各方状态 ==\n"
                        "【claude】更新于 2026-01-01 09:00:00\n  任务：实现\n"
                        "  进度：进行中\n  卡点：等待\n  下一步：测试\n\n== 文件认领 ==\n"
                        "  a.py ← claude，到期 2026-01-01 10:00:00\n\n== 给你的未读消息（1 条）==\n"
                        "  #1 [2026-01-01 09:00:00] claude → 所有人：交接")
            self.assertEqual(self.text("bridge_overview", mark_read=False), expected)
            self.assertEqual(self.text("bridge_overview"), expected + "\n  （以上消息已标为已读）")
            self.assertEqual(self.text("read_messages"), "没有未读消息。")
            self.assertIn("你的身份：human", tools.show_text(self.project))

    def test_empty_and_recent_message_texts_and_project_list(self):
        self.assertEqual(self.text("list_projects"), "还没有项目使用过 Bridge。")
        self.assertEqual(self.text("read_messages", include_read=True), "还没有任何消息。")
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            self.assertEqual(self.text("send_message", content="广播"), "消息 #1 已发送给 所有人。")
        self.assertEqual(self.text("read_messages", include_read=True),
                         "最近的消息：\n  #1 [2026-01-01 09:00:00] codex → 所有人：广播")
        self.assertEqual(self.text("list_projects"),
                         f"用过 Bridge 的项目：\n  {tools.norm_project(self.project)}（最近活动 2026-01-01 09:00:00）")
