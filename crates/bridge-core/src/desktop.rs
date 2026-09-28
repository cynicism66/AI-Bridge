//! 人用结构化接口。桌面和 CLI 共用 Bridge 的开关、消息与协作逻辑。
#[path = "desktop_types.rs"]
mod types;
use crate::{collaboration, database::query, format::text, now, Bridge};
use anyhow::{bail, Result};
use rusqlite::{params, Connection};
use serde::de::DeserializeOwned;
pub use types::*;

fn decode<T: DeserializeOwned>(rows: Vec<serde_json::Value>) -> Result<Vec<T>> {
    rows.into_iter()
        .map(|r| Ok(serde_json::from_value(r)?))
        .collect()
}

impl Bridge {
    pub fn project_list(&self) -> Result<Vec<ProjectSummary>> {
        let conn = self.database.open()?;
        self.projects()?.into_iter().map(|r| {
            let project = text(&r, "project").to_owned();
            let initialized = collaboration::load(&conn, &project)?.is_some();
            let unread = conn.query_row("SELECT count(*) FROM messages m WHERE project=? AND sender!='human' AND recipient IN ('human','all') AND NOT EXISTS (SELECT 1 FROM reads WHERE message_id=m.id AND agent='human')", [&project], |r| r.get(0))?;
            Ok(ProjectSummary { project, initialized, unread, enabled: r["enabled"].as_bool().unwrap_or(false), last: r["last"].as_str().map(str::to_owned) })
        }).collect()
    }

    pub fn project_detail(&self, project: &str) -> Result<ProjectDetail> {
        let mut conn = self.database.open()?;
        let tx = conn.transaction()?;
        let charter = collaboration::load(&tx, project)?.map(|i| Charter {
            version: i.version,
            template: i.name,
            goal: i.goal,
            summary: i.charter,
        });
        let roles = collaboration::roles(&tx, project)?;
        let names = self.agent_names(project)?;
        let agents = names
            .iter()
            .map(|agent| {
                Ok(Agent {
                    agent: agent.clone(),
                    role: roles.get(agent).cloned(),
                    enabled: self.access_state(Some(project), Some(agent))?.agent_enabled,
                })
            })
            .collect::<Result<Vec<_>>>()?;
        let cutoff = (chrono::NaiveDateTime::parse_from_str(&now()?, crate::TIME_FMT)?
            - chrono::Duration::hours(2))
        .format(crate::TIME_FMT)
        .to_string();
        let sessions = decode(query(&tx, "SELECT w.last_active,w.last_active < ? AS older,w.agent,w.session_no,w.branch,w.worktree,COALESCE(s.task,'') AS task,COALESCE(s.progress,'') AS progress,COALESCE(s.blockers,'') AS blockers,COALESCE(s.next_step,'') AS next_step,COALESCE(s.updated_at,w.last_active) AS updated_at FROM sessions w LEFT JOIN status s ON s.project=w.project AND s.session_id=w.id WHERE w.project=? ORDER BY w.agent,w.session_no,w.started_at", params![cutoff,project])?)?;
        let claims = decode(query(
            &tx,
            "SELECT * FROM claims WHERE project=? AND expires_at>=? ORDER BY agent,path",
            params![project, now()?],
        )?)?;
        tx.commit()?;
        Ok(ProjectDetail {
            charter,
            agents,
            sessions,
            claims,
        })
    }

    /// ID 游标向前翻页；返回页内按发送顺序排列，不改任何人的已读状态。
    pub fn message_page(
        &self,
        project: &str,
        before: Option<i64>,
        limit: i64,
    ) -> Result<MessagePage> {
        let limit = limit.clamp(1, 200);
        let mut messages: Vec<Message> = decode(query(
            &self.database.open()?,
            "SELECT * FROM messages WHERE project=? AND id<? ORDER BY id DESC LIMIT ?",
            params![project, before.unwrap_or(i64::MAX), limit + 1],
        )?)?;
        let more = messages.len() as i64 > limit;
        messages.truncate(limit as usize);
        messages.reverse();
        let before = if more {
            messages.first().map(|m| m.id)
        } else {
            None
        };
        Ok(MessagePage { messages, before })
    }

    pub fn post_human(&self, project: &str, content: &str, to: &str, via: HumanVia) -> Result<i64> {
        let content = content.trim();
        if content.is_empty() {
            bail!("消息内容不能为空");
        }
        self.send_via(
            project,
            "human",
            to,
            content,
            match via {
                HumanVia::Cli => "cli",
                HumanVia::Gui => "gui",
            },
        )
    }

    /// 只标记当前已经展示到的消息，避免并发新消息被悄悄标为已读。
    pub fn mark_human_read(&self, project: &str, through: i64) -> Result<()> {
        self.database.open()?.execute("INSERT OR IGNORE INTO reads SELECT id,'human' FROM messages WHERE project=? AND id<=? AND sender!='human' AND recipient IN ('human','all')", params![project,through])?;
        Ok(())
    }

    pub fn notification_messages(&self, after: i64) -> Result<Vec<Message>> {
        decode(query(&self.database.open()?, "SELECT * FROM messages WHERE id>? AND sender!='human' AND recipient IN ('human','all') ORDER BY id LIMIT 100", [after])?)
    }

    pub fn latest_message_id(&self) -> Result<i64> {
        Ok(self.database.open()?.query_row(
            "SELECT COALESCE(MAX(id),0) FROM messages",
            [],
            |r| r.get(0),
        )?)
    }
}

/// 必须保留同一连接：所有 Bridge 写入走独立连接，含本进程 GUI 写入都会改变 data_version。
pub struct ChangeDetector(Connection);
impl ChangeDetector {
    pub fn new(bridge: &Bridge) -> Result<Self> {
        Ok(Self(bridge.database.open()?))
    }
    pub fn version(&self) -> Result<i64> {
        Ok(self.0.query_row("PRAGMA data_version", [], |r| r.get(0))?)
    }
}

#[cfg(test)]
#[path = "desktop_tests.rs"]
mod tests;
