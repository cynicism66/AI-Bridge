"""只启动被测程序，不导入任何 bridge_mcp 模块。"""

import json
import os
from pathlib import Path
import queue
import re
import subprocess
import tempfile
import threading
import unittest

ROOT = Path(__file__).resolve().parents[2]
EXECUTABLE = Path(os.environ["BRIDGE_CMD"]) if os.environ.get("BRIDGE_CMD") else (
    ROOT / "target" / "release" / ("bridge-mcp.exe" if os.name == "nt" else "bridge-mcp"))
if not EXECUTABLE.is_file():
    raise RuntimeError("未找到 Rust 可执行文件，请先运行 cargo build --release -p bridge-mcp；或设置 BRIDGE_CMD。")
COMMAND = [str(EXECUTABLE.resolve())]
GOLDEN = ROOT / "crates" / "bridge-core" / "resources"
STAMP = "2090-01-02 03:04:05"
PROJECT_OFF = "Bridge 未在此项目开启（由用户控制）。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。"
GLOBAL_OFF = "Bridge 已被用户全局关闭。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。"


def normalized(text):
    return re.sub(r"\d{4}-\d\d-\d\d \d\d:\d\d:\d\d", "<时间>", text)


class Session:
    def __init__(self, env, command=COMMAND):
        self.process = subprocess.Popen(command, stdin=subprocess.PIPE, stdout=subprocess.PIPE,
                                        stderr=subprocess.PIPE, env=env, cwd=ROOT)
        self.responses = queue.Queue()
        self.reader = threading.Thread(target=self._read, daemon=True)
        self.reader.start()
        self.number = 0

    def _read(self):
        for line in self.process.stdout:
            self.responses.put(line)

    def send(self, value):
        line = value if isinstance(value, bytes) else json.dumps(value, ensure_ascii=False).encode("utf-8")
        self.process.stdin.write(line + b"\n")
        self.process.stdin.flush()

    def request(self, method, params=None):
        self.number += 1
        self.send({"jsonrpc": "2.0", "id": self.number, "method": method, "params": params or {}})
        response = json.loads(self.responses.get(timeout=30))
        assert response["id"] == self.number, response
        return response

    def call(self, name, **arguments):
        return self.request("tools/call", {"name": name, "arguments": arguments})["result"]

    def close(self):
        self.process.stdin.close()
        try:
            self.process.wait(timeout=10)
        except subprocess.TimeoutExpired:
            self.process.kill()
            self.process.wait()
        self.reader.join(timeout=10)
        stderr = self.process.stderr.read()
        self.process.stdout.close()
        self.process.stderr.close()
        assert self.process.returncode == 0, stderr.decode("utf-8", errors="replace")
        assert not stderr, stderr


class ContractCase(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="bridge-契约-")
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name).resolve()
        self.database = self.directory / "bridge.db"
        self.project = os.path.normcase(str(self.directory)).replace("\\", "/")
        self.env = {**os.environ, "BRIDGE_DB": str(self.database), "BRIDGE_AGENT": "codex",
                    "BRIDGE_FAKE_NOW": STAMP, "PYTHONUTF8": "1"}

    def session(self, agent="codex", clock=STAMP, command=COMMAND):
        session = Session({**self.env, "BRIDGE_AGENT": agent, "BRIDGE_FAKE_NOW": clock}, command)
        self.addCleanup(session.close)
        return session

    def cli(self, *args, command=COMMAND, ok=True, cwd=None):
        result = subprocess.run([*command, *args], env=self.env, cwd=cwd or ROOT,
                                capture_output=True, timeout=30)
        text = result.stdout.decode("utf-8").replace("\r\n", "\n")
        if ok:
            self.assertEqual(result.returncode, 0, result.stderr.decode("utf-8", errors="replace"))
            self.assertEqual(result.stderr, b"")
        else:
            self.assertNotEqual(result.returncode, 0)
        return text if ok else result.stderr.decode("utf-8", errors="replace")

    def initialize(self, project=None):
        # 只管理 BRIDGE_DB 指向的临时测试库；结对模板允许两端保持历史认领测试。
        self.cli("init", project or self.project, "--template", "结对流程", "--role", "开发甲=claude",
                 "--role", "开发乙=codex", "--goal", "契约测试", "--no-kickoff")

    def enable(self):
        self.cli("on", self.project)
        self.initialize()

    def check_board(self, result, text):
        # 旧公告板区段继续逐字验证；新增章程区段由 T06b 契约独立完整验证。
        result = json.loads(json.dumps(result))
        value = result["content"][0]["text"]
        identity, remaining = value.split("\n\n", 1)
        _, rest = remaining.split("== 各方状态 ==", 1)
        result["content"][0]["text"] = identity + "\n\n== 各方状态 ==" + rest
        self.check(result, text)

    def check(self, result, text, error=False):
        expected = {"content": [{"type": "text", "text": normalized(text)}]}
        if error:
            expected["isError"] = True
        actual = json.loads(normalized(json.dumps(result, ensure_ascii=False)))
        self.assertEqual(actual, expected)
