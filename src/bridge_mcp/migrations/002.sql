CREATE TABLE IF NOT EXISTS events (
    id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT NOT NULL, agent TEXT NOT NULL,
    kind TEXT NOT NULL, detail TEXT NOT NULL, created_at TEXT NOT NULL);
CREATE INDEX IF NOT EXISTS events_project_id ON events(project, id);

INSERT INTO events(project, agent, kind, detail, created_at)
    SELECT COALESCE(project, ''), COALESCE(sender, ''), 'message',
        json_object('recipient', recipient, 'content', content), COALESCE(created_at, '')
    FROM messages WHERE NOT EXISTS (SELECT 1 FROM events WHERE kind = 'message') ORDER BY id;
INSERT INTO events(project, agent, kind, detail, created_at)
    SELECT COALESCE(project, ''), COALESCE(agent, ''), 'status',
        json_object('task', task, 'progress', progress, 'blockers', blockers, 'next_step', next_step),
        COALESCE(updated_at, '')
    FROM status WHERE NOT EXISTS (SELECT 1 FROM events WHERE kind = 'status') ORDER BY project, agent;

CREATE TRIGGER IF NOT EXISTS history_status AFTER INSERT ON status BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        COALESCE(NEW.project, ''), COALESCE(NEW.agent, ''), 'status',
        json_object('task', NEW.task, 'progress', NEW.progress, 'blockers', NEW.blockers, 'next_step', NEW.next_step),
        COALESCE(NEW.updated_at, ''));
END;
CREATE TRIGGER IF NOT EXISTS history_message AFTER INSERT ON messages BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        COALESCE(NEW.project, ''), COALESCE(NEW.sender, ''), 'message',
        json_object('recipient', NEW.recipient, 'content', NEW.content), COALESCE(NEW.created_at, ''));
END;
CREATE TRIGGER IF NOT EXISTS history_claim AFTER INSERT ON claims BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        COALESCE(NEW.project, ''), COALESCE(NEW.agent, ''), 'claim',
        json_object('path', NEW.path, 'note', NEW.note, 'expires_at', NEW.expires_at), COALESCE(NEW.claimed_at, ''));
END;
CREATE TRIGGER IF NOT EXISTS history_release AFTER DELETE ON claims BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        COALESCE(OLD.project, ''), COALESCE(OLD.agent, ''),
        CASE WHEN OLD.expires_at <= datetime('now', 'localtime') THEN 'expire' ELSE 'release' END,
        json_object('path', OLD.path), datetime('now', 'localtime'));
END;
CREATE TRIGGER IF NOT EXISTS history_switch AFTER INSERT ON settings BEGIN
    INSERT INTO events(project, agent, kind, detail, created_at) VALUES (
        NEW.scope, 'human', 'switch', json_object('scope', NEW.scope, 'enabled', NEW.enabled),
        datetime('now', 'localtime'));
END;
