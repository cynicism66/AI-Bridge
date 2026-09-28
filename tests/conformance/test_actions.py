import sqlite3
from contextlib import closing

from .support import ContractCase, ROOT


class ActionTests(ContractCase):
    def test_needs_action_validation_read_and_reply(self):
        self.enable()
        server = self.session()
        for recipient in ('all', 'claude', 'codex'):
            result = server.call('send_message', project=self.project, to=recipient,
                                 content='请确认', needs_action=True)
            self.assertTrue(result.get('isError'))
        self.assertTrue(server.call('send_message', project=self.project, to='human',
                                   content='请确认', needs_action='true').get('isError'))
        result = server.call('send_message', project=self.project, to='human', content='请确认', needs_action=True)
        self.assertFalse(result.get('isError', False))
        self.cli('read', self.project)
        with closing(sqlite3.connect(self.database)) as db, db:
            row = db.execute('SELECT id,actioned_at FROM messages WHERE needs_action=1').fetchone()
            self.assertIsNone(row[1])
        # Later timestamp, including a reply through the real human CLI.
        self.env['BRIDGE_FAKE_NOW'] = '2090-01-02 03:04:06'
        self.cli('post', self.project, '已经确认', '--to', 'codex')
        with closing(sqlite3.connect(self.database)) as db, db:
            self.assertIsNotNone(db.execute('SELECT actioned_at FROM messages WHERE id=?', (row[0],)).fetchone()[0])
            self.assertEqual(db.execute("SELECT count(*) FROM events WHERE kind='action_done'").fetchone()[0], 1)

    def test_blocker_same_content_ack_and_changed_generation(self):
        self.enable()
        server = self.session()
        server.call('update_status', project=self.project, task='测试', blockers='请决定')
        with closing(sqlite3.connect(self.database)) as db, db:
            old_id = db.execute('SELECT id FROM action_blockers').fetchone()[0]
            db.execute("UPDATE action_blockers SET acknowledged_at='2090-01-02 03:04:06' WHERE id=?", (old_id,))
        server.call('update_status', project=self.project, task='测试', blockers='请决定')
        with closing(sqlite3.connect(self.database)) as db, db:
            self.assertIsNotNone(db.execute('SELECT acknowledged_at FROM action_blockers').fetchone()[0])
        server.call('update_status', project=self.project, task='测试', blockers='另一决定')
        with closing(sqlite3.connect(self.database)) as db, db:
            new = db.execute('SELECT id,acknowledged_at FROM action_blockers').fetchone()
            self.assertNotEqual(old_id, new[0])
            self.assertIsNone(new[1])
        server.call('update_status', project=self.project, task='测试', blockers='')
        with closing(sqlite3.connect(self.database)) as db, db:
            self.assertEqual(db.execute('SELECT count(*) FROM action_blockers').fetchone()[0], 0)
            self.assertEqual(db.execute("SELECT count(*) FROM events WHERE kind='blocker_ack'").fetchone()[0], 1)

    def test_v7_migration_preserves_existing_data(self):
        with closing(sqlite3.connect(self.database)) as db, db:
            for n in range(1, 8):
                db.executescript((ROOT / 'migrations' / f'{n:03d}.sql').read_text(encoding='utf-8'))
            db.execute('PRAGMA user_version=7')
            db.execute("INSERT INTO messages(project,sender,recipient,content,created_at) VALUES (?,'codex','human','保留','2090-01-01 00:00:00')", (self.project,))
        self.cli('show', self.project)
        with closing(sqlite3.connect(self.database)) as db, db:
            self.assertEqual(db.execute('PRAGMA user_version').fetchone()[0], 8)
            self.assertEqual(db.execute('SELECT content,needs_action,actioned_at FROM messages').fetchone(), ('保留', 0, None))
