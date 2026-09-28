use crate::{database::query, now, Bridge};
use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::Value;

pub(crate) fn unread(conn: &Connection, project: &str, agent: &str) -> Result<Vec<Value>> {
    query(conn, "SELECT * FROM messages m WHERE project = ? AND sender != ? AND recipient IN ('all', ?)
        AND NOT EXISTS (SELECT 1 FROM reads r WHERE r.message_id = m.id AND r.agent = ?) ORDER BY id",
        params![project, agent, agent, agent])
}

pub(crate) fn mark_read(conn: &Connection, rows: &[Value], agent: &str) -> Result<()> {
    for row in rows {
        conn.execute(
            "INSERT OR IGNORE INTO reads VALUES (?, ?)",
            params![row["id"].as_i64(), agent],
        )?;
    }
    Ok(())
}

impl Bridge {
    pub fn send(&self, project: &str, sender: &str, recipient: &str, content: &str) -> Result<i64> {
        self.send_via(project, sender, recipient, content, "mcp")
    }

    pub(crate) fn send_via(
        &self,
        project: &str,
        sender: &str,
        recipient: &str,
        content: &str,
        via: &str,
    ) -> Result<i64> {
        let conn = self.database.open()?;
        conn.execute("INSERT INTO messages (project, sender, recipient, content, created_at, via) VALUES (?, ?, ?, ?, ?, ?)",
            params![project, sender, recipient, content, now()?, via])?;
        Ok(conn.last_insert_rowid())
    }

    pub fn read(
        &self,
        project: &str,
        agent: &str,
        limit: Option<i64>,
        include_read: bool,
    ) -> Result<Vec<Value>> {
        let mut conn = self.database.open()?;
        let transaction =
            conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let rows = if include_read {
            query(
                &transaction,
                "SELECT * FROM (SELECT * FROM messages WHERE project = ?
                AND (recipient IN ('all', ?) OR sender = ?) ORDER BY id DESC LIMIT ?) ORDER BY id",
                params![project, agent, agent, limit.unwrap_or(-1)],
            )?
        } else {
            let mut rows = unread(&transaction, project, agent)?;
            if let Some(limit) = limit {
                let length = if limit >= 0 {
                    limit as usize
                } else {
                    rows.len().saturating_sub(limit.unsigned_abs() as usize)
                };
                rows.truncate(length);
            }
            mark_read(&transaction, &rows, agent)?;
            rows
        };
        transaction.commit()?;
        Ok(rows)
    }

    pub(crate) fn recent(&self, project: &str) -> Result<Vec<Value>> {
        query(&self.database.open()?, "SELECT * FROM (SELECT * FROM messages WHERE project = ? ORDER BY id DESC LIMIT 20) ORDER BY id", [project])
    }
}
