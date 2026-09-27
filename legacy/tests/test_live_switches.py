"""正在运行的 MCP 子进程实时读取外部 CLI 修改的开关。"""

import json
import os
import queue
import subprocess
import sys
import threading

from tests.support import DatabaseTestCase, ROOT
from bridge_mcp import tools


class LiveSwitchTests(DatabaseTestCase):
    def test_running_server_observes_cli_switches_without_restart(self):
        command = [sys.executable, "-X", "dev", "-W", "error", str(ROOT / "bridge.py")]
        environment = {**os.environ, "PYTHONIOENCODING": "ascii"}
        responses = queue.Queue()
        with subprocess.Popen(command, env=environment, stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
            def collect():
                for line in process.stdout:
                    responses.put(line)

            reader = threading.Thread(target=collect, daemon=True)
            reader.start()
            counter = 0

            def rpc(name):
                nonlocal counter
                counter += 1
                request = {"jsonrpc": "2.0", "id": counter, "method": "tools/call", "params": {
                    "name": name, "arguments": {"project": self.project, "task": "实时开关"},
                }}
                process.stdin.write((json.dumps(request, ensure_ascii=False) + "\n").encode("utf-8"))
                process.stdin.flush()
                response = json.loads(responses.get(timeout=5).decode("utf-8"))
                self.assertEqual(response["id"], counter)
                self.assertNotIn("error", response)
                self.assertNotIn("isError", response["result"])
                return response["result"]["content"][0]["text"]

            def cli(*arguments):
                result = subprocess.run(command + list(arguments), env=environment,
                                        capture_output=True, check=True, timeout=5)
                self.assertEqual(result.stderr, b"")

            try:
                self.assertEqual(rpc("update_status"), tools.PROJECT_OFF)
                cli("on", self.project)
                self.assertEqual(rpc("update_status"), "状态已更新（codex）。")
                cli("off", self.project)
                self.assertEqual(rpc("bridge_overview"), tools.PROJECT_OFF)
                cli("on", self.project)
                cli("off", "--global")
                self.assertEqual(rpc("list_projects"), tools.GLOBAL_OFF)
                self.assertEqual(rpc("bridge_overview"), tools.GLOBAL_OFF)
                cli("on", "--global")
                self.assertIn("实时开关", rpc("bridge_overview"))
            finally:
                process.stdin.close()
                try:
                    process.wait(timeout=5)
                except subprocess.TimeoutExpired:
                    process.kill()
                    process.wait(timeout=5)
                reader.join(timeout=5)
            self.assertFalse(reader.is_alive())
            self.assertEqual(process.returncode, 0)
            self.assertEqual(process.stderr.read(), b"")
