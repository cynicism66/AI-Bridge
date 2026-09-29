use crate::{database::query, now, repository::Project, Bridge, TIME_FMT};
use anyhow::Result;
use chrono::{Duration, NaiveDateTime};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};

pub(crate) fn random_id() -> Result<String> {
    Ok(Connection::open_in_memory()?
        .query_row("SELECT lower(hex(randomblob(16)))", [], |r| r.get(0))?)
}

pub(crate) fn touch(
    conn: &mut Connection,
    id: &str,
    agent: &str,
    project: &Project,
    time: &str,
) -> Result<i64> {
    let cutoff = (NaiveDateTime::parse_from_str(time, TIME_FMT)? - Duration::hours(2))
        .format(TIME_FMT)
        .to_string();
    let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
    let existing: Option<(i64, String)> = tx
        .query_row(
            "SELECT session_no,last_active FROM sessions WHERE id=? AND project=?",
            params![id, project.key],
            |r| Ok((r.get(0)?, r.get(1)?)),
        )
        .optional()?;
    let used = query(
        &tx,
        "SELECT session_no FROM sessions WHERE project=? AND agent=? AND last_active>=? AND id!=?",
        params![project.key, agent, cutoff, id],
    )?;
    let occupied = |number| {
        used.iter()
            .any(|r| r["session_no"].as_i64() == Some(number))
    };
    let number = if let Some((n, _)) = existing.filter(|(n, last)| last >= &cutoff && !occupied(*n))
    {
        n
    } else {
        let mut n = 1;
        while occupied(n) {
            n += 1;
        }
        tx.execute(
            "UPDATE claims SET session_no=? WHERE project=? AND session_id=?",
            params![n, project.key, id],
        )?;
        n
    };
    // 编号属于当前会话；复用前清除旧拥有者，防止旧状态挂到新身份下。
    tx.execute(
        "DELETE FROM status WHERE project=? AND agent=? AND session_no=? AND session_id!=?",
        params![project.key, agent, number, id],
    )?;
    tx.execute(
        "DELETE FROM sessions WHERE project=? AND agent=? AND session_no=? AND id!=?",
        params![project.key, agent, number, id],
    )?;
    tx.execute(
        "UPDATE status SET session_no=? WHERE project=? AND session_id=?",
        params![number, project.key, id],
    )?;
    tx.execute("INSERT INTO sessions (id,project,agent,session_no,worktree,branch,pid,started_at,last_active) VALUES (?,?,?,?,?,?,?,?,?) ON CONFLICT(id,project) DO UPDATE SET
                session_no=excluded.session_no,worktree=excluded.worktree,branch=excluded.branch,last_active=excluded.last_active",
               params![id,project.key,agent,number,project.worktree,project.branch,std::process::id(),time,time])?;
    tx.commit()?;
    Ok(number)
}

impl Bridge {
    pub(crate) fn touch_session(&self, project: &Project) -> Result<i64> {
        touch(
            &mut self.database.open()?,
            &self.session_id,
            &self.agent,
            project,
            &now()?,
        )
    }
    pub(crate) fn refresh_sessions(&self) -> Result<()> {
        let rows = query(&self.database.open()?, "SELECT project, worktree FROM sessions WHERE id=? AND EXISTS (SELECT 1 FROM settings WHERE scope=sessions.project AND enabled!=0)", [&self.session_id])?;
        for row in rows {
            let project = crate::repository::resolve(crate::format::text(&row, "worktree"))?;
            self.touch_session(&project)?;
        }
        Ok(())
    }
    pub(crate) fn session_no(&self, project: &str) -> Result<i64> {
        Ok(self.database.open()?.query_row(
            "SELECT session_no FROM sessions WHERE id=? AND project=?",
            params![self.session_id, project],
            |r| r.get(0),
        )?)
    }
    pub(crate) fn identity(&self, project: &str) -> Result<String> {
        let conn = self.database.open()?;
        let (number, branch, worktree): (i64, String, String) = conn.query_row(
            "SELECT session_no,branch,worktree FROM sessions WHERE id=? AND project=?",
            params![self.session_id, project],
            |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)),
        )?;
        Ok(format!(
            "{} #{number} · 分支 {branch} · 文件夹 {worktree}",
            self.agent
        ))
    }
}

#[cfg(test)]
#[path = "session_tests.rs"]
mod tests;
