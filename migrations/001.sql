CREATE TABLE IF NOT EXISTS status (
    project TEXT, agent TEXT, task TEXT, progress TEXT, blockers TEXT, next_step TEXT,
    updated_at TEXT, PRIMARY KEY (project, agent));
CREATE TABLE IF NOT EXISTS messages (
    id INTEGER PRIMARY KEY AUTOINCREMENT, project TEXT, sender TEXT, recipient TEXT,
    content TEXT, created_at TEXT);
CREATE TABLE IF NOT EXISTS reads (
    message_id INTEGER, agent TEXT, PRIMARY KEY (message_id, agent));
CREATE TABLE IF NOT EXISTS claims (
    project TEXT, path TEXT, agent TEXT, note TEXT, claimed_at TEXT, expires_at TEXT,
    PRIMARY KEY (project, path));
CREATE TABLE IF NOT EXISTS settings (
    scope TEXT PRIMARY KEY, enabled INTEGER NOT NULL);
