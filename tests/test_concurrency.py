"""两个协作身份同时写入同一 SQLite 数据库。"""

from concurrent.futures import ThreadPoolExecutor
from threading import Barrier

from tests.support import DatabaseTestCase
from bridge_mcp import store


class ConcurrencyTests(DatabaseTestCase):
    def test_two_agents_each_write_fifty_statuses_and_messages(self):
        start = Barrier(2)

        def write(agent):
            start.wait(timeout=5)
            for index in range(50):
                store.update_status(self.project, agent, task=f"任务 {index}")
                store.send_message(self.project, agent, "all", f"{agent} 消息 {index}")

        # 身份显式传入数据层，避免在线程间修改共享环境变量。
        with ThreadPoolExecutor(max_workers=2) as executor:
            futures = [executor.submit(write, agent) for agent in ("claude", "codex")]
            for future in futures:
                future.result(timeout=10)

        with store.db() as conn:
            statuses = [dict(row) for row in conn.execute("SELECT * FROM status ORDER BY agent")]
            messages = [dict(row) for row in conn.execute("SELECT * FROM messages ORDER BY id")]
        self.assertEqual([(row["agent"], row["task"]) for row in statuses],
                         [("claude", "任务 49"), ("codex", "任务 49")])
        self.assertEqual(len(messages), 100)
        self.assertEqual(len({row["id"] for row in messages}), 100)
        for agent in ("claude", "codex"):
            self.assertEqual([row["content"] for row in messages if row["sender"] == agent],
                             [f"{agent} 消息 {index}" for index in range(50)])
