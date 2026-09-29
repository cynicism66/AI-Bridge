use crate::{database::query, format::text, now, Bridge};
use anyhow::Result;
use rusqlite::{params, Connection};
use serde_json::Value;

#[derive(Clone, Copy)]
pub struct SwitchState {
    pub global_enabled: bool,
    pub project_enabled: bool,
}
impl SwitchState {
    pub fn enabled(self) -> bool {
        self.global_enabled && self.project_enabled
    }
}
pub(crate) fn state_in(conn: &Connection, project: Option<&str>) -> Result<SwitchState> {
    let mut state = SwitchState {
        global_enabled: true,
        project_enabled: false,
    };
    if query(
        conn,
        "SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings'",
        [],
    )?
    .is_empty()
    {
        return Ok(state);
    }
    for row in query(
        conn,
        "SELECT scope, enabled FROM settings WHERE scope IN (?, ?)",
        params!["global", project.unwrap_or("global")],
    )? {
        let enabled = row["enabled"].as_i64().unwrap_or(0) != 0;
        if text(&row, "scope") == "global" {
            state.global_enabled = enabled;
        } else {
            state.project_enabled = enabled;
        }
    }
    Ok(state)
}
impl Bridge {
    pub fn switch_state(&self, project: Option<&str>) -> Result<SwitchState> {
        if !self.database.path.is_file() {
            return Ok(SwitchState {
                global_enabled: true,
                project_enabled: false,
            });
        }
        state_in(&self.database.read_only()?, project)
    }
    pub fn set_enabled(&self, enabled: bool, project: Option<&str>) -> Result<()> {
        self.database.open()?.execute(
            "INSERT OR REPLACE INTO settings VALUES (?, ?)",
            params![project.unwrap_or("global"), enabled],
        )?;
        Ok(())
    }
    pub fn projects(&self) -> Result<Vec<Value>> {
        let conn = self.database.open()?;
        let mut rows = query(
            &conn,
            "SELECT project, MAX(t) AS last FROM (
            SELECT project, updated_at AS t FROM status
            UNION ALL SELECT project, created_at FROM messages
            UNION ALL SELECT project, claimed_at FROM claims
            UNION ALL SELECT scope, NULL FROM settings WHERE scope != 'global')
            GROUP BY project ORDER BY last DESC",
            [],
        )?;
        let settings = query(&conn, "SELECT scope, enabled FROM settings", [])?;
        for row in &mut rows {
            row["enabled"] =
                Value::Bool(settings.iter().any(|s| {
                    s["scope"] == row["project"] && s["enabled"].as_i64().unwrap_or(0) != 0
                }));
        }
        Ok(rows)
    }
    pub(crate) fn update(&self, project: &str, args: &Value) -> Result<String> {
        let mut conn = self.database.open()?;
        let transaction = conn.transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO status VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
            params![
                project,
                self.agent,
                text(args, "task"),
                text(args, "progress"),
                text(args, "blockers"),
                text(args, "next_step"),
                now()?,
                self.session_no(project)?,
                self.session_id
            ],
        )?;
        let count = crate::messages::unread(&transaction, project, &self.agent)?.len();
        transaction.commit()?;
        let tip = if count > 0 {
            format!("\n提示：你有 {count} 条未读消息，请用 read_messages 查看。")
        } else {
            String::new()
        };
        Ok(format!("状态已更新（{}）。{tip}", self.agent))
    }
}
