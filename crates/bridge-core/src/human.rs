use crate::{
    format::{self, text},
    status::SwitchState,
    Bridge,
};
use anyhow::{bail, Result};

pub fn label(enabled: bool) -> &'static str {
    if enabled {
        "已开启"
    } else {
        "未开启"
    }
}

pub fn notice(state: SwitchState, project: bool) -> &'static str {
    if !state.global_enabled {
        "提示：Bridge 全局已关闭；人用命令仍可使用。\n"
    } else if project && !state.project_enabled {
        "提示：Bridge 未在此项目开启；人用命令仍可使用。\n"
    } else {
        ""
    }
}

impl Bridge {
    pub fn status_text(&self) -> Result<String> {
        let state = self.switch_state(None)?;
        let mut lines = vec![format!(
            "{}全局开关：{}",
            notice(state, false),
            if state.global_enabled {
                "已开启"
            } else {
                "已关闭"
            }
        )];
        for row in self.projects()? {
            let enabled = row["enabled"].as_bool().unwrap_or(false);
            lines.push(format!(
                "  {}：项目开关 {}，有效状态 {}，最近活动 {}",
                text(&row, "project"),
                label(enabled),
                label(enabled && state.global_enabled),
                row["last"].as_str().unwrap_or("-")
            ));
        }
        if lines.len() == 1 {
            lines.push("  （还没有项目）".into());
        }
        Ok(lines.join("\n"))
    }

    pub fn toggle_text(
        &self,
        enabled: bool,
        project: Option<&str>,
        exists: bool,
    ) -> Result<String> {
        self.set_enabled(enabled, project)?;
        let state = self.switch_state(project)?;
        if let Some(project) = project {
            let warning = if exists {
                ""
            } else {
                "提示：项目目录不存在，已按指定路径保存开关。\n"
            };
            Ok(format!(
                "{}{warning}项目开关：{}（{project}）",
                notice(state, true),
                label(state.project_enabled)
            ))
        } else {
            Ok(format!(
                "{}全局开关：{}",
                notice(state, false),
                if state.global_enabled {
                    "已开启"
                } else {
                    "已关闭"
                }
            ))
        }
    }

    pub fn show_text(&self, project: &str) -> Result<String> {
        let state = self.switch_state(Some(project))?;
        let overview = self.overview(project, "human", false)?;
        let messages = format::section(&self.recent(project)?, "  （无）", format::message);
        Ok(format!(
            "{}启用状态：{}\n{overview}\n\n== 最近消息 ==\n{messages}",
            notice(state, true),
            label(state.enabled())
        ))
    }

    pub fn post_text(&self, project: &str, content: &str, to: &str) -> Result<String> {
        let content = content.trim();
        if content.is_empty() {
            bail!("消息内容不能为空");
        }
        let prefix = notice(self.switch_state(Some(project))?, true);
        let id = self.send(project, "human", to, content)?;
        Ok(format!(
            "{prefix}消息 #{id} 已发送给 {}。",
            format::recipient(to)
        ))
    }

    pub fn read_text(&self, project: &str) -> Result<String> {
        let prefix = notice(self.switch_state(Some(project))?, true);
        let messages = self.read(project, "human", None, false)?;
        Ok(format!("{prefix}{}", format::messages(&messages, false)))
    }
}
