use crate::{
    collaboration,
    file_batch::Batch,
    initialization::InitOptions,
    now,
    rules_files::RulesFiles,
    templates::{self, Assignments, Template},
    Bridge,
};
use anyhow::{bail, Context, Result};
use rusqlite::{params, Connection, TransactionBehavior};
use std::path::Path;

pub struct PreparedInit {
    conn: Connection,
    version: i64,
    project: String,
    template: Template,
    roles: Assignments,
    goal: String,
    pub charter: String,
    write_rules: bool,
    no_kickoff: bool,
    batch: Batch,
}
impl Bridge {
    pub fn prepare_init(&self, project: &str, options: InitOptions<'_>) -> Result<PreparedInit> {
        let state = self.access_state(Some(project), None)?;
        if !state.project_enabled {
            bail!("项目尚未开启，请先执行 on");
        }
        if !state.global_enabled {
            bail!("Bridge 全局关闭，请先执行 on --global");
        }
        if options.goal.trim().is_empty() {
            bail!("项目目标不能为空");
        }
        let conn = self.database.open()?;
        let version = conn.query_row("PRAGMA data_version", [], |r| r.get(0))?;
        let template = templates::load(options.template, options.template_file)?;
        let roles = templates::assignments(&template, options.roles)?;
        let charter = templates::charter(&template, options.goal, &roles);
        let mut batch = Batch::default();
        if options.write_rules {
            RulesFiles::prepare(Path::new(project), &charter)?.append_to(&mut batch)?;
        }
        Ok(PreparedInit {
            conn,
            version,
            project: project.into(),
            template,
            roles,
            goal: options.goal.into(),
            charter,
            write_rules: options.write_rules,
            no_kickoff: options.no_kickoff,
            batch,
        })
    }
}
impl PreparedInit {
    pub fn commit(mut self) -> Result<String> {
        let tx = self
            .conn
            .transaction_with_behavior(TransactionBehavior::Immediate)?;
        let current: i64 = tx.query_row("PRAGMA data_version", [], |r| r.get(0))?;
        if current != self.version {
            bail!("预览后协作数据发生变化，请重新预览并确认");
        }
        let project = &self.project;
        let version = collaboration::load(&tx, project)?.map_or(1, |i| i.version + 1);
        tx.execute(
            "DELETE FROM permission_overrides WHERE project=?",
            [project],
        )?;
        tx.execute("DELETE FROM role_assignments WHERE project=?", [project])?;
        for (agent, slot) in &self.roles {
            tx.execute(
                "UPDATE agent_settings SET enabled=1 WHERE project=? AND agent=? AND enabled!=1",
                params![project, agent],
            )?;
            tx.execute(
                "INSERT INTO role_assignments VALUES (?,?,?)",
                params![project, agent, slot],
            )?;
        }
        let stamp = now()?;
        tx.execute("INSERT INTO project_init VALUES (?,?,?,?,?,?,?,?,?) ON CONFLICT(project) DO UPDATE SET template=excluded.template,template_json=excluded.template_json,goal=excluded.goal,charter=excluded.charter,version=excluded.version,initialized_at=excluded.initialized_at,roles_json=excluded.roles_json,write_rules=excluded.write_rules",params![project,self.template.name,serde_json::to_string(&self.template)?,self.goal,self.charter,version,stamp,serde_json::to_string(&self.roles)?,self.write_rules])?;
        if !self.no_kickoff {
            for (agent, slot) in &self.roles {
                let r = self
                    .template
                    .roles
                    .iter()
                    .find(|r| &r.slot == slot)
                    .context("模板职务缺失")?;
                let task = if r.kickoff {
                    &self.template.first_task.content
                } else {
                    &self.template.first_task_others.content
                };
                tx.execute("INSERT INTO messages(project,sender,recipient,content,created_at) VALUES (?,'bridge',?,?,?)",params![project,agent,templates::expand(task,&self.goal,&self.roles),stamp])?;
            }
        }
        for path in self.batch.paths() {
            crate::repo_files::output(Path::new(project), &path.to_string_lossy())?;
        }
        self.batch
            .apply()
            .map_err(|e| anyhow::anyhow!("写入章程失败，已恢复原规则文件：{e}"))?;
        if let Err(e) = tx.commit() {
            self.batch.rollback()?;
            return Err(e.into());
        }
        Ok(format!(
            "协作初始化完成：{}，章程 v{version}（{project}）",
            self.template.name
        ))
    }
}
