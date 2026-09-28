import json
import sqlite3
from contextlib import closing
from .support import ContractCase, ROOT, STAMP
from .test_sessions import content


class PermissionV6Tests(ContractCase):
    def test_permission_override_reset_and_charter_claims(self):
        self.enable()
        server = self.session('claude')
        server.call('bridge_overview', project=self.project)
        self.cli('permission', self.project, '开发甲', '--allow', '只写文档', '--write', 'docs/**')
        board = content(server.call('bridge_overview', project=self.project))
        self.assertIn('== 协作章程 v2 ==', board)
        self.assertIn('只写文档', board)
        self.assertIn('超出', content(server.call('claim_files', project=self.project, files=['src/a.rs'])))
        self.assertIn('已认领', content(server.call('claim_files', project=self.project, files=['docs/a.md'])))
        self.cli('permission', self.project, '开发甲', '--write', '../escape', ok=False)
        self.assertIn('本会话已显示过', content(server.call('bridge_overview', project=self.project)))
        self.cli('permission', self.project, '开发甲', '--reset')
        self.assertIn('== 协作章程 v3 ==', content(server.call('bridge_overview', project=self.project)))
        self.assertIn('已认领', content(server.call('claim_files', project=self.project, files=['src/a.rs'])))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute('SELECT count(*) FROM permission_overrides').fetchone()[0], 0)
            self.assertEqual(db.execute("SELECT count(*) FROM events WHERE kind='permission'").fetchone()[0], 2)
        self.assertIn('重置职务', self.cli('history', self.project, '--kind', 'permission'))

    def test_v5_migration_preserves_data_and_removes_reused_owners(self):
        with closing(sqlite3.connect(self.database)) as db:
            for n in range(1, 6):
                db.executescript((ROOT / f'migrations/{n:03}.sql').read_text(encoding='utf-8'))
            db.execute('PRAGMA user_version=5')
            for ident, active in [('legacy', '2080-01-01 00:00:00'), ('latest', STAMP)]:
                db.execute('INSERT INTO sessions VALUES(?,?,?,2,?,?,0,?,?,0)', (ident,self.project,'claude',self.project,'main',active,active))
            db.execute('INSERT INTO status VALUES(?,?,?,?,?,?,?,2,?)', (self.project,'claude','stale','','','',STAMP,'legacy'))
            db.execute("INSERT INTO messages VALUES(1,?,'human','all','preserved',?,'gui')",(self.project,STAMP))
            db.execute("INSERT INTO reads VALUES(1,'claude')")
            db.commit()
            events = db.execute('SELECT * FROM events').fetchall()
        self.cli('status')
        self.cli('status')
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute('PRAGMA user_version').fetchone()[0],6)
            self.assertEqual(db.execute('SELECT id FROM sessions').fetchall(),[('latest',)])
            self.assertEqual(db.execute('SELECT count(*) FROM status').fetchone()[0],0)
            self.assertEqual(db.execute('SELECT * FROM events').fetchall(),events)
            self.assertEqual(db.execute('SELECT via,content FROM messages').fetchall(),[('gui','preserved')])
            self.assertEqual(db.execute('SELECT * FROM reads').fetchall(),[(1,'claude')])
            self.assertEqual(db.execute('PRAGMA integrity_check').fetchone()[0],'ok')

    def test_legacy_and_expired_number_reuse_has_current_status(self):
        self.enable()
        active = self.session('claude')
        active.call('update_status',project=self.project,task='active one')
        old = self.session('claude',clock='2080-01-01 00:00:00')
        old.call('update_status',project=self.project,task='stale two')
        with closing(sqlite3.connect(self.database)) as db:
            db.execute("INSERT INTO sessions VALUES('legacy-extra',?,'claude',2,?,'-',0,'2070-01-01 00:00:00','2070-01-01 00:00:00',0)",(self.project,self.project))
            db.commit()
        current = self.session('claude')
        current.call('update_status',project=self.project,task='current two')
        board = content(current.call('bridge_overview',project=self.project))
        self.assertEqual(board.count('【claude #1】'),1)
        self.assertEqual(board.count('【claude #2】'),1)
        self.assertIn('任务：current two',board.split('【claude #2】')[1])
        self.assertNotIn('stale two',board)
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute("SELECT session_no,count(*) FROM sessions WHERE agent='claude' GROUP BY session_no").fetchall(),[(1,1),(2,1)])

    def test_permission_empty_list_and_unmentioned_lists(self):
        self.enable()
        self.cli('permission',self.project,'开发甲','--allow','first','--allow','second','--write','docs/**')
        self.cli('permission',self.project,'开发甲','--deny','danger')
        self.cli('permission',self.project,'开发甲','--write','')
        with closing(sqlite3.connect(self.database)) as db:
            a,d,w=db.execute('SELECT allow_json,deny_json,write_json FROM permission_overrides').fetchone()
            self.assertEqual(json.loads(a),['first','second'])
            self.assertEqual(json.loads(d),['danger'])
            self.assertEqual(json.loads(w),[])
