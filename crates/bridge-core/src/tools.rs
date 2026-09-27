use crate::{
    claims::purge,
    database::query,
    format::{self, text},
    messages, paths, Bridge,
};
use anyhow::{bail, Result};
use serde_json::Value;

pub const PROJECT_OFF: &str = "Bridge 未在此项目开启（由用户控制）。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。";
pub const GLOBAL_OFF: &str =
    "Bridge 已被用户全局关闭。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。";

fn integer(args: &Value, key: &str, default: i64) -> Result<i64> {
    let Some(value) = args.get(key) else {
        return Ok(default);
    };
    if let Some(n) = value.as_i64() {
        return Ok(n);
    }
    if let Some(b) = value.as_bool() {
        return Ok(i64::from(b));
    }
    if let Some(n) = value.as_f64() {
        return Ok(n as i64);
    }
    let s = value.as_str().unwrap_or("");
    s.trim()
        .parse()
        .map_err(|_| anyhow::anyhow!("invalid literal for int() with base 10: '{s}'"))
}

impl Bridge {
    pub fn tool(&self, name: &str, args: &Value) -> Result<String> {
        if name == "list_projects" {
            if !self.switch_state(None)?.global_enabled {
                return Ok(GLOBAL_OFF.into());
            }
            let rows = self.projects()?;
            if rows.is_empty() {
                return Ok("还没有项目使用过 Bridge。".into());
            }
            return Ok(format!(
                "用过 Bridge 的项目：\n{}",
                rows.iter()
                    .map(|r| {
                        let last = r["last"].as_str().unwrap_or("-");
                        let enabled = crate::human::label(r["enabled"].as_bool().unwrap_or(false));
                        format!("  {}（最近活动 {last}，{enabled}）", text(r, "project"))
                    })
                    .collect::<Vec<_>>()
                    .join("\n")
            ));
        }
        if ![
            "bridge_overview",
            "update_status",
            "send_message",
            "read_messages",
            "claim_files",
            "release_files",
        ]
        .contains(&name)
        {
            bail!("未知工具：{name}");
        }
        let project = paths::project(text(args, "project"))?;
        let state = self.switch_state(Some(&project))?;
        if !state.global_enabled {
            return Ok(GLOBAL_OFF.into());
        }
        if !state.project_enabled {
            return Ok(PROJECT_OFF.into());
        }
        match name {
            "bridge_overview" => self.overview(
                &project,
                &self.agent,
                args["mark_read"].as_bool().unwrap_or(true),
            ),
            "update_status" => self.update(&project, args),
            "send_message" => {
                let content = text(args, "content").trim();
                if content.is_empty() {
                    bail!("消息内容不能为空");
                }
                let to = text(args, "to");
                let to = if to.is_empty() { "all" } else { to }.trim().to_lowercase();
                let id = self.send(&project, &self.agent, &to, content)?;
                Ok(format!("消息 #{id} 已发送给 {}。", format::recipient(&to)))
            }
            "read_messages" => {
                let include = args["include_read"].as_bool().unwrap_or(false);
                let rows = self.read(
                    &project,
                    &self.agent,
                    Some(integer(args, "limit", 20)?),
                    include,
                )?;
                Ok(format::messages(&rows, include))
            }
            "claim_files" | "release_files" => {
                let files = args["files"]
                    .as_array()
                    .map(|files| {
                        files
                            .iter()
                            .filter(|f| {
                                name == "release_files"
                                    || !f.as_str().unwrap_or("").trim().is_empty()
                            })
                            .map(|f| paths::file(&project, f.as_str().unwrap_or("")))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if name == "release_files" {
                    return self.release(&project, &files);
                }
                if files.is_empty() {
                    bail!("files 不能为空");
                }
                self.claim(
                    &project,
                    &files,
                    integer(args, "ttl_minutes", 60)?,
                    text(args, "note"),
                )
            }
            _ => unreachable!(),
        }
    }

    pub fn overview(&self, project: &str, agent: &str, mark_read: bool) -> Result<String> {
        let mut conn = self.database.open()?;
        let transaction =
            conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        purge(&transaction)?;
        let statuses = query(
            &transaction,
            "SELECT * FROM status WHERE project = ? ORDER BY agent",
            [project],
        )?;
        let claims = query(
            &transaction,
            "SELECT * FROM claims WHERE project = ? ORDER BY agent, path",
            [project],
        )?;
        let unread = messages::unread(&transaction, project, agent)?;
        if mark_read {
            messages::mark_read(&transaction, &unread, agent)?;
        }
        transaction.commit()?;
        let statuses = format::section(&statuses, "  （还没有人汇报状态）", format::status);
        let claims = format::section(&claims, "  （没有文件被认领）", format::claim);
        let messages = format::section(&unread, "  （没有未读消息）", format::message);
        let tip = if mark_read && !unread.is_empty() {
            "\n  （以上消息已标为已读）"
        } else {
            ""
        };
        Ok(format!("项目：{project}\n你的身份：{agent}\n\n== 各方状态 ==\n{statuses}\n\n== 文件认领 ==\n{claims}\n\n== 给你的未读消息（{} 条）==\n{messages}{tip}", unread.len()))
    }
}
