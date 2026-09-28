use crate::{
    database::Database,
    desktop::{ChangeDetector, HumanVia},
    test_support::Directory,
    Bridge,
};
use anyhow::Result;

#[test]
fn recipients_read_times_and_project_isolation() -> Result<()> {
    let d = Directory::new();
    let b = Bridge {
        database: Database::for_test(d.0.join("reads.db")),
        agent: "codex".into(),
        session_id: "test".into(),
    };
    b.set_enabled(true, Some("/p"))?;
    b.set_agent_enabled("/p", "codex", true)?;
    b.set_agent_enabled("/p", "claude", true)?;
    b.set_agent_enabled("/other", "other-ai", true)?;
    let direct = b.post_human("/p", "直接", "claude", HumanVia::Gui)?;
    let broadcast = b.post_human("/p", "广播", "all", HumanVia::Cli)?;
    b.send("/p", "claude", "all", "回复")?;
    let page = b.message_page("/p", None, 100)?;
    assert_eq!(page.messages[0].receipts.len(), 1);
    assert_eq!(page.messages[0].receipts[0].agent, "claude");
    assert!(page.messages[1].receipts.iter().all(|r| !r.read));
    assert_eq!(
        page.messages[1]
            .receipts
            .iter()
            .map(|r| r.agent.as_str())
            .collect::<Vec<_>>(),
        ["claude", "codex"]
    );
    assert_eq!(
        page.messages[2]
            .receipts
            .iter()
            .map(|r| r.agent.as_str())
            .collect::<Vec<_>>(),
        ["codex", "human"]
    );
    let watcher = ChangeDetector::new(&b)?;
    let before = watcher.version()?;
    b.read("/p", "claude", None, false)?;
    assert_ne!(watcher.version()?, before);
    let page = b.message_page("/p", None, 100)?;
    let stamp = page.messages[0].receipts[0].read_at.clone();
    assert!(stamp.as_ref().is_some_and(|s| s.len() == 19));
    assert!(page.messages[0].receipts[0].read);
    assert!(page.messages[1].receipts[0].read);
    assert!(!page.messages[1].receipts[1].read);
    b.database.open()?.execute(
        "UPDATE reads SET read_at='2000-01-01 01:02:03' WHERE message_id=?",
        [direct],
    )?;
    b.read("/p", "claude", None, false)?;
    assert_eq!(
        b.message_page("/p", None, 100)?.messages[0].receipts[0]
            .read_at
            .as_deref(),
        Some("2000-01-01 01:02:03")
    );
    b.database.open()?.execute(
        "INSERT INTO reads (message_id,agent) VALUES (?,'codex')",
        [broadcast],
    )?;
    b.read("/p", "codex", None, false)?;
    let old = b.message_page("/p", None, 100)?.messages[1].receipts[1].clone();
    assert!(old.read);
    assert!(old.read_at.is_none());
    b.mark_human_read("/p", i64::MAX)?;
    let reply = &b.message_page("/p", None, 100)?.messages[2];
    assert!(reply.receipts[1].read_at.is_some());
    Ok(())
}
