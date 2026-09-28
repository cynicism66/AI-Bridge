use crate::{database::query, now, Bridge, TIME_FMT};
use anyhow::{bail, Result};
use rusqlite::params;
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
pub struct ActionItem {
    pub id: i64,
    pub kind: String,
    pub project: String,
    pub agent: String,
    pub content: String,
}
impl ActionItem {
    pub fn key(&self) -> String {
        format!("{}:{}", self.kind, self.id)
    }
}
impl Bridge {
    pub fn pending_actions(&self, project: &str) -> Result<Vec<ActionItem>> {
        let conn = self.database.open()?;
        let cutoff = (chrono::NaiveDateTime::parse_from_str(&now()?, TIME_FMT)?
            - chrono::Duration::hours(2))
        .format(TIME_FMT)
        .to_string();
        let mut items = Vec::new();
        for row in query(&conn,"SELECT id,sender AS agent,content FROM messages WHERE project=? AND needs_action=1 AND actioned_at IS NULL ORDER BY id",[project])? {
            items.push(ActionItem {id:row["id"].as_i64().unwrap(),kind:"message".into(),project:project.into(),agent:row["agent"].as_str().unwrap_or("").into(),content:row["content"].as_str().unwrap_or("").into()});
        }
        for row in query(&conn,"SELECT a.id,a.agent,a.content FROM action_blockers a JOIN sessions w ON w.project=a.project AND w.agent=a.agent AND w.id=a.session_id JOIN status s ON s.project=a.project AND s.agent=a.agent AND s.session_id=a.session_id WHERE a.project=? AND a.acknowledged_at IS NULL AND trim(a.content)!='' AND w.last_active>=? ORDER BY a.id",params![project,cutoff])? {
            items.push(ActionItem{id:row["id"].as_i64().unwrap(),kind:"blocker".into(),project:project.into(),agent:row["agent"].as_str().unwrap_or("").into(),content:row["content"].as_str().unwrap_or("").into()});
        }
        Ok(items)
    }
    /// 人用接口：只有桌面确认按钮调用，AI 工具没有此能力。
    pub fn resolve_action(&self, project: &str, kind: &str, id: i64) -> Result<()> {
        let conn = self.database.open()?;
        match kind {
            "message" => {
                conn.execute("UPDATE messages SET actioned_at=? WHERE project=? AND id=? AND needs_action=1 AND actioned_at IS NULL",params![now()?,project,id])?;
            }
            "blocker" => {
                conn.execute("UPDATE action_blockers SET acknowledged_at=? WHERE project=? AND id=? AND acknowledged_at IS NULL",params![now()?,project,id])?;
            }
            _ => bail!("未知待处理事项类型"),
        }
        Ok(())
    }
    pub fn send_action(
        &self,
        project: &str,
        recipient: &str,
        content: &str,
        needs_action: bool,
    ) -> Result<i64> {
        if needs_action && recipient != "human" {
            bail!("needs_action 只能用于发给 human 的消息");
        }
        let conn = self.database.open()?;
        conn.execute("INSERT INTO messages(project,sender,recipient,content,created_at,via,needs_action) VALUES (?,?,?,?,?,'mcp',?)",params![project,self.agent,recipient,content,now()?,needs_action])?;
        Ok(conn.last_insert_rowid())
    }
}
