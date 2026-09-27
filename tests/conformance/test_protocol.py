import json

from .support import ContractCase, GOLDEN, GLOBAL_OFF, PROJECT_OFF


class ProtocolTests(ContractCase):
    def test_protocol_golden_and_persistent_requests(self):
        server = self.session()
        init = server.request("initialize", {"protocolVersion": "test-protocol"})["result"]
        self.assertEqual(init, {"protocolVersion": "test-protocol", "serverInfo": {
            "name": "bridge", "version": "0.1.0"}, "capabilities": {"tools": {}},
            "instructions": (GOLDEN / "instructions.txt").read_text(encoding="utf-8")})
        self.assertEqual(server.request("initialize")["result"]["protocolVersion"], "2025-06-18")
        self.assertEqual(server.request("tools/list")["result"],
                         json.loads((GOLDEN / "tools_list.json").read_text(encoding="utf-8")))
        server.send({"jsonrpc": "2.0", "method": "notifications/initialized"})
        self.assertEqual(server.request("ping")["result"], {})
        self.assertEqual(server.request("missing")["error"],
                         {"code": -32601, "message": "Method not found: missing"})
        server.send(b"{bad json\xff}")
        self.assertEqual(json.loads(server.responses.get(timeout=30)), {
            "jsonrpc": "2.0", "id": None, "error": {"code": -32700, "message": "Parse error"}})
        self.check(server.call("missing"), "未知工具：missing", error=True)
        self.assertEqual(server.request("ping")["result"], {})

    def test_default_off_then_live_switches(self):
        server = self.session()
        for name in ("bridge_overview", "update_status", "send_message", "read_messages",
                     "claim_files", "release_files"):
            self.check(server.call(name, project=self.project), PROJECT_OFF)
        self.assertFalse(self.database.exists())
        self.enable()
        self.check(server.call("update_status", project=self.project, task="中文"), "状态已更新（codex）。")
        self.check(server.call("list_projects"),
                   f"用过 Bridge 的项目：\n  {self.project}（最近活动 <时间>，已开启）")
        self.cli("off", self.project)
        self.check(server.call("bridge_overview", project=self.project), PROJECT_OFF)
        self.check(server.call("list_projects"),
                   f"用过 Bridge 的项目：\n  {self.project}（最近活动 <时间>，未开启）")
        self.cli("off", "--global")
        for name in ("bridge_overview", "update_status", "send_message", "read_messages",
                     "claim_files", "release_files", "list_projects"):
            self.check(server.call(name, project=self.project), GLOBAL_OFF)
        self.cli("on", self.project)
        self.check(server.call("bridge_overview", project=self.project), GLOBAL_OFF)
        self.cli("on", "--global")
        self.check(server.call("update_status", project=self.project, task="恢复"), "状态已更新（codex）。")

    def test_tool_errors_and_empty_results(self):
        server = self.session()
        self.check(server.call("list_projects"), "还没有项目使用过 Bridge。")
        self.enable()
        for name in ("bridge_overview", "update_status", "send_message", "read_messages",
                     "claim_files", "release_files"):
            self.check(server.call(name), "出错：缺少 project 参数：请填当前项目根目录的绝对路径", True)
            self.check(server.call(name, project="relative"), "出错：project 必须是项目根目录的绝对路径", True)
        self.check(server.call("send_message", project=self.project, content=" \n "), "出错：消息内容不能为空", True)
        self.check(server.call("claim_files", project=self.project, files=[]), "出错：files 不能为空", True)
        self.check(server.call("read_messages", project=self.project, limit="bad"),
                   "出错：limit 必须是整数", True)
        self.check(server.call("claim_files", project=self.project, files=["a"], ttl_minutes="bad"),
                   "出错：ttl_minutes 必须是整数", True)
        self.check(server.call("read_messages", project=self.project), "没有未读消息。")
        self.check(server.call("read_messages", project=self.project, include_read=True), "还没有任何消息。")
        self.check(server.call("release_files", project=self.project), "已释放 0 个文件。")
        self.check(server.call("bridge_overview", project=self.project),
                   f"项目：{self.project}\n你的身份：codex\n\n== 各方状态 ==\n  （还没有人汇报状态）"
                   "\n\n== 文件认领 ==\n  （没有文件被认领）\n\n== 给你的未读消息（0 条）==\n  （没有未读消息）")
