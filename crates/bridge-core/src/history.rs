use crate::{
    database::query,
    format::{recipient, text},
    Bridge,
};
use anyhow::{bail, Result};
use rusqlite::params_from_iter;
use serde_json::Value;

impl Bridge {
    pub fn history(
        &self,
        project: &str,
        limit: i64,
        agent: Option<&str>,
        kind: Option<&str>,
    ) -> Result<String> {
        if limit < 0 {
            bail!("limit 不能小于 0");
        }
        let mut conditions = vec!["project IN (?, 'global')"];
        let mut values = vec![rusqlite::types::Value::Text(project.into())];
        for (condition, value) in [("agent = ?", agent), ("kind = ?", kind)] {
            if let Some(value) = value {
                conditions.push(condition);
                values.push(rusqlite::types::Value::Text(value.into()));
            }
        }
        values.push(rusqlite::types::Value::Integer(limit));
        let sql = format!("SELECT * FROM (SELECT * FROM events WHERE {} ORDER BY created_at DESC, id DESC LIMIT ?) ORDER BY created_at, id", conditions.join(" AND "));
        let rows = query(&self.database.open()?, &sql, params_from_iter(values))?;
        rows.iter()
            .map(format_event)
            .collect::<Result<Vec<_>>>()
            .map(|lines| lines.join("\n"))
    }
}

pub(crate) fn format_event(row: &Value) -> Result<String> {
    let data: Value = serde_json::from_str(text(row, "detail"))?;
    let action = match text(row, "kind") {
        "status" => {
            let fields = [
                ("任务", "task"),
                ("进度", "progress"),
                ("卡点", "blockers"),
                ("下一步", "next_step"),
            ];
            format!(
                "更新状态：{}",
                fields
                    .iter()
                    .filter(|(_, key)| !text(&data, key).is_empty())
                    .map(|(label, key)| format!("{label} {}", text(&data, key)))
                    .collect::<Vec<_>>()
                    .join("｜")
            )
        }
        "message" => format!(
            "→ {} 留言：{}",
            recipient(text(&data, "recipient")),
            text(&data, "content")
        ),
        "claim" => {
            let note = text(&data, "note");
            let note = if note.is_empty() {
                String::new()
            } else {
                format!("（{note}）")
            };
            format!(
                "认领：{}{note}，到期 {}",
                text(&data, "path"),
                text(&data, "expires_at")
            )
        }
        "release" => format!("释放认领：{}", text(&data, "path")),
        "expire" => format!("的认领已到期：{}", text(&data, "path")),
        "agent_switch" => format!(
            "对 {} {} Bridge",
            text(&data, "agent"),
            if data["enabled"].as_i64().unwrap_or(0) != 0 {
                "开启"
            } else {
                "关闭"
            }
        ),
        "init" => {
            let roles: crate::templates::Assignments =
                serde_json::from_value(data["roles"].clone())?;
            format!(
                "初始化协作：{}，{}",
                text(&data, "template"),
                crate::initialization::role_summary(&roles)
            )
        }
        "role" => format!(
            "把 {} 的职务改为 {}",
            text(&data, "agent"),
            text(&data, "slot")
        ),
        "handover" => format!("交接给 {}：独立开发", text(&data, "to")),
        "action_done" => format!(
            "已处理 {} 的消息 #{}",
            text(&data, "sender"),
            data["message_id"]
        ),
        "blocker_ack" => format!(
            "已知道 {} 的卡点：{}",
            text(&data, "agent"),
            text(&data, "blockers")
        ),
        "forget" => "从列表移除项目（协作已关闭，数据保留）".into(),
        "permission" => format!(
            "{}职务 {} 的权限",
            if data["reset"].as_bool().unwrap_or(false) {
                "重置"
            } else {
                "修改"
            },
            text(&data, "slot")
        ),
        "switch" => format!(
            "{}{}",
            if data["enabled"].as_i64().unwrap_or(0) != 0 {
                "开启"
            } else {
                "关闭"
            },
            if text(&data, "scope") == "global" {
                "全局总开关"
            } else {
                "项目"
            }
        ),
        kind => bail!("未知历史事件：{kind}"),
    };
    Ok(format!(
        "[{}] {} {action}",
        text(row, "created_at"),
        crate::format::sender(text(row, "agent"), text(&data, "via"))
    ))
}
