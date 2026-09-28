ALTER TABLE messages ADD COLUMN via TEXT NOT NULL DEFAULT 'mcp'
    CHECK (via IN ('mcp', 'cli', 'gui'));
DROP TRIGGER history_message;
CREATE TRIGGER history_message AFTER INSERT ON messages BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES (
        NEW.project,NEW.sender,'message',json_object('message_id',NEW.id,
        'recipient',NEW.recipient,'content',NEW.content,'via',NEW.via),NEW.created_at);
END;
