-- 精简版只新增收件窗口；旧表、旧消息、旧已读和历史数据保持不变。
CREATE TABLE hook_receivers (
    project TEXT NOT NULL,
    agent TEXT NOT NULL CHECK(agent IN ('claude','codex')),
    session_id TEXT,
    last_active TEXT,
    pending_rebind INTEGER NOT NULL DEFAULT 0 CHECK(pending_rebind IN (0,1)),
    PRIMARY KEY(project,agent)
);
