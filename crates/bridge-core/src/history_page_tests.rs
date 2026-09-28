use crate::{database::Database, history_page::HistoryFilter, test_support::Directory, Bridge};
use anyhow::Result;
use rusqlite::params;

#[test]
fn history_filters_before_paging_and_uses_stable_cursor() -> Result<()> {
    let d = Directory::new();
    let b = Bridge {
        database: Database::for_test(d.0.join("db")),
        agent: "codex".into(),
        session_id: "history".into(),
    };
    let conn = b.database.open()?;
    for n in 0..620 {
        let data=serde_json::json!({"recipient":"human","content":if n%2==0 {"needle"} else {"other"},"via":"mcp"}).to_string();
        conn.execute("INSERT INTO events(project,agent,kind,detail,created_at) VALUES('/p',?,'message',?,'2026-01-01 12:00:00')",params![if n%3==0 {"claude"} else {"codex"},data])?;
    }
    let filter = || HistoryFilter {
        agents: vec!["claude".into()],
        kinds: vec!["message".into()],
        from: "2026-01-01".into(),
        until: "2026-01-01".into(),
        keyword: "needle".into(),
        ..Default::default()
    };
    let first = b.history_page("/p", filter())?;
    assert_eq!(first.events.len(), 100);
    assert!(first
        .events
        .iter()
        .all(|e| e.agent == "claude" && e.description.contains("needle")));
    let second = b.history_page(
        "/p",
        HistoryFilter {
            before: first.before,
            ..filter()
        },
    )?;
    assert_eq!(second.events.len(), 4);
    assert!(second.before.is_none());
    assert!(second
        .events
        .iter()
        .all(|e| !first.events.iter().any(|a| a.id == e.id)));
    assert!(b
        .history_page(
            "/p",
            HistoryFilter {
                from: "bad".into(),
                ..Default::default()
            }
        )
        .is_err());
    Ok(())
}
