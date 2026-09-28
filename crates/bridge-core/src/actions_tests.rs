use crate::{
    database::{Database, MIGRATIONS},
    initialization::InitOptions,
    test_support::{key, Directory},
    Bridge,
};
use anyhow::Result;
use serde_json::json;
fn fixture() -> Result<(Directory, Bridge, String)> {
    let d = Directory::new();
    let p = key(&d.dir("project"));
    let b = Bridge {
        database: Database::for_test(d.0.join("db")),
        agent: "codex".into(),
        session_id: "action-test".into(),
    };
    b.set_enabled(true, Some(&p))?;
    b.init_text(
        &p,
        InitOptions {
            template: "任务书流程",
            template_file: None,
            roles: &["规划审查=claude".into(), "执行=codex".into()],
            goal: "测试",
            write_rules: false,
            no_kickoff: true,
        },
    )?;
    Ok((d, b, p))
}
#[test]
fn action_message_read_does_not_resolve_and_manual_done_is_project_scoped_idempotent() -> Result<()>
{
    let (_d, b, p) = fixture()?;
    assert!(b.send_action(&p, "all", "请确认", true).is_err());
    let id = b.send_action(&p, "human", "请确认", true)?;
    b.mark_human_read(&p, id)?;
    assert_eq!(b.pending_actions(&p)?.len(), 1);
    b.resolve_action("other", "message", id)?;
    assert_eq!(b.pending_actions(&p)?.len(), 1);
    b.resolve_action(&p, "message", id)?;
    b.resolve_action(&p, "message", id)?;
    assert!(b.pending_actions(&p)?.is_empty());
    let history = b.history(&p, 999, None, None)?;
    assert_eq!(history.matches("已处理").count(), 1);
    Ok(())
}
#[test]
fn replies_resolve_only_earlier_messages_to_the_replied_ai() -> Result<()> {
    let (_d, b, p) = fixture()?;
    let id = b.send_action(&p, "human", "请确认", true)?;
    let conn = b.database.open()?;
    conn.execute(
        "UPDATE messages SET created_at='2090-01-01 00:00:00' WHERE id=?",
        [id],
    )?;
    for (to, stamp, expected) in [
        ("claude", "2090-01-02 00:00:00", 1),
        ("codex", "2090-01-01 00:00:00", 1),
        ("codex", "2090-01-02 00:00:00", 0),
    ] {
        conn.execute("INSERT INTO messages(project,sender,recipient,content,created_at,via) VALUES (?,'human',?,'确认',?,'gui')",rusqlite::params![p,to,stamp])?;
        assert_eq!(b.pending_actions(&p)?.len(), expected);
    }
    Ok(())
}
#[test]
fn blockers_acknowledgement_survives_same_content_but_not_changes_clear_or_expiry() -> Result<()> {
    let (_d, b, p) = fixture()?;
    let update = |blockers: &str| {
        b.tool(
            "update_status",
            &json!({"project":p,"task":"测试","blockers":blockers}),
        )
    };
    update("需要决定")?;
    let first = b.pending_actions(&p)?[0].id;
    b.resolve_action(&p, "blocker", first)?;
    update("需要决定")?;
    assert!(b.pending_actions(&p)?.is_empty());
    update("新的决定")?;
    let second = b.pending_actions(&p)?[0].id;
    assert_ne!(first, second);
    b.resolve_action(&p, "blocker", first)?;
    assert_eq!(b.pending_actions(&p)?.len(), 1);
    update("需要决定")?;
    assert_eq!(b.pending_actions(&p)?.len(), 1);
    update("")?;
    assert!(b.pending_actions(&p)?.is_empty());
    update("再次决定")?;
    b.database
        .open()?
        .execute("UPDATE sessions SET last_active='2000-01-01 00:00:00'", [])?;
    assert!(b.pending_actions(&p)?.is_empty());
    assert!(b.history(&p, 999, None, None)?.contains("已知道"));
    Ok(())
}
#[test]
fn version_seven_migrates_without_losing_messages_or_read_timestamps() -> Result<()> {
    let d = Directory::new();
    let path = d.0.join("db");
    let conn = rusqlite::Connection::open(&path)?;
    for sql in &MIGRATIONS[..7] {
        conn.execute_batch(sql)?;
    }
    conn.execute_batch("PRAGMA user_version=7; INSERT INTO messages(project,sender,recipient,content,created_at) VALUES ('p','codex','human','old','2090-01-01 00:00:00'); INSERT INTO reads(message_id,agent,read_at) VALUES (1,'human','2090-01-02 00:00:00');")?;
    drop(conn);
    let db = Database::for_test(path);
    let c = db.open()?;
    assert_eq!(
        c.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?,
        8
    );
    assert_eq!(
        c.query_row("SELECT needs_action FROM messages WHERE id=1", [], |r| r
            .get::<_, i64>(0))?,
        0
    );
    assert_eq!(
        c.query_row("SELECT read_at FROM reads", [], |r| r.get::<_, String>(0))?,
        "2090-01-02 00:00:00"
    );
    Ok(())
}
