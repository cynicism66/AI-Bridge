//! 项目列表管理只改 Bridge 数据与软件设置，不写入项目文件。
use crate::{app_settings::AppSettings, file_batch::Batch, now, Bridge, TIME_FMT};
use anyhow::{bail, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::Serialize;
use std::path::Path;

#[derive(Serialize)]
pub struct PurgeInfo {
    pub active_sessions: i64,
    pub write_rules: bool,
}

fn info(conn: &Connection, project: &str) -> Result<PurgeInfo> {
    let cutoff = (chrono::NaiveDateTime::parse_from_str(&now()?, TIME_FMT)?
        - chrono::Duration::hours(2))
    .format(TIME_FMT)
    .to_string();
    Ok(PurgeInfo {
        active_sessions: conn.query_row(
            "SELECT COUNT(*) FROM sessions WHERE project=? AND last_active>=?",
            params![project, cutoff],
            |r| r.get(0),
        )?,
        write_rules: conn.query_row(
            "SELECT EXISTS(SELECT 1 FROM project_init WHERE project=? AND write_rules!=0)",
            [project],
            |r| r.get(0),
        )?,
    })
}

pub(crate) fn purge_data(
    conn: &rusqlite::Transaction<'_>,
    project: &str,
    yes: bool,
    force: bool,
) -> Result<PurgeInfo> {
    if !yes {
        bail!("彻底删除不可撤销，请先用 export 导出，再加 --yes 确认");
    }
    let state = info(conn, project)?;
    if state.active_sessions > 0 && !force {
        bail!("有 AI 正在这个项目里工作；确认仍要删除时请加 --force");
    }
    conn.execute(
        "DELETE FROM reads WHERE message_id IN (SELECT id FROM messages WHERE project=?)",
        [project],
    )?;
    for table in [
        "status",
        "sessions",
        "messages",
        "claims",
        "agent_settings",
        "role_assignments",
        "permission_overrides",
        "project_init",
    ] {
        conn.execute(&format!("DELETE FROM {table} WHERE project=?"), [project])?;
    }
    conn.execute("DELETE FROM settings WHERE scope=?", [project])?;
    // claims 删除触发器可能产生 release/expire，最后统一清掉该项目历史。
    conn.execute("DELETE FROM events WHERE project=?", [project])?;
    Ok(state)
}

// 保存设置前先捕获原字节，普通失败回滚；并发改写设置时拒绝覆盖。
fn settings_batch(path: &Path, change: impl FnOnce(&mut AppSettings)) -> Result<Batch> {
    let original = crate::file_batch::original(path)?;
    let mut settings = original
        .as_deref()
        .map(serde_json::from_slice)
        .transpose()?
        .unwrap_or_default();
    let mut batch = Batch::default();
    change(&mut settings);
    batch.add(path.into(), serde_json::to_vec_pretty(&settings)?)?;
    if crate::file_batch::original(path)? != original {
        bail!("软件设置已变化，请重试");
    }
    Ok(batch)
}

impl Bridge {
    pub fn forget_project(&self, project: &str, settings_path: &Path) -> Result<String> {
        let mut conn = self.database.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut batch = settings_batch(settings_path, |s| {
            s.hidden_projects.insert(project.into());
        })?;
        tx.execute("INSERT OR REPLACE INTO settings VALUES (?,0)", [project])?;
        tx.execute("INSERT INTO events(project,agent,kind,detail,created_at) VALUES (?,'human','forget','{}',?)", params![project, now()?])?;
        batch.apply()?;
        if let Err(e) = tx.commit() {
            batch.rollback()?;
            return Err(e.into());
        }
        Ok(format!(
            "已从列表移除：{}；协作已关闭，数据保留，项目文件未改变。",
            crate::display_path(project)
        ))
    }

    pub fn add_project(&self, project: &str, settings_path: &Path) -> Result<()> {
        let mut conn = self.database.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let mut batch = settings_batch(settings_path, |s| {
            s.hidden_projects.remove(project);
        })?;
        tx.execute("INSERT OR IGNORE INTO settings VALUES (?,0)", [project])?;
        batch.apply()?;
        if let Err(e) = tx.commit() {
            batch.rollback()?;
            return Err(e.into());
        }
        Ok(())
    }

    pub fn purge_info(&self, project: &str) -> Result<PurgeInfo> {
        info(&self.database.open()?, project)
    }

    pub fn purge_project(
        &self,
        project: &str,
        settings_path: &Path,
        yes: bool,
        force: bool,
    ) -> Result<String> {
        if !yes {
            bail!("彻底删除不可撤销，请先用 export 导出，再加 --yes 确认");
        }
        let mut conn = self.database.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        purge_data(&tx, project, yes, force)?;
        // 仅保留 Bridge 自己的列表偏好，阻止自动发现立即把项目加回来。
        let mut batch = settings_batch(settings_path, |s| {
            s.hidden_projects.insert(project.into());
        })?;
        batch.apply()?;
        if let Err(e) = tx.commit() {
            batch.rollback()?;
            return Err(e.into());
        }
        Ok(format!(
            "已彻底删除：{}；Bridge 协作数据已清空，不保留删除历史。项目已在 Bridge 中隐藏；Claude/Codex 配置和项目文件未改变。\n如果曾写入规则文件，AGENTS.md / CLAUDE.md 中的 Bridge 章程区块仍保留，如不需要请手动删除。",
            crate::display_path(project)
        ))
    }
}
