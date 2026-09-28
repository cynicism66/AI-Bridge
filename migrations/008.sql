ALTER TABLE messages ADD COLUMN needs_action INTEGER NOT NULL DEFAULT 0 CHECK(needs_action IN (0,1) AND (needs_action=0 OR recipient='human'));
ALTER TABLE messages ADD COLUMN actioned_at TEXT;
CREATE TABLE action_blockers (
    id INTEGER PRIMARY KEY AUTOINCREMENT,
    project TEXT NOT NULL, agent TEXT NOT NULL, session_id TEXT NOT NULL,
    content TEXT NOT NULL, acknowledged_at TEXT,
    UNIQUE(project,agent,session_id));
INSERT INTO action_blockers(project,agent,session_id,content)
    SELECT project,agent,session_id,blockers FROM status WHERE trim(COALESCE(blockers,''))!='';
CREATE TRIGGER action_blocker_insert AFTER INSERT ON status BEGIN
    DELETE FROM action_blockers WHERE project=NEW.project AND agent=NEW.agent AND session_id=NEW.session_id AND content!=COALESCE(NEW.blockers,'');
    INSERT INTO action_blockers(project,agent,session_id,content)
      SELECT NEW.project,NEW.agent,NEW.session_id,NEW.blockers WHERE trim(COALESCE(NEW.blockers,''))!='' AND NOT EXISTS (SELECT 1 FROM action_blockers WHERE project=NEW.project AND agent=NEW.agent AND session_id=NEW.session_id);
END;
CREATE TRIGGER action_blocker_update AFTER UPDATE OF blockers ON status BEGIN
    DELETE FROM action_blockers WHERE project=NEW.project AND agent=NEW.agent AND session_id=NEW.session_id AND content!=COALESCE(NEW.blockers,'');
    INSERT INTO action_blockers(project,agent,session_id,content)
      SELECT NEW.project,NEW.agent,NEW.session_id,NEW.blockers WHERE trim(COALESCE(NEW.blockers,''))!='' AND NOT EXISTS (SELECT 1 FROM action_blockers WHERE project=NEW.project AND agent=NEW.agent AND session_id=NEW.session_id);
END;
CREATE TRIGGER action_blocker_delete AFTER DELETE ON status BEGIN
    DELETE FROM action_blockers WHERE project=OLD.project AND agent=OLD.agent AND session_id=OLD.session_id;
END;
CREATE TRIGGER action_human_reply AFTER INSERT ON messages WHEN NEW.sender='human' BEGIN
    UPDATE messages SET actioned_at=NEW.created_at WHERE project=NEW.project AND needs_action=1
      AND actioned_at IS NULL AND created_at<NEW.created_at AND (sender=NEW.recipient OR NEW.recipient='all');
END;
CREATE TRIGGER action_message_done AFTER UPDATE OF actioned_at ON messages
WHEN OLD.actioned_at IS NULL AND NEW.actioned_at IS NOT NULL BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES(NEW.project,'human','action_done',json_object('message_id',NEW.id,'sender',NEW.sender),NEW.actioned_at);
END;
CREATE TRIGGER action_blocker_ack AFTER UPDATE OF acknowledged_at ON action_blockers
WHEN OLD.acknowledged_at IS NULL AND NEW.acknowledged_at IS NOT NULL BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES(NEW.project,'human','blocker_ack',json_object('agent',NEW.agent,'session_id',NEW.session_id,'blockers',NEW.content),NEW.acknowledged_at);
END;
DROP TRIGGER history_message;
CREATE TRIGGER history_message AFTER INSERT ON messages BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES (
        NEW.project,NEW.sender,'message',json_object('message_id',NEW.id,
        'recipient',NEW.recipient,'content',NEW.content,'via',NEW.via,'needs_action',json(CASE WHEN NEW.needs_action THEN 'true' ELSE 'false' END)),NEW.created_at);
END;
