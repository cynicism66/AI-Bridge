"""路径规范化：同时检查 Windows 和 POSIX 规则。"""

import ntpath
import posixpath
from types import SimpleNamespace
from unittest.mock import patch

from tests.support import DatabaseTestCase
from bridge_mcp import paths


class PathTests(DatabaseTestCase):
    def test_project_case_separators_and_trailing_slash(self):
        for module, source, expected in [
            (ntpath, " C:\\Work\\Demo\\ ", "c:/work/demo"),
            (posixpath, " /Work/Demo/ ", "/Work/Demo"),
        ]:
            with self.subTest(platform=module.__name__), patch.object(
                paths, "os", SimpleNamespace(path=module)
            ):
                self.assertEqual(paths.norm_project(source), expected)

    def test_empty_and_relative_projects_are_rejected(self):
        for module in (ntpath, posixpath):
            with patch.object(paths, "os", SimpleNamespace(path=module)):
                for project in (None, "", "  ", ".", "src", "../project"):
                    with self.subTest(platform=module.__name__, project=project):
                        with self.assertRaisesRegex(ValueError, "项目根目录的绝对路径"):
                            paths.norm_project(project)

    def test_file_dot_segments_and_backslashes(self):
        for module, project in [(ntpath, "c:/work"), (posixpath, "/work")]:
            with patch.object(paths, "os", SimpleNamespace(path=module)):
                for source, expected in [
                    ("src/../src/a.py", "src/a.py"),
                    ("./src/./a.py", "src/a.py"),
                    (".\\src\\a.py", "src/a.py"),
                    ("src/a.py/", "src/a.py"),
                    ("src/../../outside.py", "../outside.py"),
                    ("../outside/../a.py", "../a.py"),
                    ("./", "."),
                ]:
                    with self.subTest(platform=module.__name__, source=source):
                        self.assertEqual(paths.norm_file(project, source), expected)

    def test_windows_absolute_files_case_and_drives(self):
        with patch.object(paths, "os", SimpleNamespace(path=ntpath)):
            for source, expected in [
                ("SRC\\A.PY", "src/a.py"),
                ("C:\\Work\\src\\..\\A.py", "a.py"),
                ("C:/Work2/A.py", "c:/work2/a.py"),
                ("C:/Outside/A.py", "c:/outside/a.py"),
                ("D:/Work/A.py", "d:/work/a.py"),
            ]:
                with self.subTest(source=source):
                    self.assertEqual(paths.norm_file("c:/work", source), expected)

    def test_posix_absolute_files_preserve_case(self):
        with patch.object(paths, "os", SimpleNamespace(path=posixpath)):
            self.assertEqual(paths.norm_file("/work", "/work/Src/A.py"), "Src/A.py")
            self.assertEqual(paths.norm_file("/work", "/outside/A.py"), "/outside/A.py")
            self.assertEqual(paths.norm_file("/work", "/work2/A.py"), "/work2/A.py")
