import os
import sqlite3
import subprocess
from contextlib import closing
from pathlib import Path
from .support import ContractCase, COMMAND, ROOT


class TransferCase(ContractCase):
    def snapshot(self):
        with closing(sqlite3.connect(self.database)) as db:
            tables = [r[0] for r in db.execute("SELECT name FROM sqlite_master WHERE type='table' ORDER BY name")]
            return {t: db.execute(f'SELECT * FROM "{t}"').fetchall() for t in tables}

    def transfer(self, *args, answer="", ok=True, cwd=None):
        result = subprocess.run([*COMMAND, *args], input=answer.encode(), env=self.env,
                                cwd=cwd or ROOT, capture_output=True, timeout=30)
        stdout, stderr = result.stdout.decode("utf-8"), result.stderr.decode("utf-8")
        # Each invocation owns an isolated preview directory outside the project.
        for line in stdout.splitlines():
            if line.startswith("预览文件："):
                preview = Path(line.split("：", 1)[1])
                self.assertFalse(preview.resolve().is_relative_to(self.directory))
                self.addCleanup(preview.parent.rmdir)
                self.addCleanup(preview.unlink)
                self.assertEqual(result.returncode == 0, ok, stdout + stderr)
                return stdout, stderr, preview.read_text(encoding="utf-8")
        self.assertEqual(result.returncode == 0, ok, stdout + stderr)
        return stdout, stderr, ""

    def task_init(self, *extra):
        self.cli("init", self.project, "--template", "任务书流程", "--role", "规划审查=claude",
                 "--role", "执行=codex", "--goal", "项目中文目标", "--no-kickoff", *extra)
