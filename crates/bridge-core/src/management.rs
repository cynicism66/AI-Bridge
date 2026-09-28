use crate::{collaboration, permission_edit, Bridge};
use anyhow::Result;
use serde_json::{json, Value};
impl Bridge {
    pub fn management(&self, project: &str) -> Result<Value> {
        let conn = self.database.open()?;
        let info = collaboration::load(&conn, project)?;
        let mut known = self.agent_names(project)?;
        known.extend(["claude".into(), "codex".into()]);
        known.sort();
        known.dedup();
        let Some(info) = info else {
            return Ok(
                json!({"known":known,"roles":[],"assignments":{},"charter":null,"write_rules":true}),
            );
        };
        let effective = permission_edit::effective(&conn, project, &info.template_json)?;
        Ok(
            json!({"known":known,"roles":effective.roles,"assignments":collaboration::roles(&conn,project)?,"write_rules":info.write_rules,
            "charter":{"version":info.version,"template":info.name,"goal":info.goal,"summary":info.charter}}),
        )
    }
}
