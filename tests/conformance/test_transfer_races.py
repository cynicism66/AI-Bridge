import os
import queue
import subprocess
import threading
from pathlib import Path
from .support import COMMAND, ROOT
from .transfer_support import TransferCase


class TransferRaceTests(TransferCase):
    def preview_then_change(self, change):
        with subprocess.Popen([*COMMAND, "handover", self.project, "--to", "claude"],
                              env=self.env, cwd=ROOT, stdin=subprocess.PIPE,
                              stdout=subprocess.PIPE, stderr=subprocess.PIPE) as process:
            ready = queue.Queue()
            prompt = "确认写入？(y/N) ".encode()
            def read_preview():
                data = bytearray()
                while not data.endswith(prompt):
                    value = process.stdout.read(1)
                    if not value:
                        break
                    data.extend(value)
                ready.put(bytes(data))
            reader = threading.Thread(target=read_preview)
            reader.start()
            try:
                preview_output = ready.get(timeout=15).decode("utf-8")
                self.assertIn("确认写入？", preview_output)
                for line in preview_output.splitlines():
                    if line.startswith("预览文件："):
                        path = Path(line.split("：", 1)[1])
                        self.addCleanup(path.parent.rmdir)
                        self.addCleanup(path.unlink)
                change()
                stdout, stderr = process.communicate(b"y\n", timeout=15)
                self.assertNotEqual(process.returncode, 0, preview_output + stdout.decode())
                return stderr.decode("utf-8")
            finally:
                if process.poll() is None:
                    process.kill()
                    process.communicate()
                reader.join(timeout=15)

    def test_db_change_after_preview_requires_new_confirmation(self):
        self.enable()
        saved = []
        def change():
            self.cli("post", self.project, "new-message-after-preview")
            saved.append(self.snapshot())
        error = self.preview_then_change(change)
        self.assertIn("预览后协作数据发生变化", error)
        self.assertEqual(self.snapshot(), saved[0])
        self.assertFalse((self.directory / "docs").exists())

    def test_destination_change_is_preserved_and_db_rolled_back(self):
        self.enable()
        before = self.snapshot()
        def change():
            path = self.directory / "docs/HANDOVER.md"
            path.parent.mkdir()
            path.write_bytes(b"user edit during preview")
        error = self.preview_then_change(change)
        self.assertIn("预览后目标文件发生变化", error)
        self.assertEqual(self.snapshot(), before)
        self.assertEqual((self.directory / "docs/HANDOVER.md").read_bytes(), b"user edit during preview")

    def test_output_cannot_follow_windows_junction(self):
        if os.name != "nt":
            self.skipTest("Windows junction")
        self.enable()
        outside = self.directory / "outside"
        outside.mkdir()
        # A link inside the project is rejected even when it points inside the project.
        junction = self.directory / "linked"
        result = subprocess.run(["powershell", "-NoProfile", "-Command",
            "$ErrorActionPreference='Stop'; New-Item -ItemType Junction -Path $env:BRIDGE_LINK -Target $env:BRIDGE_TARGET | Out-Null"],
            env={**self.env, "BRIDGE_LINK": str(junction), "BRIDGE_TARGET": str(outside)}, capture_output=True, timeout=30)
        self.assertEqual(result.returncode, 0, result.stderr)
        try:
            before = self.snapshot()
            _, error, _ = self.transfer("export", self.project, "--out", "linked/export.md", "--yes", ok=False)
            self.assertTrue("重解析点" in error or "符号链接" in error, error)
            self.assertEqual(before, self.snapshot())
            self.assertFalse((outside / "export.md").exists())
        finally:
            junction.rmdir()
