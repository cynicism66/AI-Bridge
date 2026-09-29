use super::*;
use crate::{database::Database, test_support::Directory};

#[test]
fn cursor_filters_old_read_self_other_recipient_and_other_project_without_writes() -> Result<()> {
    let d = Directory::new();
    let b = Bridge {
        database: Database::for_test(d.0.join("wait.db")),
        agent: "codex".into(),
        session_id: "unused".into(),
    };
    b.set_enabled(true, Some("/p"))?;
    b.send("/p", "human", "all", "旧消息")?;
    let mut reader = Reader::new(&b, "/p", "codex")?;
    b.send("/p", "human", "codex", "已读")?;
    b.read("/p", "codex", None, false)?;
    b.send("/p", "codex", "all", "自己")?;
    b.send("/p", "human", "claude", "给别人")?;
    b.send("/else", "human", "all", "另一个项目")?;
    let wanted = b.send("/p", "human", "all", "新消息")?;
    let conn = b.database.read_only()?;
    let before = query(&conn, "SELECT * FROM reads", [])?;
    let Poll::Messages(messages) = reader.poll()? else {
        panic!("意外关闭");
    };
    assert_eq!(messages.iter().map(|m| m.id).collect::<Vec<_>>(), [wanted]);
    assert!(matches!(reader.poll()?,Poll::Messages(m) if m.is_empty()));
    assert_eq!(query(&conn, "SELECT * FROM reads", [])?, before);
    assert!(query(&conn, "SELECT * FROM sessions", [])?.is_empty());
    assert!(reader.conn.execute("DELETE FROM messages", []).is_err());
    b.set_enabled(false, Some("/p"))?;
    assert!(matches!(reader.poll()?,Poll::Disabled(s) if s.contains("项目")));
    b.set_enabled(false, None)?;
    assert!(matches!(reader.poll()?,Poll::Disabled(s) if s.contains("全局")));
    Ok(())
}

#[test]
fn unicode_output_is_one_line_and_limits_content_not_bytes() {
    let m = Message {
        id: 42,
        sender: "human".into(),
        recipient: "all".into(),
        content: format!("甲\r\n乙\n丙\r丁\u{2028}{}", "🦀".repeat(250)),
        via: "gui".into(),
    };
    let line = line(&m);
    let (header, content) = line.split_once('：').unwrap();
    assert_eq!(header, "新消息 #42 human（软件界面） → 所有人");
    assert!(content.starts_with("甲 乙 丙 丁 "));
    assert_eq!(content.chars().count(), 200);
    assert_eq!(line.lines().count(), 1);
}
