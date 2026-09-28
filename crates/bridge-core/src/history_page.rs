use crate::{database::query, format::text, history::format_event, Bridge};
use anyhow::{bail, Result};
use rusqlite::{params_from_iter, types::Value as SqlValue};
use serde::{Deserialize, Serialize};

#[derive(Default, Deserialize)]
#[serde(default)]
pub struct HistoryFilter {
    pub agents: Vec<String>,
    pub kinds: Vec<String>,
    pub from: String,
    pub until: String,
    pub keyword: String,
    pub before: Option<(String, i64)>,
}
#[derive(Serialize)]
pub struct HistoryEvent {
    pub id: i64,
    pub created_at: String,
    pub agent: String,
    pub kind: String,
    pub description: String,
    pub detail: serde_json::Value,
}
#[derive(Serialize)]
pub struct HistoryPage {
    pub events: Vec<HistoryEvent>,
    pub before: Option<(String, i64)>,
}
impl Bridge {
    pub fn history_page(&self, project: &str, filter: HistoryFilter) -> Result<HistoryPage> {
        let mut clauses = vec!["project IN (?, 'global')".into()];
        let mut values = vec![SqlValue::Text(project.into())];
        for (field, list) in [("agent", filter.agents), ("kind", filter.kinds)] {
            if !list.is_empty() {
                clauses.push(format!("{field} IN ({})", vec!["?"; list.len()].join(",")));
                values.extend(list.into_iter().map(SqlValue::Text));
            }
        }
        for (date, op, suffix) in [
            (&filter.from, ">=", " 00:00:00"),
            (&filter.until, "<=", " 23:59:59"),
        ] {
            if !date.is_empty() {
                if chrono::NaiveDate::parse_from_str(date, "%Y-%m-%d").is_err() {
                    bail!("日期格式必须是 YYYY-MM-DD");
                }
                clauses.push(format!("created_at {op} ?"));
                values.push(SqlValue::Text(format!("{date}{suffix}")));
            }
        }
        if let Some((stamp, id)) = filter.before {
            clauses.push("(created_at,id) < (?,?)".into());
            values.extend([SqlValue::Text(stamp), SqlValue::Integer(id)]);
        }
        let mut conn = self.database.open()?;
        let tx = conn.transaction()?;
        let base = clauses.join(" AND ");
        let mut events = Vec::new();
        let keyword = filter.keyword.to_lowercase();
        // 分块扫描描述匹配；不能先截取100条再过滤，从而漏掉更早的匹配项。
        let mut offset = 0;
        'scan: loop {
            let rows = query(&tx,&format!("SELECT * FROM events WHERE {base} ORDER BY created_at DESC,id DESC LIMIT 500 OFFSET {offset}"),params_from_iter(values.iter()))?;
            let count = rows.len();
            for r in rows {
                let description = format_event(&r)?;
                if description.to_lowercase().contains(&keyword) {
                    events.push(HistoryEvent {
                        id: r["id"].as_i64().unwrap_or(0),
                        created_at: text(&r, "created_at").into(),
                        agent: text(&r, "agent").into(),
                        kind: text(&r, "kind").into(),
                        detail: serde_json::from_str(text(&r, "detail"))?,
                        description,
                    });
                    if events.len() == 101 {
                        break 'scan;
                    }
                }
            }
            if count < 500 {
                break;
            }
            offset += 500;
        }
        let more = events.len() > 100;
        events.truncate(100);
        let before = if more {
            events.last().map(|e| (e.created_at.clone(), e.id))
        } else {
            None
        };
        tx.commit()?;
        Ok(HistoryPage { events, before })
    }
}
