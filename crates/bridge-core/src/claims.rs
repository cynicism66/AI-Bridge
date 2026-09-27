use crate::{
    database::query,
    format::{claim, text},
    now, Bridge, TIME_FMT,
};
use anyhow::Result;
use chrono::{Duration, NaiveDateTime};
use rusqlite::{params, Connection};

pub(crate) fn purge(conn: &Connection) -> Result<()> {
    conn.execute("DELETE FROM claims WHERE expires_at < ?", [now()?])?;
    Ok(())
}

impl Bridge {
    pub(crate) fn claim(
        &self,
        project: &str,
        files: &[String],
        ttl: i64,
        note: &str,
    ) -> Result<String> {
        let time = now()?;
        let duration =
            Duration::try_minutes(ttl.max(1)).ok_or_else(|| anyhow::anyhow!("认领时长超出范围"))?;
        let expires = NaiveDateTime::parse_from_str(&time, TIME_FMT)?
            .checked_add_signed(duration)
            .ok_or_else(|| anyhow::anyhow!("认领时长超出范围"))?
            .format(TIME_FMT)
            .to_string();
        let mut conn = self.database.open()?;
        let transaction =
            conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        purge(&transaction)?;
        let mut conflicts = Vec::new();
        for path in files {
            let rows = query(
                &transaction,
                "SELECT * FROM claims WHERE project = ? AND path = ?",
                params![project, path],
            )?;
            if let Some(row) = rows.into_iter().find(|r| text(r, "agent") != self.agent) {
                conflicts.push(row);
            }
        }
        if conflicts.is_empty() {
            for path in files {
                transaction.execute(
                    "INSERT OR REPLACE INTO claims VALUES (?, ?, ?, ?, ?, ?)",
                    params![project, path, self.agent, note, time, expires],
                )?;
            }
        }
        transaction.commit()?;
        if !conflicts.is_empty() {
            return Ok(format!("认领失败，以下文件已被别人认领（本次一个都没认领）：\n{}\n请先用 send_message 和对方协商，或等对方释放。",
                conflicts.iter().map(claim).collect::<Vec<_>>().join("\n")));
        }
        Ok(format!(
            "已认领 {} 个文件，到期 {expires}：\n{}",
            files.len(),
            files
                .iter()
                .map(|f| format!("  {f}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }

    pub(crate) fn release(&self, project: &str, files: &[String]) -> Result<String> {
        let mut conn = self.database.open()?;
        let transaction = conn.transaction()?;
        let mut count = 0;
        if files.is_empty() {
            count = transaction.execute(
                "DELETE FROM claims WHERE project = ? AND agent = ?",
                params![project, self.agent],
            )?;
        } else {
            for path in files {
                count += transaction.execute(
                    "DELETE FROM claims WHERE project = ? AND path = ? AND agent = ?",
                    params![project, path, self.agent],
                )?;
            }
        }
        transaction.commit()?;
        Ok(format!("已释放 {count} 个文件。"))
    }
}
