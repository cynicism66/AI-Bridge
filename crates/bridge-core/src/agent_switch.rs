use crate::{database::query, format::text, human::label, Bridge};
use anyhow::{bail, Result};
use rusqlite::params;

impl Bridge {
    pub fn agent_toggle_text(&self, project: &str, agent: &str, enabled: bool) -> Result<String> {
        let agent = self.set_agent_enabled(project, agent, enabled)?;
        Ok(format!(
            "AI 开关：{agent} {}（{}）",
            label(enabled),
            crate::display_path(project)
        ))
    }

    pub fn set_agent_enabled(&self, project: &str, agent: &str, enabled: bool) -> Result<String> {
        let agent = agent.trim().to_lowercase();
        if agent.is_empty() {
            bail!("agent 不能为空");
        }
        self.database.open()?.execute(
            "INSERT INTO agent_settings VALUES (?,?,?) ON CONFLICT(project,agent) DO UPDATE SET enabled=excluded.enabled WHERE enabled!=excluded.enabled",
            params![project,agent,enabled])?;
        Ok(agent)
    }

    pub(crate) fn agents_text(&self, project: &str) -> Result<String> {
        let mut lines = Vec::new();
        for agent in self.agent_names(project)? {
            let state = self.access_state(Some(project), Some(&agent))?;
            lines.push(format!(
                "  {agent}：AI 开关 {}，有效状态 {}",
                label(state.agent_enabled),
                label(state.enabled())
            ));
        }
        Ok(if lines.is_empty() {
            "  （还没有 AI）".into()
        } else {
            lines.join("\n")
        })
    }

    pub(crate) fn agent_names(&self, project: &str) -> Result<Vec<String>> {
        Ok(query(&self.database.open()?,
            "SELECT agent FROM sessions WHERE project=?1 UNION SELECT agent FROM status WHERE project=?1 UNION SELECT agent FROM agent_settings WHERE project=?1 UNION SELECT agent FROM role_assignments WHERE project=?1 ORDER BY agent",[project])?
            .iter().map(|r| text(r, "agent").to_owned()).collect())
    }
}
