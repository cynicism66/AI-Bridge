use crate::{
    database::query,
    format::text,
    templates::{Assignments, Template},
    Bridge,
};
use anyhow::Result;
use rusqlite::{Connection, OptionalExtension};

pub struct Info {
    pub template_json: String,
    pub goal: String,
    pub charter: String,
    pub version: i64,
    pub name: String,
    pub write_rules: bool,
}
pub fn load(conn: &Connection, project: &str) -> Result<Option<Info>> {
    Ok(conn
        .query_row(
            "SELECT template_json,goal,charter,version,template,write_rules FROM project_init WHERE project=?",
            [project],
            |r| {
                Ok(Info {
                    template_json: r.get(0)?,
                    goal: r.get(1)?,
                    charter: r.get(2)?,
                    version: r.get(3)?,
                    name: r.get(4)?,
                    write_rules: r.get(5)?,
                })
            },
        )
        .optional()?)
}
pub fn roles(conn: &Connection, project: &str) -> Result<Assignments> {
    Ok(query(
        conn,
        "SELECT agent,slot FROM role_assignments WHERE project=? ORDER BY agent",
        [project],
    )?
    .iter()
    .map(|r| (text(r, "agent").into(), text(r, "slot").into()))
    .collect())
}
impl Bridge {
    pub(crate) fn collaboration_text(&self, project: &str) -> Result<String> {
        let conn = self.database.open()?;
        let state = self.access_state(Some(project), None)?;
        let active = state.global_enabled && state.project_enabled;
        let Some(info) = load(&conn, project)? else {
            return Ok(format!(
                "协作状态：{}",
                if active { "待初始化" } else { "关闭" }
            ));
        };
        let label = if active { "协作中" } else { "关闭" };
        let assignments = roles(&conn, project)?;
        Ok(format!(
            "协作状态：{label} · 章程 v{}\n模板：{}\n项目目标：{}\n{}",
            info.version,
            info.name,
            info.goal,
            assignments
                .iter()
                .map(|(a, s)| format!("  {a}：职务 {s}"))
                .collect::<Vec<_>>()
                .join("\n")
        ))
    }
    pub(crate) fn charter_overview(&self, project: &str, agent: &str) -> Result<String> {
        let mut conn = self.database.open()?;
        let tx = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
        let Some(info) = load(&tx, project)? else {
            return Ok("协作状态：待初始化\n".into());
        };
        if agent != "human" {
            let seen: i64 = tx.query_row(
                "SELECT charter_version FROM sessions WHERE project=? AND id=?",
                rusqlite::params![project, self.session_id],
                |r| r.get(0),
            )?;
            if seen == info.version {
                return Ok(format!(
                    "协作章程 v{}（本会话已显示过，未变化）\n",
                    info.version
                ));
            }
        }
        let assignments = roles(&tx, project)?;
        let t: Template = serde_json::from_str(&info.template_json)?;
        let mine = assignments
            .get(agent)
            .and_then(|slot| t.roles.iter().find(|r| &r.slot == slot));
        let permissions = mine.map_or(
            "未分配职务；不能认领文件，请联系用户。".into(),
            |r| {
                format!(
                    "职务：{}\n职责：{}\n允许：{}\n禁止：{}\n可写范围：{}",
                    r.slot,
                    r.duties,
                    r.allow.join("、"),
                    r.deny.join("、"),
                    if r.write.is_empty() {
                        "不限制".into()
                    } else {
                        r.write.join("、")
                    }
                )
            },
        );
        let all = assignments
            .iter()
            .map(|(a, s)| format!("  {a}：{s}{}", if a == agent { " ← 你" } else { "" }))
            .collect::<Vec<_>>()
            .join("\n");
        let result=format!("== 协作章程 v{} ==\n{}\n\n== 你的职务和权限 ==\n{permissions}\n\n== 各方职务 ==\n{all}\n",info.version,info.charter);
        if agent != "human" {
            tx.execute(
                "UPDATE sessions SET charter_version=? WHERE project=? AND id=?",
                rusqlite::params![info.version, project, self.session_id],
            )?;
        }
        tx.commit()?;
        Ok(result)
    }
}
