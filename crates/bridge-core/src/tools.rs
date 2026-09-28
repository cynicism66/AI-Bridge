use crate::{
    claims::purge,
    database::query,
    format::{self, text},
    messages, paths, repository, Bridge,
};
use anyhow::{bail, Result};
use serde_json::Value;

pub const PENDING_INIT: &str = "此项目的 Bridge 协作尚未初始化（由用户控制）。请提醒用户完成初始化（分配职务和权限），在此之前正常工作，不必再调用 Bridge 工具。";

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
        .map_err(|_| anyhow::anyhow!("{key} 必须是整数"))
}

impl Bridge {
    pub fn tool(&self, name: &str, args: &Value) -> Result<String> {
        if name == "list_projects" {
            if !self.switch_state(None)?.global_enabled {
                return Ok(GLOBAL_OFF.into());
            }
            let rows = self.projects_for(Some(&self.agent))?;
            self.refresh_sessions()?;
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
        let context = repository::resolve(text(args, "project"))?;
        let project = context.key.clone();
        let state = self.access_state(Some(&project), Some(&self.agent))?;
        if !state.global_enabled {
            return Ok(GLOBAL_OFF.into());
        }
        if !state.project_enabled {
            return Ok(PROJECT_OFF.into());
        }
        if !state.agent_enabled {
            return Ok(format!("Bridge 在此项目中未对你（{}）开启（由用户控制）。请忽略协作规则，正常工作；本次会话不必再调用 Bridge 工具。", self.agent));
        }
        if !state.initialized {
            return Ok(PENDING_INIT.into());
        }
        self.touch_session(&context)?;
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
                            .map(|f| paths::file(&context.worktree, f.as_str().unwrap_or("")))
                            .collect::<Vec<_>>()
                    })
                    .unwrap_or_default();
                if name == "release_files" {
                    return self.release(&project, &files);
                }
                if files.is_empty() {
                    bail!("files 不能为空");
                }
                if let Some(denial) = self.claim_permission(&project, &files)? {
                    return Ok(denial);
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
            "SELECT s.*, COALESCE(w.branch, '-') AS branch, COALESCE(w.worktree,s.project) AS worktree, COALESCE(w.last_active,s.updated_at) AS last_active FROM status s LEFT JOIN sessions w ON w.id=s.session_id AND w.project=s.project WHERE s.project = ? ORDER BY s.agent,s.session_no",
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
        let statuses = format::statuses(&statuses)?;
        let identity = if agent == "human" {
            agent.to_owned()
        } else {
            self.identity(project)?
        };
        let claims = format::section(&claims, "  （没有文件被认领）", format::claim);
        let messages = format::section(&unread, "  （没有未读消息）", format::message);
        let tip = if mark_read && !unread.is_empty() {
            "\n  （以上消息已标为已读）"
        } else {
            ""
        };
        let charter = self.charter_overview(project, agent)?;
        Ok(format!("项目：{project}\n你的身份：{identity}\n\n{charter}\n== 各方状态 ==\n{statuses}\n\n== 文件认领 ==\n{claims}\n\n== 给你的未读消息（{} 条）==\n{messages}{tip}", unread.len()))
    }
}
