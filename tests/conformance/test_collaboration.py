import os

from .support import ContractCase, normalized


class CollaborationTests(ContractCase):
    def test_status_messages_identity_and_human(self):
        self.enable()
        codex, claude, human = self.session(), self.session("claude"), self.session("human")
        for recipient, content in (("codex", "定向中文"), ("all", "广播"), ("human", "给用户")):
            result = claude.call("send_message", project=self.project, to=recipient, content=content)
            self.assertNotIn("isError", result)
        self.check(codex.call("update_status", project=self.project, task="任务", progress="实现中",
                              blockers="无", next_step="测试"),
                   "状态已更新（codex）。\n提示：你有 2 条未读消息，请用 read_messages 查看。")
        overview = (f"项目：{self.project}\n你的身份：codex\n\n== 各方状态 ==\n【codex】更新于 <时间>"
                    "\n  任务：任务\n  进度：实现中\n  卡点：无\n  下一步：测试"
                    "\n\n== 文件认领 ==\n  （没有文件被认领）\n\n== 给你的未读消息（2 条）=="
                    "\n  #1 [<时间>] claude → codex：定向中文\n  #2 [<时间>] claude → 所有人：广播")
        self.check(codex.call("bridge_overview", project=self.project, mark_read=False), overview)
        self.check(codex.call("bridge_overview", project=self.project), overview + "\n  （以上消息已标为已读）")
        self.check(codex.call("read_messages", project=self.project), "没有未读消息。")
        self.check(codex.call("read_messages", project=self.project, include_read=True, limit=1),
                   "最近的消息：\n  #2 [<时间>] claude → 所有人：广播")
        self.check(human.call("read_messages", project=self.project, limit=1),
                   "未读消息（1 条，已标为已读）：\n  #2 [<时间>] claude → 所有人：广播")
        self.assertEqual(normalized(self.cli("read", self.project)),
                         "未读消息（1 条，已标为已读）：\n  #3 [<时间>] claude → human：给用户\n")
        self.assertEqual(self.cli("read", self.project), "没有未读消息。\n")
        self.assertEqual(self.cli("post", self.project, " 用户留言 ", "--to", "codex"), "消息 #4 已发送给 codex。\n")
        self.check(codex.call("read_messages", project=self.project),
                   "未读消息（1 条，已标为已读）：\n  #4 [<时间>] human → codex：用户留言")
        self.check(claude.call("read_messages", project=self.project), "没有未读消息。")

    def test_atomic_claim_renew_expire_and_owner_release(self):
        self.enable()
        codex, claude = self.session(), self.session("claude")
        self.check(codex.call("claim_files", project=self.project, files=["src/a.py"], note="重构", ttl_minutes=10),
                   "已认领 1 个文件，到期 <时间>：\n  src/a.py")
        self.check(claude.call("claim_files", project=self.project, files=["other.py", "src/a.py"]),
                   "认领失败，以下文件已被别人认领（本次一个都没认领）：\n"
                   "  src/a.py ← codex（重构），到期 <时间>\n请先用 send_message 和对方协商，或等对方释放。")
        claude.call("claim_files", project=self.project, files=["held.py"], ttl_minutes=1)
        before = self.cli("history", self.project, "--kind", "claim", "--agent", "claude")
        claude.call("claim_files", project=self.project, files=["held.py", "src/a.py"], ttl_minutes=60)
        self.assertEqual(self.cli("history", self.project, "--kind", "claim", "--agent", "claude"), before)
        self.check(claude.call("release_files", project=self.project, files=["src/a.py"]), "已释放 0 个文件。")
        self.check(codex.call("claim_files", project=self.project, files=["src/a.py"], ttl_minutes=30),
                   "已认领 1 个文件，到期 <时间>：\n  src/a.py")
        self.check(codex.call("claim_files", project=self.project, files=["other.py"]),
                   "已认领 1 个文件，到期 <时间>：\n  other.py")
        later = self.session("claude", "2090-01-02 03:35:00")
        self.check(later.call("claim_files", project=self.project, files=["src/a.py"]),
                   "已认领 1 个文件，到期 <时间>：\n  src/a.py")
        self.check(later.call("release_files", project=self.project), "已释放 1 个文件。")
        self.check(codex.call("release_files", project=self.project), "已释放 1 个文件。")

    def test_file_path_normalization(self):
        self.enable()
        server = self.session()
        source = self.project + "/Src/../A.py"
        expected = "a.py" if os.name == "nt" else "A.py"
        self.check(server.call("claim_files", project=self.project, files=[source, "src\\x\\..\\b.py"]),
                   f"已认领 2 个文件，到期 <时间>：\n  {expected}\n  src/b.py")
        self.check(server.call("release_files", project=self.project + "/x/..", files=["./" + expected]),
                   "已释放 1 个文件。")
        if os.name == "nt":
            self.check(server.call("release_files", project=self.project.upper().replace("/", "\\")),
                       "已释放 1 个文件。")

    def test_windows_project_rules_at_both_interfaces(self):
        if os.name != "nt":
            self.skipTest("Windows 专有路径规则")
        server = self.session()
        message = "Windows 项目路径必须带盘符或使用 UNC 路径；Git Bash 路径请用 /d/x 格式"
        for invalid in ("\\x", "/x", "/abc/x"):
            self.check(server.call("bridge_overview", project=invalid), "出错：" + message, True)
            self.assertIn(message, self.cli("on", invalid, ok=False))
        converted = "/" + self.project[0] + self.project[2:]
        self.assertEqual(self.cli("on", converted), f"项目开关：已开启（{self.project}）\n")
        self.check(server.call("update_status", project=converted, task="bash"), "状态已更新（codex）。")
        double = self.project.replace(":/", "://")
        self.check(server.call("update_status", project=double, task="双斜杠"), "状态已更新（codex）。")
        self.assertEqual(self.cli("on", double), f"项目开关：已开启（{self.project}）\n")
        unc = "\\\\server\\share\\project"
        self.cli("on", unc)
        self.check(server.call("update_status", project=unc, task="UNC"), "状态已更新（codex）。")
