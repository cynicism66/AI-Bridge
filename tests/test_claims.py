"""文件认领的批次冲突、续期、到期和权限。"""

import os
from unittest.mock import patch

from tests.support import DatabaseTestCase
from bridge_mcp import store, tools


class ClaimTests(DatabaseTestCase):
    def test_conflict_prevents_entire_batch_including_renewal(self):
        store.claim_files(self.project, "claude", ["busy.py"], 60)
        store.claim_files(self.project, "codex", ["mine.py"], 60, "原认领")
        before = store.overview(self.project, "codex")["claims"]
        result = store.claim_files(self.project, "codex", ["new.py", "mine.py", "busy.py"], 120)
        self.assertEqual([r["path"] for r in result["conflicts"]], ["busy.py"])
        self.assertEqual(store.overview(self.project, "codex")["claims"], before)

    def test_same_agent_renews_claim_using_patched_clock(self):
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            first = store.claim_files(self.project, "claude", ["a.py"], 60)
        with patch.object(store, "now", return_value="2026-01-01 09:30:00"):
            second = store.claim_files(self.project, "claude", ["a.py"], 60, "续期")
            rows = store.overview(self.project, "codex")["claims"]
        self.assertEqual(first["expires"], "2026-01-01 10:00:00")
        self.assertEqual(second, {"conflicts": [], "expires": "2026-01-01 10:30:00"})
        self.assertEqual(len(rows), 1)
        self.assertEqual(rows[0]["claimed_at"], "2026-01-01 09:30:00")
        self.assertEqual(rows[0]["note"], "续期")

    def test_expired_claims_disappear_from_overview(self):
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            store.claim_files(self.project, "claude", ["a.py"], 1)
        with patch.object(store, "now", return_value="2026-01-01 09:01:01"):
            self.assertEqual(store.overview(self.project, "codex")["claims"], [])

    def test_claim_purges_expired_conflicts_without_overview(self):
        with patch.object(store, "now", return_value="2026-01-01 09:00:00"):
            store.claim_files(self.project, "claude", ["a.py"], 1)
        with patch.object(store, "now", return_value="2026-01-01 09:01:01"):
            result = store.claim_files(self.project, "codex", ["a.py"], 10)
            self.assertEqual(result["conflicts"], [])
            self.assertEqual(store.overview(self.project, "codex")["claims"][0]["agent"], "codex")

    def test_only_owner_can_release_named_claims(self):
        store.claim_files(self.project, "claude", ["a.py"], 60)
        store.claim_files(self.project, "codex", ["b.py"], 60)
        self.assertEqual(store.release_files(self.project, "codex", ["a.py", "b.py"]), {"count": 1})
        self.assertEqual([r["path"] for r in store.overview(self.project, "codex")["claims"]], ["a.py"])

    def test_tool_omitting_files_releases_all_owned_claims(self):
        project = tools.norm_project(self.project)
        store.claim_files(project, "claude", ["a.py"], 60)
        store.claim_files(project, "codex", ["b.py", "c.py"], 60)
        with patch.dict(os.environ, {"BRIDGE_AGENT": "codex"}):
            self.assertEqual(tools.t_release_files({"project": self.project}), "已释放 2 个文件。")
        remaining = store.overview(project, "claude")["claims"]
        self.assertEqual([(r["path"], r["agent"]) for r in remaining], [("a.py", "claude")])
