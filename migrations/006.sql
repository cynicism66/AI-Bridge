CREATE TABLE permission_overrides (
    project TEXT NOT NULL, slot TEXT NOT NULL,
    allow_json TEXT NOT NULL, deny_json TEXT NOT NULL, write_json TEXT NOT NULL,
    PRIMARY KEY (project, slot));
-- 删除复用编号的旧拥有者；保留最新会话，历史 events 不动。
DELETE FROM status WHERE EXISTS (
    SELECT 1 FROM sessions newer WHERE newer.project=status.project
    AND newer.agent=status.agent AND newer.session_no=status.session_no
    AND newer.id!=status.session_id AND newer.rowid=(
        SELECT rowid FROM sessions WHERE project=status.project AND agent=status.agent
        AND session_no=status.session_no ORDER BY last_active DESC,started_at DESC,rowid DESC LIMIT 1));
DELETE FROM sessions WHERE rowid NOT IN (
    SELECT rowid FROM (SELECT rowid,ROW_NUMBER() OVER (
        PARTITION BY project,agent,session_no ORDER BY last_active DESC,started_at DESC,rowid DESC) AS n
        FROM sessions) WHERE n=1);
