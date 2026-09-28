CREATE TABLE project_init (
    project TEXT PRIMARY KEY, template TEXT NOT NULL, template_json TEXT NOT NULL,
    goal TEXT NOT NULL, charter TEXT NOT NULL, version INTEGER NOT NULL,
    initialized_at TEXT NOT NULL, roles_json TEXT NOT NULL,
    write_rules INTEGER NOT NULL DEFAULT 0);
CREATE TABLE role_assignments (
    project TEXT NOT NULL, agent TEXT NOT NULL, slot TEXT NOT NULL,
    PRIMARY KEY (project, agent));
ALTER TABLE sessions ADD COLUMN charter_version INTEGER NOT NULL DEFAULT 0;
CREATE TRIGGER history_init AFTER INSERT ON project_init BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES (
        NEW.project,'human','init',json_object('template',NEW.template,'goal',NEW.goal,
        'roles',json(NEW.roles_json)),NEW.initialized_at);
END;
CREATE TRIGGER history_reinit AFTER UPDATE OF initialized_at ON project_init BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES (
        NEW.project,'human','init',json_object('template',NEW.template,'goal',NEW.goal,
        'roles',json(NEW.roles_json)),NEW.initialized_at);
END;
CREATE TRIGGER history_role AFTER UPDATE OF slot ON role_assignments
WHEN OLD.slot != NEW.slot BEGIN
    INSERT INTO events(project,agent,kind,detail,created_at) VALUES (
        NEW.project,'human','role',json_object('agent',NEW.agent,'slot',NEW.slot),
        datetime('now','localtime'));
END;
