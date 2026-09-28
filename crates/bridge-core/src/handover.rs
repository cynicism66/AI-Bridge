use crate::{
    collaboration, documents,
    format::text,
    messages,
    templates::{self, Assignments},
};
use anyhow::{Context, Result};
use rusqlite::{params, Connection};
use serde_json::{json, Value};

pub struct Plan {
    pub to: String,
    pub template: templates::Template,
    pub assignments: Assignments,
    pub charter: String,
    pub version: i64,
    pub write_rules: bool,
    pub others: Vec<String>,
    pub forwarded: Vec<Value>,
}
impl Plan {
    pub fn prepare(conn: &Connection, project: &str, to: &str) -> Result<Self> {
        let info =
            collaboration::load(conn, project)?.context("项目尚未初始化，请用户先执行 init")?;
        let template = templates::load("独立开发", None)?;
        let assignments = templates::assignments(&template, &[format!("独立开发={to}")])?;
        let to = assignments.keys().next().unwrap().clone();
        let others = documents::agents(conn, project)?
            .into_iter()
            .filter(|a| a != &to)
            .collect::<Vec<_>>();
        let mut forwarded = std::collections::BTreeMap::new();
        for agent in &others {
            for message in messages::unread(conn, project, agent)? {
                forwarded.insert(message["id"].as_i64().unwrap(), message);
            }
        }
        let charter = templates::charter(&template, &info.goal, &assignments);
        Ok(Self {
            to,
            template,
            assignments,
            charter,
            version: info.version + 1,
            write_rules: info.write_rules,
            others,
            forwarded: forwarded.into_values().collect(),
        })
    }
    pub fn apply(&self, conn: &Connection, project: &str, time: &str) -> Result<()> {
        conn.execute(
            "DELETE FROM permission_overrides WHERE project=?",
            [project],
        )?;
        conn.execute("DELETE FROM role_assignments WHERE project=?", [project])?;
        conn.execute(
            "INSERT INTO role_assignments VALUES (?,?,'独立开发')",
            params![project, self.to],
        )?;
        conn.execute("UPDATE project_init SET template='独立开发',template_json=?,charter=?,version=?,roles_json=? WHERE project=?",params![serde_json::to_string(&self.template)?,self.charter,self.version,serde_json::to_string(&self.assignments)?,project])?;
        for agent in &self.others {
            conn.execute("INSERT INTO agent_settings VALUES (?,?,0) ON CONFLICT(project,agent) DO UPDATE SET enabled=0 WHERE enabled!=0",params![project,agent])?;
            conn.execute(
                "DELETE FROM claims WHERE project=? AND agent=?",
                params![project, agent],
            )?;
        }
        conn.execute("INSERT INTO agent_settings VALUES (?,?,1) ON CONFLICT(project,agent) DO UPDATE SET enabled=1 WHERE enabled!=1",params![project,self.to])?;
        for row in &self.forwarded {
            let body = format!(
                "交接转发原消息 #{}（{} → {}）：\n{}",
                row["id"],
                text(row, "sender"),
                text(row, "recipient"),
                text(row, "content")
            );
            conn.execute("INSERT INTO messages(project,sender,recipient,content,created_at) VALUES (?,'bridge',?,?,?)",params![project,self.to,body,time])?;
        }
        conn.execute("INSERT INTO messages(project,sender,recipient,content,created_at) VALUES (?,'bridge',?,?,?)",params![project,self.to,self.template.first_task.content,time])?;
        // 不改变 v4 结构；业务修改、已有触发器事件及交接总事件同一事务提交。
        conn.execute("INSERT INTO events(project,agent,kind,detail,created_at) VALUES (?,'human','handover',?,?)",params![project,json!({"to":self.to,"template":"独立开发"}).to_string(),time])?;
        Ok(())
    }
}
