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

pub(crate) fn message(row: &Value) -> String {
    format!(
        "  #{} [{}] {} → {}：{}",
        row["id"],
        text(row, "created_at"),
        text(row, "sender"),
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
            "【{}】更新于 {}",
            text(row, "agent"),
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
