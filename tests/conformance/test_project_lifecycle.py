import json
import sqlite3
from contextlib import closing
from .support import ContractCase
from .test_sessions import content


class ProjectLifecycleTests(ContractCase):
    def setUp(self):
        super().setUp()
        self.home = self.directory / 'home'
        self.home.mkdir()
        self.env['BRIDGE_APP_HOME'] = str(self.home)
        self.settings = self.home / '.bridge' / 'app.json'

    def test_forget_disables_hides_and_retains_data_and_files(self):
        self.enable()
        marker = self.directory / 'keep.txt'
        marker.write_text('保留内容', encoding='utf-8')
        server = self.session()
        server.call('update_status', project=self.project, task='保留状态')
        self.cli('post', self.project, '保留消息')
        with closing(sqlite3.connect(self.database)) as db:
            before = db.execute('SELECT * FROM messages').fetchall()
        result = self.cli('forget', self.project)
        self.assertIn('已从列表移除', result)
        self.assertIn('数据保留', result)
        self.assertIn(self.project, json.loads(self.settings.read_text(encoding='utf-8'))['hidden_projects'])
        self.assertIn('从列表移除项目', self.cli('history', self.project, '--kind', 'forget'))
        self.assertIn('未在此项目开启', content(server.call('bridge_overview', project=self.project)))
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute('SELECT * FROM messages').fetchall(), before)
            self.assertEqual(db.execute('SELECT enabled FROM settings WHERE scope=?', (self.project,)).fetchone(), (0,))
        self.assertEqual(marker.read_text(encoding='utf-8'), '保留内容')

    def test_purge_requires_yes_even_with_force(self):
        self.enable()
        for flags in [[], ['--force']]:
            error = self.cli('purge', self.project, *flags, ok=False)
            self.assertIn('--yes', error)
            self.assertIn('export', error)
        self.assertIn('结对流程', self.cli('show', self.project))

    def test_purge_active_sessions_require_force_and_no_residual_events(self):
        self.enable()
        server = self.session()
        server.call('update_status', project=self.project, task='运行中')
        server.call('claim_files', project=self.project, files=['test.txt'])
        marker = self.directory / 'AGENTS.md'
        marker.write_text('项目文件不能改变', encoding='utf-8')
        error = self.cli('purge', self.project, '--yes', ok=False)
        self.assertIn('有 AI 正在这个项目里工作', error)
        self.assertIn('--force', error)
        result = self.cli('purge', self.project, '--yes', '--force')
        self.assertIn('已彻底删除', result)
        self.assertIn('Claude/Codex 配置和项目文件未改变', result)
        self.assertIn(self.project, json.loads(self.settings.read_text(encoding='utf-8'))['hidden_projects'])
        self.assertEqual(marker.read_text(encoding='utf-8'), '项目文件不能改变')
        self.assertIn('未在此项目开启', content(server.call('bridge_overview', project=self.project)))
        with closing(sqlite3.connect(self.database)) as db:
            for table in ['status', 'sessions', 'messages', 'reads', 'claims', 'agent_settings', 'project_init', 'role_assignments', 'permission_overrides', 'events']:
                self.assertEqual(db.execute(f'SELECT COUNT(*) FROM {table}').fetchone(), (0,), table)
            self.assertEqual(db.execute('SELECT COUNT(*) FROM settings WHERE scope=?', (self.project,)).fetchone(), (0,))
            self.assertEqual(db.execute('PRAGMA user_version').fetchone(), (6,))

    def test_forget_from_current_directory_and_invalid_settings_rolls_back(self):
        self.enable()
        self.settings.parent.mkdir(parents=True)
        self.settings.write_text('{invalid', encoding='utf-8')
        self.cli('forget', cwd=self.directory, ok=False)
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute('SELECT enabled FROM settings WHERE scope=?', (self.project,)).fetchone(), (1,))
        self.settings.write_text('{}', encoding='utf-8')
        self.assertIn('已从列表移除', self.cli('forget', cwd=self.directory))
