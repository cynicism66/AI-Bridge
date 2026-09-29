import os
import sqlite3
from contextlib import closing
import subprocess
from concurrent.futures import ThreadPoolExecutor
from .support import ContractCase, COMMAND, normalized, display_path


def key(path):
    return os.path.normcase(str(path)).replace("\\", "/")


def content(result):
    assert not result.get("isError"), result
    return result["content"][0]["text"]


class SessionTests(ContractCase):
    def git_pair(self):
        main, wt = self.directory / "main", self.directory / "worktree"
        common = main / ".git"
        private = common / "worktrees" / "wt"
        private.mkdir(parents=True)
        wt.mkdir()
        (main / "child").mkdir()
        (common / "HEAD").write_text("ref: refs/heads/main\n", encoding="utf-8")
        (private / "HEAD").write_text("ref: refs/heads/feature/中文\n", encoding="utf-8")
        (private / "commondir").write_text("../..\n", encoding="utf-8")
        (wt / ".git").write_text("gitdir: ../main/.git/worktrees/wt\n", encoding="utf-8")
        self.cli("on", str(main / "child"))
        return main, wt, private

    def test_worktrees_share_board_and_sessions_keep_ownership(self):
        main, wt, private = self.git_pair()
        first, second = self.session(), self.session()
        first.call("update_status", project=str(main / "child"), task="主目录任务")
        second.call("update_status", project=str(wt), task="工作树任务")
        overview = content(second.call("bridge_overview", project=str(wt)))
        self.assertIn(f"项目：{key(main)}", overview)
        self.assertIn(f"你的身份：codex #2 · 分支 feature/中文 · 文件夹 {key(wt)}", overview)
        self.assertIn(f"【codex #1】分支 main · 文件夹 {key(main)}", overview)
        self.assertIn("任务：主目录任务", overview)
        self.assertIn("任务：工作树任务", overview)
        first.call("claim_files", project=str(main), files=[str(main / "src/a.rs")])
        self.assertIn("认领失败", content(second.call("claim_files", project=str(wt), files=[str(wt / "src/a.rs")])))
        self.check(second.call("release_files", project=str(wt)), "已释放 0 个文件。")
        self.check(second.call("release_files", project=str(wt), files=["src/a.rs"]), "已释放 0 个文件。")
        self.check(first.call("release_files", project=str(main)), "已释放 1 个文件。")
        (private / "HEAD").write_text("0123456789abcdef\n", encoding="utf-8")
        self.assertIn("分支 0123456", content(second.call("bridge_overview", project=str(wt))))
        self.assertIn("工作树任务", self.cli("show", str(main / "child")))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM status").fetchone()[0], 2)
            self.assertEqual(db.execute("SELECT count(DISTINCT id) FROM sessions").fetchone()[0], 2)

    def test_messages_read_receipts_shared_by_agent(self):
        self.enable()
        one, two, sender = self.session(), self.session(), self.session("claude")
        sender.call("send_message", project=self.project, to="codex", content="两窗口共享")
        self.assertIn("两窗口共享", content(one.call("read_messages", project=self.project)))
        self.check(two.call("read_messages", project=self.project), "没有未读消息。")
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT agent FROM reads").fetchall(), [("codex",)])

    def test_concurrent_numbers_and_cross_project_numbering(self):
        self.enable()
        servers = [self.session() for _ in range(4)]
        with ThreadPoolExecutor(max_workers=4) as pool:
            rows = list(pool.map(lambda s: content(s.call("bridge_overview", project=self.project)), servers))
        for n in range(1, 5):
            self.assertEqual(sum(f"你的身份：codex #{n} ·" in row for row in rows), 1)
        other = self.directory / "other"
        other.mkdir()
        self.cli("on", str(other))
        self.assertIn("你的身份：codex #1 ·", content(servers[0].call("bridge_overview", project=str(other))))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT count(*) FROM sessions").fetchone()[0], 5)
            self.assertEqual(db.execute("SELECT count(DISTINCT id) FROM sessions").fetchone()[0], 4)

    def test_reused_number_cannot_release_previous_process_claims(self):
        self.enable()
        old = self.session(clock="2090-01-01 00:00:00")
        old.call("claim_files", project=self.project, files=["old.rs"], ttl_minutes=10000)
        new = self.session()
        self.assertIn("codex #1 ·", content(new.call("bridge_overview", project=self.project)))
        self.check(new.call("release_files", project=self.project), "已释放 0 个文件。")
        self.assertIn("认领失败", content(new.call("claim_files", project=self.project, files=["old.rs"])))
        self.check(old.call("release_files", project=self.project), "已释放 1 个文件。")

    def test_older_statuses_are_collapsed_using_last_activity(self):
        self.enable()
        old = self.session(clock="2089-12-30 03:04:05")
        old.call("update_status", project=self.project, task="三天前任务")
        viewer = self.session("claude")
        board = content(viewer.call("bridge_overview", project=self.project))
        self.assertIn("较早的会话（1）", board)
        self.assertNotIn("任务：三天前任务", board)
        expanded = content(viewer.call("bridge_overview", project=self.project, include_older=True))
        self.assertIn("任务：三天前任务", expanded)
        self.assertIn("任务：三天前任务", self.cli("show", self.project, "--include-older"))

    def test_malformed_git_is_nonfatal_and_diagnostic_is_stderr_only(self):
        (self.directory / ".git").write_text("gitdir: missing\n", encoding="utf-8")
        result = subprocess.run([*COMMAND, "on", self.project], env=self.env, capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0)
        self.assertIn(f"项目开关：已开启（{display_path(self.project)}）", result.stdout.decode("utf-8"))
        self.assertIn("退回原路径", result.stderr.decode("utf-8"))
        self.assertEqual(len(result.stderr.splitlines()), 1)
        # MCP 同样退回路径，且 stdout 仍只含 JSON。
        import json
        request = {"jsonrpc": "2.0", "id": 1, "method": "tools/call", "params": {
            "name": "bridge_overview", "arguments": {"project": self.project}}}
        result = subprocess.run(COMMAND, input=(json.dumps(request)+"\n").encode(), env=self.env,
                                capture_output=True, timeout=20)
        self.assertEqual(result.returncode, 0)
        self.assertIn(self.project, content(json.loads(result.stdout)["result"]))
        self.assertEqual(len(result.stderr.splitlines()), 1)

    def test_windows_short_and_long_git_paths_share_identity(self):
        if os.name != "nt":
            self.skipTest("Windows 8.3 路径")
        import ctypes
        from ctypes import wintypes
        main, _, _ = self.git_pair()
        get_short = ctypes.WinDLL("kernel32", use_last_error=True).GetShortPathNameW
        get_short.argtypes = [wintypes.LPCWSTR, wintypes.LPWSTR, wintypes.DWORD]
        get_short.restype = wintypes.DWORD
        buffer = ctypes.create_unicode_buffer(32768)
        self.assertGreater(get_short(str(main), buffer, len(buffer)), 0)
        if buffer.value.lower() == str(main).lower():
            self.skipTest("该卷没有生成 8.3 别名")
        server = self.session()
        server.call("bridge_overview", project=str(main))  # 随后比较两种路径的完整返回
        via_short = content(server.call("bridge_overview", project=buffer.value))
        via_long = content(server.call("bridge_overview", project=str(main)))
        self.assertEqual(via_short, via_long)
        self.assertIn(f"文件夹 {key(main)}", via_short)
