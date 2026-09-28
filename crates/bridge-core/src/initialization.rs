use crate::{
    collaboration,
    rules_files::RulesFiles,
    templates::{self, Assignments},
    Bridge,
};
use anyhow::{bail, Context, Result};
use rusqlite::{params, TransactionBehavior};
use std::path::Path;

pub struct InitOptions<'a> {
    pub template: &'a str,
    pub template_file: Option<&'a Path>,
    pub roles: &'a [String],
    pub goal: &'a str,
    pub write_rules: bool,
    pub no_kickoff: bool,
}
impl Bridge {
    pub fn init_text(&self, project: &str, options: InitOptions<'_>) -> Result<String> {
        self.prepare_init(project, options)?.commit()
    }
    pub fn role_text(&self, project: &str, agent: &str, slot: &str) -> Result<String> {
        let mut conn = self.database.open()?;
        let tx = conn.transaction_with_behavior(TransactionBehavior::Immediate)?;
        let info = collaboration::load(&tx, project)?.context("项目尚未初始化，请先执行 init")?;
        let template = crate::permission_edit::effective(&tx, project, &info.template_json)?;
        if !template.roles.iter().any(|r| r.slot == slot) {
            bail!("当前模板没有职务位置：{slot}");
        }
        let agent = agent.trim().to_lowercase();
        let mut roles = collaboration::roles(&tx, project)?;
        let old = roles
            .get(&agent)
            .context("该 AI 未参与当前协作，请用 init 重新分配参与者")?
            .clone();
        if old == slot {
            return Ok(format!("{agent} 的职务已是 {slot}，未变更"));
        }
        let other = roles
            .iter()
            .find(|(_, s)| s.as_str() == slot)
            .map(|(a, _)| a.clone())
            .context("目标职务未分配，请重新 init")?;
        roles.insert(agent.clone(), slot.into());
        roles.insert(other.clone(), old.clone());
        for a in [&agent, &other] {
            tx.execute(
                "UPDATE role_assignments SET slot=? WHERE project=? AND agent=?",
                params![roles[a], project, a],
            )?;
        }
        let charter = templates::charter(&template, &info.goal, &roles);
        let files = if info.write_rules {
            Some(RulesFiles::prepare(Path::new(project), &charter)?)
        } else {
            None
        };
        tx.execute(
            "UPDATE project_init SET charter=?,version=version+1,roles_json=? WHERE project=?",
            params![charter, serde_json::to_string(&roles)?, project],
        )?;
        if let Some(files) = &files {
            files.write()?;
        }
        if let Err(error) = tx.commit() {
            if let Some(files) = &files {
                files.restore()?;
            }
            return Err(error.into());
        }
        Ok(format!(
            "职务已交换：{agent} → {slot}，{other} → {old}；章程 v{}",
            info.version + 1
        ))
    }
}
pub fn role_summary(roles: &Assignments) -> String {
    roles
        .iter()
        .map(|(a, s)| format!("{a}={s}"))
        .collect::<Vec<_>>()
        .join("，")
}
