use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::{collections::BTreeMap, path::Path};

pub const COMMON: &str = include_str!("../../../templates/common.md");
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Role {
    pub slot: String,
    pub duties: String,
    #[serde(default)]
    pub allow: Vec<String>,
    #[serde(default)]
    pub deny: Vec<String>,
    #[serde(default)]
    pub write: Vec<String>,
    #[serde(default)]
    pub kickoff: bool,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Task {
    pub content: String,
    pub to: Option<String>,
}
#[derive(Clone, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Template {
    pub name: String,
    pub charter: String,
    pub roles: Vec<Role>,
    pub first_task: Task,
    pub first_task_others: Task,
}
pub type Assignments = BTreeMap<String, String>; // agent -> slot

pub fn load(name: &str, file: Option<&Path>) -> Result<Template> {
    let source = match (name, file) {
        ("自定义", Some(path)) => std::fs::read_to_string(path).context("无法读取自定义模板")?,
        ("自定义", None) => bail!("自定义模板必须提供 --template-file"),
        (_, Some(_)) => bail!("--template-file 只能和 --template 自定义 一起使用"),
        ("任务书流程", None) => include_str!("../../../templates/task.toml").into(),
        ("结对流程", None) => include_str!("../../../templates/pair.toml").into(),
        ("独立开发", None) => include_str!("../../../templates/solo.toml").into(),
        _ => bail!("未知模板：{name}；可选：任务书流程、结对流程、独立开发、自定义"),
    };
    parse(&source)
}
pub fn parse(source: &str) -> Result<Template> {
    let t: Template = toml::from_str(source).context("模板 TOML 格式错误")?;
    let mut slots = std::collections::HashSet::new();
    if t.name.trim().is_empty() || t.charter.trim().is_empty() || t.roles.is_empty() {
        bail!("模板名字、章程和职务位置不能为空");
    }
    for r in &t.roles {
        if r.slot.trim().is_empty()
            || r.slot != r.slot.trim()
            || r.slot.contains(['{', '}', '='])
            || r.slot == "goal"
            || !slots.insert(&r.slot)
        {
            bail!("职务位置为空、重复或含保留字符：{}", r.slot);
        }
        for rule in &r.write {
            crate::permissions::validate(rule)?;
        }
    }
    if t.roles.iter().filter(|r| r.kickoff).count() != 1 {
        bail!("模板必须有且只有一个 kickoff = true 的职务位置");
    }
    if let Some(to) = &t.first_task.to {
        if !t.roles.iter().any(|r| r.kickoff && &r.slot == to) {
            bail!("first_task.to 必须指向 kickoff 职务");
        }
    }
    Ok(t)
}
pub fn assignments(t: &Template, values: &[String]) -> Result<Assignments> {
    let mut result = Assignments::new();
    let mut slots = std::collections::HashSet::new();
    for value in values {
        let (slot, agent) = value
            .split_once('=')
            .context("--role 格式必须是 职务位置=agent")?;
        let slot = slot.trim();
        let agent = agent.trim().to_lowercase();
        if agent.is_empty()
            || agent.chars().any(char::is_whitespace)
            || ["human", "bridge", "all"].contains(&agent.as_str())
        {
            bail!("无效或保留的 AI 名称：{agent}");
        }
        if !t.roles.iter().any(|r| r.slot == slot) {
            bail!("模板中没有职务位置：{slot}");
        }
        if result.insert(agent.clone(), slot.into()).is_some() {
            bail!("一个 agent 只能担任一个职务位置：{agent}");
        }
        if !slots.insert(slot.to_owned()) {
            bail!("职务位置不能重复分配：{slot}");
        }
    }
    let missing = t
        .roles
        .iter()
        .filter(|r| !slots.contains(&r.slot))
        .map(|r| r.slot.as_str())
        .collect::<Vec<_>>();
    if !missing.is_empty() {
        bail!("缺少职务位置：{}", missing.join("、"));
    }
    Ok(result)
}
pub fn expand(body: &str, goal: &str, roles: &Assignments) -> String {
    // 一次扫描，用户目标和 AI 名称中出现的占位符不会再次展开。
    let mut output = String::new();
    let mut rest = body;
    while let Some(start) = rest.find('{') {
        output.push_str(&rest[..start]);
        let Some(end) = rest[start..].find('}') else {
            output.push_str(&rest[start..]);
            return output;
        };
        let token = &rest[start + 1..start + end];
        if token == "goal" {
            output.push_str(goal);
        } else if let Some((agent, _)) = roles.iter().find(|(_, slot)| slot.as_str() == token) {
            output.push_str(agent);
        } else {
            output.push_str(&rest[start..=start + end]);
        }
        rest = &rest[start + end + 1..];
    }
    output.push_str(rest);
    output
}
pub fn charter(t: &Template, goal: &str, roles: &Assignments) -> String {
    let mut result = expand(&t.charter, goal, roles);
    for r in &t.roles {
        let agents = roles
            .iter()
            .filter(|(_, slot)| *slot == &r.slot)
            .map(|(a, _)| a.as_str())
            .collect::<Vec<_>>()
            .join("、");
        result.push_str(&format!(
            "\n## {}（{}）\n职责：{}\n允许：{}\n禁止：{}\n可写范围：{}\n",
            r.slot,
            agents,
            r.duties,
            r.allow.join("、"),
            r.deny.join("、"),
            if r.write.is_empty() {
                "不限制".into()
            } else {
                r.write.join("、")
            }
        ));
    }
    format!("{}\n\n{}", result.trim_end(), COMMON.trim_end())
}
