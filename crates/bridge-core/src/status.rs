use crate::{database::query, format::text, now, Bridge};
use anyhow::Result;
use rusqlite::params;
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

impl Bridge {
    pub fn switch_state(&self, project: Option<&str>) -> Result<SwitchState> {
        let mut state = SwitchState {
            global_enabled: true,
            project_enabled: false,
        };
        if self.database.path.is_file() {
            let conn = self.database.read_only()?;
            if !query(
                &conn,
                "SELECT 1 FROM sqlite_master WHERE type='table' AND name='settings'",
                [],
            )?
            .is_empty()
            {
                let rows = query(
                    &conn,
                    "SELECT scope, enabled FROM settings WHERE scope IN (?, ?)",
                    params!["global", project.unwrap_or("global")],
                )?;
                for row in rows {
                    let enabled = row["enabled"].as_i64().unwrap_or(0) != 0;
                    if text(&row, "scope") == "global" {
                        state.global_enabled = enabled;
                    } else {
                        state.project_enabled = enabled;
                    }
                }
            }
        }
        Ok(state)
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
            let enabled = settings
                .iter()
                .find(|s| s["scope"] == row["project"])
                .is_some_and(|s| s["enabled"].as_i64().unwrap_or(0) != 0);
            row["enabled"] = Value::Bool(enabled);
        }
        Ok(rows)
    }

    pub(crate) fn update(&self, project: &str, args: &Value) -> Result<String> {
        let mut conn = self.database.open()?;
        let transaction = conn.transaction()?;
        transaction.execute(
            "INSERT OR REPLACE INTO status VALUES (?, ?, ?, ?, ?, ?, ?)",
            params![
                project,
                self.agent,
                text(args, "task"),
                text(args, "progress"),
                text(args, "blockers"),
                text(args, "next_step"),
                now()?
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
