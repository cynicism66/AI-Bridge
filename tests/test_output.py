"""控制台 Unicode、重定向 UTF-8 和提前关闭的管道。"""

import errno
import io
import os
import subprocess
import sys
from unittest.mock import Mock, patch

from tests.support import DatabaseTestCase, ROOT
from bridge_mcp import cli, output


class OutputTests(DatabaseTestCase):
    def test_console_uses_text_stream_not_encoded_bytes(self):
        console = Mock()
        console.isatty.return_value = True
        output.write("中文公告板", stream=console)
        console.write.assert_called_once_with("中文公告板\n")
        console.flush.assert_called_once_with()
        console.buffer.write.assert_not_called()

    def test_redirected_output_is_utf8_even_if_text_encoding_is_ascii(self):
        with io.BytesIO() as raw, io.TextIOWrapper(raw, encoding="ascii") as stream:
            output.write("中文公告板", stream=stream)
            self.assertEqual(raw.getvalue(), "中文公告板\n".encode("utf-8"))

    def test_closed_pipe_is_silent_for_write_and_flush_failures(self):
        for stage in ("write", "flush"):
            for error in (BrokenPipeError(), OSError(errno.EINVAL, "管道关闭")):
                with self.subTest(stage=stage, error=error):
                    stream = Mock()
                    stream.isatty.return_value = False
                    getattr(stream.buffer, stage).side_effect = error
                    with patch.object(sys, "stdout", stream), patch.object(output, "silence_broken_pipe") as silence:
                        cli.main(["show", self.project])
                    silence.assert_called_once_with()

    def test_unrelated_output_error_is_not_hidden(self):
        stream = Mock()
        stream.isatty.return_value = True
        stream.write.side_effect = OSError(errno.ENOSPC, "空间不足")
        with self.assertRaises(OSError):
            output.write("中文", stream=stream)

    def test_real_subprocess_redirects_chinese_as_utf8(self):
        result = subprocess.run(
            [sys.executable, "-X", "dev", "-W", "error", str(ROOT / "bridge.py"), "show", self.project],
            env={**os.environ, "PYTHONIOENCODING": "ascii"}, capture_output=True, check=True, timeout=10,
        )
        self.assertEqual(result.stderr, b"")
        self.assertIn("启用状态：未开启".encode("utf-8"), result.stdout)
        self.assertIn("你的身份：human", result.stdout.decode("utf-8"))

    def test_real_subprocess_exits_cleanly_when_pipe_reader_closes(self):
        with subprocess.Popen(
            [sys.executable, "-X", "dev", "-W", "error", str(ROOT / "bridge.py"), "show", self.project],
            env=os.environ.copy(), stdin=subprocess.DEVNULL, stdout=subprocess.PIPE, stderr=subprocess.PIPE,
        ) as process:
            process.stdout.close()
            process.stdout = None
            _, errors = process.communicate(timeout=10)
            self.assertEqual(process.returncode, 0)
            self.assertEqual(errors, b"")
