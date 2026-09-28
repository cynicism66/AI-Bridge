use crate::{
    database::query,
    desktop::{Message, ReadReceipt},
    format::text,
};
use anyhow::Result;
use rusqlite::{params, Connection};
use std::collections::{BTreeMap, BTreeSet};

pub(crate) fn attach(conn: &Connection, project: &str, messages: &mut [Message]) -> Result<()> {
    if messages.is_empty() {
        return Ok(());
    }
    let mut names: BTreeSet<String> = query(conn,
        "SELECT agent FROM sessions WHERE project=?1 UNION SELECT agent FROM status WHERE project=?1
         UNION SELECT agent FROM agent_settings WHERE project=?1 UNION SELECT agent FROM role_assignments WHERE project=?1
         UNION SELECT sender AS agent FROM messages WHERE project=?1 UNION SELECT recipient AS agent FROM messages WHERE project=?1
         UNION SELECT r.agent FROM reads r JOIN messages m ON m.id=r.message_id WHERE m.project=?1", [project])?
        .iter().map(|r| text(r,"agent").to_owned()).filter(|a| !matches!(a.as_str(), ""|"all"|"bridge"|"unknown")).collect();
    names.insert("human".into());
    let first = messages.first().unwrap().id;
    let last = messages.last().unwrap().id;
    let reads: BTreeMap<(i64,String),Option<String>> = query(conn,
        "SELECT r.* FROM reads r JOIN messages m ON m.id=r.message_id WHERE m.project=? AND m.id BETWEEN ? AND ?",
        params![project,first,last])?.into_iter().map(|r| ((r["message_id"].as_i64().unwrap(),text(&r,"agent").into()),r["read_at"].as_str().map(str::to_owned))).collect();
    for m in messages {
        let recipients = if m.recipient == "all" {
            names.iter().map(String::as_str).collect::<Vec<_>>()
        } else {
            vec![m.recipient.as_str()]
        };
        m.receipts = recipients
            .into_iter()
            .filter(|a| *a != m.sender)
            .map(|agent| {
                let stamp = reads.get(&(m.id, agent.to_owned()));
                ReadReceipt {
                    agent: agent.into(),
                    read: stamp.is_some(),
                    read_at: stamp.cloned().flatten(),
                }
            })
            .collect();
    }
    Ok(())
}

#[cfg(test)]
#[path = "message_receipts_tests.rs"]
mod tests;
