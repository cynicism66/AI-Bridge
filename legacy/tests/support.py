"""共享测试环境：每个用例独立使用临时数据库。"""

import os
from pathlib import Path
import sys
import tempfile
import unittest
from unittest.mock import patch

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "src"))


class DatabaseTestCase(unittest.TestCase):
    def setUp(self):
        temporary = tempfile.TemporaryDirectory(prefix="bridge-test-")
        self.addCleanup(temporary.cleanup)
        self.directory = Path(temporary.name)
        self.database = self.directory / "bridge.db"
        self.project = str(self.directory / "项目")
        environment = patch.dict(os.environ, {
            "BRIDGE_DB": str(self.database), "BRIDGE_AGENT": "codex",
        })
        environment.start()
        self.addCleanup(environment.stop)
