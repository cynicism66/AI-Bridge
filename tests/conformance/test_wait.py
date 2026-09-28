"""真实子进程等待：仅使用临时数据库，输出读取有界且在退出前验证刷新。"""
from contextlib import closing
import queue
import sqlite3
import subprocess
import threading
import time

from .support import COMMAND, ROOT, ContractCase, STAMP


class WaitTests(ContractCase):
    def run_wait(self, *args, cwd=None):
        return subprocess.run([*COMMAND, 'wait', *args], cwd=cwd or ROOT, env=self.env,
                              capture_output=True, timeout=15)

    def start_wait(self, *args):
        p = subprocess.Popen([*COMMAND, 'wait', self.project, '--agent', 'codex',
                              '--interval', '1', *args], cwd=ROOT, env=self.env,
                             stdout=subprocess.PIPE, stderr=subprocess.PIPE)
        lines = queue.Queue()

        def read():
            for line in p.stdout:
                lines.put(line.decode('utf-8').strip())

        reader = threading.Thread(target=read, daemon=True)
        reader.start()

        def close():
            if p.poll() is None:
                p.kill()
            p.wait(timeout=10)
            reader.join(timeout=10)
            p.stdout.close()
            p.stderr.close()

        self.addCleanup(close)
        return p, lines

    def insert(self, sender='human', recipient='codex', content='新消息\r\n第二行', via='gui'):
        with closing(sqlite3.connect(self.database)) as db:
            cursor = db.execute('INSERT INTO messages (project,sender,recipient,content,created_at,via) VALUES(?,?,?,?,?,?)',
                                (self.project, sender, recipient, content, STAMP, via))
            db.commit()
            return cursor.lastrowid

    def wake(self, lines):
        # Repeated bounded attempts tolerate a cold CI process startup without a test-only CLI flag.
        for _ in range(4):
            ident = self.insert()
            try:
                return lines.get(timeout=1.5), ident
            except queue.Empty:
                pass
        self.fail('等待命令未在期限内输出新消息')

    def snapshot(self):
        with closing(sqlite3.connect(self.database)) as db:
            return list(db.iterdump())

    def test_new_message_exits_zero_flushes_and_does_not_mark_read(self):
        self.enable()
        p, lines = self.start_wait('--timeout', '10')
        text, _ = self.wake(lines)
        self.assertRegex(text, r'^新消息 #\d+ human（软件界面） → codex：新消息 第二行$')
        self.assertEqual(p.wait(timeout=5), 0)
        self.assertEqual(p.stderr.read(), b'')
        with closing(sqlite3.connect(self.database)) as db:
            self.assertEqual(db.execute('SELECT count(*) FROM reads').fetchone()[0], 0)
            self.assertEqual(db.execute('SELECT count(*) FROM sessions').fetchone()[0], 0)

    def test_old_messages_timeout_two_readonly_and_cwd_git_root(self):
        self.enable()
        self.insert()
        (self.directory / '.git').mkdir()
        child = self.directory / 'nested'
        child.mkdir()
        before = self.snapshot()
        started = time.monotonic()
        result = self.run_wait('--agent', 'codex', '--timeout', '1', '--interval', '30', cwd=child)
        self.assertEqual(result.returncode, 2, result.stderr)
        self.assertEqual(result.stdout.decode('utf-8').strip(), '等待超时')
        self.assertLess(time.monotonic() - started, 8)
        self.assertEqual(self.snapshot(), before)

    def test_follow_multiple_batches_no_duplicates_self_or_wrong_recipient(self):
        self.enable()
        p, lines = self.start_wait('--follow', '--timeout', '7')
        first, _ = self.wake(lines)
        self.assertIsNone(p.poll(), '输出必须在进程结束之前刷新')
        self.insert(sender='codex', recipient='all', content='自己')
        self.insert(recipient='claude', content='给别人')
        wanted = self.insert(recipient='all', content='命令行广播', via='cli')
        observed = [first]
        while True:
            line = lines.get(timeout=10)
            if line == '等待超时':
                break
            observed.append(line)
        self.assertEqual(p.wait(timeout=5), 2)
        self.assertEqual(p.stderr.read(), b'')
        self.assertIn(f'新消息 #{wanted} human（命令行） → 所有人：命令行广播', observed)
        ids = [int(line.split('#')[1].split()[0]) for line in observed]
        self.assertEqual(ids, sorted(set(ids)))
        self.assertFalse(any('自己' in line or '给别人' in line for line in observed))

    def test_three_switches_exit_three_and_missing_database_not_created(self):
        result = self.run_wait(self.project, '--agent', 'codex')
        self.assertEqual(result.returncode, 3)
        self.assertIn('项目', result.stdout.decode('utf-8'))
        self.assertFalse(self.database.exists())
        for args, reason in [(('off', self.project), '项目'),
                             (('off', '--global'), '全局'),
                             (('agent', self.project, 'codex', 'off'), 'AI')]:
            self.cli('on', '--global')
            self.cli('on', self.project)
            self.cli('agent', self.project, 'codex', 'on')
            self.cli(*args)
            before = self.snapshot()
            result = self.run_wait(self.project, '--agent', 'codex')
            self.assertEqual(result.returncode, 3, result.stderr)
            self.assertIn(reason, result.stdout.decode('utf-8'))
            self.assertEqual(self.snapshot(), before)

    def test_running_wait_stops_when_agent_disabled(self):
        self.enable()
        p, lines = self.start_wait('--follow', '--timeout', '10')
        self.wake(lines)
        self.cli('agent', self.project, 'codex', 'off')
        self.assertEqual(p.wait(timeout=5), 3)
        self.assertEqual(p.stderr.read(), b'')
        remaining = []
        while not lines.empty():
            remaining.append(lines.get_nowait())
        self.assertTrue(any('AI' in line and '关闭' in line for line in remaining))

    def test_interval_zero_rejected(self):
        result = self.run_wait(self.project, '--agent', 'codex', '--interval', '0')
        self.assertEqual(result.returncode, 2)
        self.assertIn(b'--interval', result.stderr)
