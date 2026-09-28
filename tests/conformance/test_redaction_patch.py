from .transfer_support import TransferCase


SAMPLES = [
    ("GITHUB_TOKEN=github-value", "github-value"),
    ("OPENAI_API_KEY=openai-value", "openai-value"),
    ("AWS_SECRET_ACCESS_KEY=aws-value", "aws-value"),
    ("DB_PASSWORD=db-value", "db-value"),
    ("client_secret: client-value", "client-value"),
    ('"access_token": "access-value"', "access-value"),
    ("export ANTHROPIC_API_KEY=anthropic-value", "anthropic-value"),
    ('$env:HF_TOKEN = "hf-value"', "hf-value"),
    ('DB_PASSWORD="a b c"', "a b c"),
]
ORDINARY = [
    "这个 token 很重要", "password 规则要写清楚", "label=token", "description: password",
    '"label": "client_secret: sample"', "value='DB_PASSWORD=example'", "value=auth=example",
]


class RedactionPatchTests(TransferCase):
    def test_export_masks_all_review_samples_in_messages_status_history_and_preview(self):
        self.enable()
        server = self.session()
        body = "\n".join([sample for sample, _ in SAMPLES] + ORDINARY)
        server.call("send_message", project=self.project, to="claude", content=body)
        server.call("update_status", project=self.project, task="补丁验证", progress=body)
        before = self.snapshot()
        stdout, _, preview = self.transfer("export", self.project, "--yes")
        result = (self.directory / "docs/bridge/协作记录.md").read_text(encoding="utf-8")
        for sample, secret in SAMPLES:
            with self.subTest(sample=sample):
                self.assertNotIn(secret, result + preview + stdout)
                self.assertIn(sample.replace(secret, "[已打码：赋值凭据]"), result)
                self.assertIn(sample.replace(secret, "[已打码：赋值凭据]"), preview)
        for sentence in ORDINARY:
            self.assertIn(sentence, result)
            self.assertIn(sentence, preview)
        self.assertIn("共打码 36 处：赋值凭据 36", stdout)
        self.assertEqual(self.snapshot(), before)
        self.assertNotIn("\\\\?\\", stdout + preview)

    def test_handover_masks_review_samples_and_displays_usable_paths(self):
        self.enable()
        server = self.session()
        for sample, _ in SAMPLES:
            server.call("send_message", project=self.project, to="claude", content=sample)
        stdout, _, preview = self.transfer("handover", self.project, "--to", "codex", "--yes")
        result = (self.directory / "docs/HANDOVER.md").read_text(encoding="utf-8")
        for sample, secret in SAMPLES:
            with self.subTest(sample=sample):
                self.assertNotIn(secret, result + preview + stdout)
                self.assertIn(sample.replace(secret, "[已打码：赋值凭据]"), result)
                self.assertIn(sample.replace(secret, "[已打码：赋值凭据]"), preview)
        self.assertNotIn("\\\\?\\", stdout + preview)
        self.assertIn("交接完成", stdout)
