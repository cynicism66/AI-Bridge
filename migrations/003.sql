CREATE TABLE sessions (
    id TEXT NOT NULL, project TEXT NOT NULL, agent TEXT NOT NULL, session_no INTEGER NOT NULL,
    worktree TEXT NOT NULL, branch TEXT NOT NULL, pid INTEGER NOT NULL,
    started_at TEXT NOT NULL, last_active TEXT NOT NULL, PRIMARY KEY (id, project));
CREATE INDEX sessions_active ON sessions(project, agent, last_active);
INSERT INTO sessions
    SELECT 'legacy:' || project || ':' || agent, project, agent, 1, project, '-', 0, MIN(t), MAX(t)
    FROM (SELECT project, agent, updated_at AS t FROM status
          UNION ALL SELECT project, agent, claimed_at FROM claims) GROUP BY project, agent;
DROP TRIGGER IF EXISTS history_status;
CREATE TABLE status_new (
    project TEXT, agent TEXT, task TEXT, progress TEXT, blockers TEXT, next_step TEXT,
    updated_at TEXT, session_no INTEGER NOT NULL DEFAULT 1, session_id TEXT NOT NULL DEFAULT '',
    PRIMARY KEY (project, agent, session_no));
INSERT INTO status_new SELECT *, 1, 'legacy:' || project || ':' || agent FROM status;
DROP TABLE status;
ALTER TABLE status_new RENAME TO status;
CREATE TRIGGER history_status AFTER INSERT ON status BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        COALESCE(NEW.project, ''), COALESCE(NEW.agent, ''), 'status',
        json_object('task', NEW.task, 'progress', NEW.progress, 'blockers', NEW.blockers,
                    'next_step', NEW.next_step, 'session_no', NEW.session_no), COALESCE(NEW.updated_at, ''));
END;
ALTER TABLE claims ADD COLUMN session_no INTEGER NOT NULL DEFAULT 1;
ALTER TABLE claims ADD COLUMN session_id TEXT NOT NULL DEFAULT '';
UPDATE claims SET session_id = 'legacy:' || project || ':' || agent;
CREATE TABLE agent_settings (
    project TEXT NOT NULL, agent TEXT NOT NULL, enabled INTEGER NOT NULL,
    PRIMARY KEY (project, agent));
CREATE TRIGGER history_agent_switch AFTER INSERT ON agent_settings BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        NEW.project, 'human', 'agent_switch', json_object('agent', NEW.agent, 'enabled', NEW.enabled),
        datetime('now', 'localtime'));
END;
CREATE TRIGGER history_agent_switch_update AFTER UPDATE OF enabled ON agent_settings
WHEN OLD.enabled != NEW.enabled BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        NEW.project, 'human', 'agent_switch', json_object('agent', NEW.agent, 'enabled', NEW.enabled),
        datetime('now', 'localtime'));
END;
