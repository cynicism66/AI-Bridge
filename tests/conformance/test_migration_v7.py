from contextlib import closing
import sqlite3

from .support import ContractCase, ROOT, STAMP
from . import test_wait


class MigrationV7Tests(ContractCase):
    def fixture_v6(self):
        with closing(sqlite3.connect(self.database)) as db:
            for n in range(1, 7):
                db.executescript((ROOT / f'migrations/{n:03}.sql').read_text(encoding='utf-8'))
            db.execute('PRAGMA user_version=6')
            db.execute("INSERT INTO settings VALUES(?,1)", (self.project,))
            db.execute("INSERT INTO messages VALUES(1,?,'human','all','历史消息',?,'gui')", (self.project, STAMP))
            db.execute("INSERT INTO reads VALUES(1,'codex')")
            db.commit()

    def test_v6_wait_does_not_migrate_or_write(self):
        self.fixture_v6()
        before = test_wait.WaitTests.snapshot(self)
        result = test_wait.WaitTests.run_wait(self, self.project, '--agent', 'codex', '--timeout', '0')
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(test_wait.WaitTests.snapshot(self), before)

    def test_migration_preserves_old_read_and_first_new_timestamp(self):
        self.fixture_v6()
        self.cli('status')
        self.cli('status')
        self.initialize()
        server = self.session('claude')
        server.call('read_messages', project=self.project)
        later = self.session('claude', clock='2091-01-01 01:01:01')
        later.call('read_messages', project=self.project)
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute('PRAGMA user_version').fetchone()[0], 7)
            self.assertEqual(db.execute('SELECT agent,read_at FROM reads ORDER BY agent').fetchall(), [('claude', STAMP), ('codex', None)])
            self.assertEqual(db.execute('PRAGMA integrity_check').fetchone()[0], 'ok')
