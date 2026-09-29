//! Stop 消息投递与每个项目、每个 AI 的单一收件窗口。
use crate::{format::text, messages, now, repository, status, Bridge, TIME_FMT};
use anyhow::{bail, Result};
use chrono::{Duration, NaiveDateTime};
use rusqlite::{params, OptionalExtension, TransactionBehavior};
use serde::Deserialize;
use serde_json::{json, Value};

#[derive(Deserialize)]
pub struct Input {
    pub cwd: String,
    pub session_id: String,
    #[serde(default)]
    pub stop_hook_active: bool,
}
fn valid_agent(agent: &str) -> Result<()> {
    if !matches!(agent, "claude" | "codex") {
        bail!("AI 必须是 claude 或 codex");
    }
    Ok(())
}
impl Bridge {
    pub fn rebind(&self, project: &str, agent: &str) -> Result<String> {
        valid_agent(agent)?;
        self.database.open()?.execute(
            "INSERT INTO hook_receivers(project,agent,pending_rebind) VALUES(?,?,1)
             ON CONFLICT(project,agent) DO UPDATE SET session_id=NULL,last_active=NULL,pending_rebind=1",
            params![project,agent],
        )?;
        Ok(format!("已清除 {agent} 的收件窗口。请在要接收消息的那个 {agent} 窗口里随便说一句话，它这一轮结束后就会成为收件窗口。"))
    }

    /// 在事务内完成绑定、输出和已读。输出失败则回滚，避免消息无声丢失。
    pub fn deliver_hook(
        &self,
        input: &Input,
        agent: &str,
        output: impl FnOnce(&Value) -> Result<()>,
    ) -> Result<()> {
        deliver_at(self, input, agent, &now()?, output)
    }
}
fn deliver_at(
    bridge: &Bridge,
    input: &Input,
    agent: &str,
    stamp: &str,
    output: impl FnOnce(&Value) -> Result<()>,
) -> Result<()> {
    valid_agent(agent)?;
    if input.session_id.trim().is_empty() {
        bail!("钩子缺少有效 session_id");
    }
    let Some(project) = repository::resolve_git(&input.cwd)? else {
        return Ok(());
    };
    // 不因一个未使用 Bridge 的窗口结束而新建数据库。
    if !bridge.database.path.is_file() {
        return Ok(());
    }
    let cutoff = (NaiveDateTime::parse_from_str(stamp, TIME_FMT)? - Duration::hours(2))
        .format(TIME_FMT)
        .to_string();
    let mut conn = bridge.database.open()?;
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let binding: Option<(Option<String>, Option<String>, bool)> = tx.query_row(
        "SELECT session_id,last_active,pending_rebind FROM hook_receivers WHERE project=? AND agent=?",
        params![project.key,agent], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
    ).optional()?;
    let bind = || -> Result<()> {
        tx.execute(
            "INSERT INTO hook_receivers(project,agent,session_id,last_active,pending_rebind)
            VALUES(?,?,?,?,0) ON CONFLICT(project,agent) DO UPDATE SET
            session_id=excluded.session_id,last_active=excluded.last_active,pending_rebind=0",
            params![project.key, agent, input.session_id, stamp],
        )?;
        Ok(())
    };
    if let Some((session, last, pending)) = &binding {
        if *pending || session.as_deref() == Some(&input.session_id) {
            // 即使 stop_hook_active 或项目关闭，也更新已绑定窗口的心跳。
            bind()?;
        } else if last.as_ref().is_some_and(|last| last >= &cutoff) {
            return Ok(());
        }
    }
    if input.stop_hook_active || !status::state_in(&tx, Some(&project.key))?.enabled() {
        tx.commit()?;
        return Ok(());
    }
    let rows = messages::unread(&tx, &project.key, agent)?;
    if rows.is_empty() {
        tx.commit()?;
        return Ok(());
    }
    let remaining = rows.len().saturating_sub(10);
    let selected = &rows[remaining..];
    let mut reason = String::from("Bridge 公告板上有给你的新消息：\n");
    for row in selected {
        reason.push_str(&format!(
            "#{} [{} → {}] {}\n",
            row["id"],
            text(row, "sender"),
            text(row, "recipient"),
            text(row, "content")
        ));
    }
    if remaining > 0 {
        reason.push_str(&format!("还有 {remaining} 条，用 read_messages 查看\n"));
    }
    reason.push_str("需要回复的用 send_message 回复；只是确认收到的消息不用回复。");
    bind()?;
    messages::mark_read(&tx, selected, agent)?;
    output(&json!({"decision":"block","reason":reason}))?;
    tx.commit()?;
    Ok(())
}

#[cfg(test)]
#[path = "hook_tests.rs"]
mod tests;
