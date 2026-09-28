//! 等待新消息的只读游标，不迁移数据库、不标已读，也不创建会话。
use crate::{
    database::{check_version, query},
    desktop::Message,
    Bridge,
};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, OptionalExtension};

pub enum Poll {
    Messages(Vec<Message>),
    Disabled(&'static str),
}
pub struct Reader {
    conn: Connection,
    project: String,
    agent: String,
    cursor: i64,
}

pub fn disabled(bridge: &Bridge, project: &str, agent: &str) -> Result<Option<&'static str>> {
    let s = bridge.access_state(Some(project), Some(agent))?;
    Ok(reason(s.global_enabled, s.project_enabled, s.agent_enabled))
}
fn reason(global: bool, project: bool, agent: bool) -> Option<&'static str> {
    if !global {
        Some("Bridge 全局已关闭，已停止等待。")
    } else if !project {
        Some("Bridge 未在此项目开启，已停止等待。")
    } else if !agent {
        Some("此 AI 在该项目的协作已关闭，已停止等待。")
    } else {
        None
    }
}
impl Reader {
    pub fn new(bridge: &Bridge, project: &str, agent: &str) -> Result<Self> {
        let conn = bridge.database.read_only()?;
        if check_version(&conn)? < 5 {
            bail!("等待命令需要数据库版本 5 或更新版本，请先由新版 Bridge 完成升级。");
        }
        let cursor =
            conn.query_row("SELECT COALESCE(MAX(id),0) FROM messages", [], |r| r.get(0))?;
        Ok(Self {
            conn,
            project: project.into(),
            agent: agent.into(),
            cursor,
        })
    }
    pub fn poll(&mut self) -> Result<Poll> {
        let tx = self.conn.transaction()?;
        check_version(&tx)?;
        let setting = |scope: &str, default: bool| -> Result<bool> {
            Ok(tx
                .query_row("SELECT enabled FROM settings WHERE scope=?", [scope], |r| {
                    r.get(0)
                })
                .optional()?
                .unwrap_or(default))
        };
        let agent = tx
            .query_row(
                "SELECT enabled FROM agent_settings WHERE project=? AND agent=?",
                params![self.project, self.agent],
                |r| r.get(0),
            )
            .optional()?
            .unwrap_or(true);
        if let Some(message) = reason(
            setting("global", true)?,
            setting(&self.project, false)?,
            agent,
        ) {
            return Ok(Poll::Disabled(message));
        }
        let rows = query(&tx,"SELECT m.* FROM messages m WHERE m.project=?1 AND m.id>?2 AND m.sender!=?3 AND m.recipient IN ('all',?3)
            AND NOT EXISTS(SELECT 1 FROM reads r WHERE r.message_id=m.id AND r.agent=?3) ORDER BY m.id LIMIT 200",params![self.project,self.cursor,self.agent])?;
        let messages: Vec<Message> = rows
            .into_iter()
            .map(serde_json::from_value)
            .collect::<Result<_, _>>()?;
        if let Some(m) = messages.last() {
            self.cursor = m.id;
        }
        tx.commit()?;
        Ok(Poll::Messages(messages))
    }
}
pub fn line(m: &Message) -> String {
    let content: String = m
        .content
        .replace("\r\n", " ")
        .chars()
        .map(|c| {
            if matches!(c, '\r' | '\n' | '\u{0085}' | '\u{2028}' | '\u{2029}') {
                ' '
            } else {
                c
            }
        })
        .take(200)
        .collect();
    format!(
        "新消息 #{} {} → {}：{}",
        m.id,
        crate::format::sender(&m.sender, &m.via),
        crate::format::recipient(&m.recipient),
        content
    )
}

#[cfg(test)]
#[path = "wait_tests.rs"]
mod tests;
