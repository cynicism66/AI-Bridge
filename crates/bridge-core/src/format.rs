use serde_json::Value;

pub(crate) fn text<'a>(row: &'a Value, key: &str) -> &'a str {
    row[key].as_str().unwrap_or("")
}
pub(crate) fn recipient(value: &str) -> &str {
    if value == "all" {
        "所有人"
    } else {
        value
    }
}

pub(crate) fn sender(agent: &str, via: &str) -> String {
    match (agent, via) {
        ("human", "gui") => "human（软件界面）".into(),
        ("human", "cli") => "human（命令行）".into(),
        _ => agent.into(),
    }
}

pub(crate) fn message(row: &Value) -> String {
    format!(
        "  #{} [{}] {} → {}：{}",
        row["id"],
        text(row, "created_at"),
        sender(text(row, "sender"), text(row, "via")),
        recipient(text(row, "recipient")),
        text(row, "content")
    )
}

pub(crate) fn messages(rows: &[Value], include_read: bool) -> String {
    if rows.is_empty() {
        return if include_read {
            "还没有任何消息。"
        } else {
            "没有未读消息。"
        }
        .into();
    }
    let heading = if include_read {
        "最近的消息：".into()
    } else {
        format!("未读消息（{} 条，已标为已读）：", rows.len())
    };
    format!(
        "{heading}\n{}",
        rows.iter().map(message).collect::<Vec<_>>().join("\n")
    )
}

pub(crate) fn status(row: &Value) -> String {
    let task = text(row, "task");
    let mut lines = vec![
        format!(
            "【{} #{}】分支 {} · 文件夹 {} · 更新于 {}",
            text(row, "agent"),
            row["session_no"],
            text(row, "branch"),
            text(row, "worktree"),
            text(row, "updated_at")
        ),
        format!("  任务：{}", if task.is_empty() { "-" } else { task }),
    ];
    for (label, key) in [
        ("进度", "progress"),
        ("卡点", "blockers"),
        ("下一步", "next_step"),
    ] {
        if !text(row, key).is_empty() {
            lines.push(format!("  {label}：{}", text(row, key)));
        }
    }
    lines.join("\n")
}

pub(crate) fn claim(row: &Value) -> String {
    let note = text(row, "note");
    let note = if note.is_empty() {
        String::new()
    } else {
        format!("（{note}）")
    };
    format!(
        "  {} ← {}{note}，到期 {}",
        text(row, "path"),
        text(row, "agent"),
        text(row, "expires_at")
    )
}

pub(crate) fn section(rows: &[Value], empty: &str, formatter: fn(&Value) -> String) -> String {
    if rows.is_empty() {
        empty.into()
    } else {
        rows.iter().map(formatter).collect::<Vec<_>>().join("\n")
    }
}

pub(crate) fn statuses(rows: &[Value], include_older: bool) -> anyhow::Result<String> {
    use chrono::NaiveDateTime;
    let now = NaiveDateTime::parse_from_str(&crate::now()?, crate::TIME_FMT)?;
    let mut lines = Vec::new();
    let mut older = Vec::new();
    for row in rows {
        let age = NaiveDateTime::parse_from_str(text(row, "last_active"), crate::TIME_FMT)
            .ok()
            .map(|t| now - t);
        if age.is_some_and(|age| age.num_seconds() > 2 * 3600) {
            older.push(status(row));
        } else {
            lines.push(status(row));
        }
    }
    if !older.is_empty() {
        lines.push(format!(
            "较早的会话（{}）{}",
            older.len(),
            if include_older {
                format!("\n{}", older.join("\n\n"))
            } else {
                "：已折叠；bridge_overview 传 include_older=true 或 show --include-older 可展开。"
                    .into()
            }
        ));
    }
    Ok(if lines.is_empty() {
        "  （还没有人汇报状态）".into()
    } else {
        lines.join("\n")
    })
}
