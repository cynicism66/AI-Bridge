use crate::{collaboration, now, permissions, rules_files::RulesFiles, templates, Bridge};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Default, Serialize, Deserialize)]
pub struct PermissionPatch {
    pub allow: Option<Vec<String>>,
    pub deny: Option<Vec<String>>,
    pub write: Option<Vec<String>>,
    #[serde(default)]
    pub reset: bool,
}

pub fn effective(conn: &Connection, project: &str, base: &str) -> Result<templates::Template> {
    let mut template: templates::Template = serde_json::from_str(base)?;
    let mut stmt = conn.prepare(
        "SELECT slot,allow_json,deny_json,write_json FROM permission_overrides WHERE project=?",
    )?;
    let rows = stmt.query_map([project], |r| {
        Ok((
            r.get::<_, String>(0)?,
            r.get::<_, String>(1)?,
            r.get::<_, String>(2)?,
            r.get::<_, String>(3)?,
        ))
    })?;
    for row in rows {
        let (slot, allow, deny, write) = row?;
        if let Some(role) = template.roles.iter_mut().find(|r| r.slot == slot) {
            role.allow = serde_json::from_str(&allow)?;
            role.deny = serde_json::from_str(&deny)?;
            role.write = serde_json::from_str(&write)?;
        }
    }
    Ok(template)
}

pub fn validate_write(rules: &[String]) -> Result<()> {
    for rule in rules {
        permissions::validate(rule)?;
    }
    Ok(())
}

impl Bridge {
    pub fn permission_text(
        &self,
        project: &str,
        slot: &str,
        patch: PermissionPatch,
    ) -> Result<String> {
        let fields = patch.allow.is_some() || patch.deny.is_some() || patch.write.is_some();
        if patch.reset && fields {
            bail!("--reset 不能和权限列表同时使用");
        }
        if !patch.reset && !fields {
            bail!("请指定权限列表或 --reset");
        }
        let mut conn = self.database.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let info = collaboration::load(&tx, project)?.context("项目尚未初始化，请先执行 init")?;
        let mut template = effective(&tx, project, &info.template_json)?;
        let role = template
            .roles
            .iter_mut()
            .find(|r| r.slot == slot)
            .context("当前模板没有该职务位置")?;
        if patch.reset {
            let base: templates::Template = serde_json::from_str(&info.template_json)?;
            *role = base
                .roles
                .into_iter()
                .find(|r| r.slot == slot)
                .context("模板职务缺失")?;
            tx.execute(
                "DELETE FROM permission_overrides WHERE project=? AND slot=?",
                params![project, slot],
            )?;
        } else {
            if let Some(v) = patch.allow {
                role.allow = clean(v);
            }
            if let Some(v) = patch.deny {
                role.deny = clean(v);
            }
            if let Some(v) = patch.write {
                role.write = clean(v);
            }
            validate_write(&role.write)?;
            tx.execute("INSERT INTO permission_overrides VALUES (?,?,?,?,?) ON CONFLICT(project,slot) DO UPDATE SET allow_json=excluded.allow_json,deny_json=excluded.deny_json,write_json=excluded.write_json", params![project,slot,serde_json::to_string(&role.allow)?,serde_json::to_string(&role.deny)?,serde_json::to_string(&role.write)?])?;
        }
        let detail = serde_json::json!({"slot":slot,"reset":patch.reset,"allow":role.allow,"deny":role.deny,"write":role.write});
        let charter =
            templates::charter(&template, &info.goal, &collaboration::roles(&tx, project)?);
        let files = if info.write_rules {
            Some(RulesFiles::prepare(Path::new(project), &charter)?)
        } else {
            None
        };
        tx.execute(
            "UPDATE project_init SET charter=?,version=version+1 WHERE project=?",
            params![charter, project],
        )?;
        tx.execute("INSERT INTO events(project,agent,kind,detail,created_at) VALUES (?,'human','permission',?,?)", params![project,detail.to_string(),now()?])?;
        if let Some(files) = &files {
            files.write()?;
        }
        if let Err(e) = tx.commit() {
            if let Some(files) = &files {
                files.restore()?;
            }
            return Err(e.into());
        }
        Ok(format!(
            "职务 {slot} 的权限已{}；章程 v{}",
            if patch.reset {
                "恢复模板默认值"
            } else {
                "保存"
            },
            info.version + 1
        ))
    }
}
fn clean(values: Vec<String>) -> Vec<String> {
    values
        .into_iter()
        .map(|s| s.trim().to_owned())
        .filter(|s| !s.is_empty())
        .collect()
}
