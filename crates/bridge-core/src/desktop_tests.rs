use super::*;
use crate::{app_settings::AppSettings, database::Database, test_support::Directory};

#[test]
fn structured_messages_switches_and_change_detection() -> Result<()> {
    let d = Directory::new();
    let b = Bridge {
        database: Database::for_test(d.0.join("test.db")),
        agent: "codex".into(),
        session_id: "test".into(),
    };
    let watcher = ChangeDetector::new(&b)?;
    let before = watcher.version()?;
    b.set_enabled(true, Some("/test"))?;
    assert_ne!(watcher.version()?, before);
    let before = watcher.version()?;
    assert_eq!(watcher.version()?, before);
    b.send("/test", "claude", "human", "用户消息")?;
    b.send("/test", "codex", "claude", "AI 私信")?;
    b.post_human("/test", "命令行", "codex", HumanVia::Cli)?;
    b.post_human("/test", "界面", "all", HumanVia::Gui)?;
    let projects = b.project_list()?;
    assert_eq!(projects.len(), 1);
    assert!(projects[0].enabled);
    assert!(!projects[0].initialized);
    assert_eq!(projects[0].unread, 1);
    let p = b.message_page("/test", None, 2)?;
    assert_eq!(
        p.messages
            .iter()
            .map(|m| m.via.as_str())
            .collect::<Vec<_>>(),
        ["cli", "gui"]
    );
    let old = b.message_page("/test", p.before, 2)?;
    assert_eq!(old.messages.len(), 2);
    assert!(old.before.is_none());
    assert_eq!(old.messages[1].recipient, "claude");
    b.mark_human_read("/test", 1)?;
    assert_eq!(b.project_list()?[0].unread, 0);
    assert_eq!(b.read("/test", "codex", None, false)?.len(), 2);
    b.send("/test", "bridge", "all", "新消息")?;
    b.mark_human_read("/test", 1)?;
    assert_eq!(b.project_list()?[0].unread, 1);
    b.set_agent_enabled("/test", "codex", false)?;
    let detail = b.project_detail("/test")?;
    assert!(!detail.agents[0].enabled);
    assert!(detail.charter.is_none());
    assert_eq!(b.notification_messages(0)?.len(), 2);
    assert_eq!(b.latest_message_id()?, 5);
    assert!(b
        .history("/test", 100, None, None)?
        .contains("human（软件界面）"));
    assert!(b
        .history("/test", 100, None, None)?
        .contains("human（命令行）"));
    Ok(())
}

#[test]
fn settings_round_trip_and_invalid_data_preserved() -> Result<()> {
    let d = Directory::new();
    let path = AppSettings::path(&d.0);
    let mut s = AppSettings::load(&path)?;
    assert!(s.notifications_enabled);
    assert!(
        serde_json::from_str::<AppSettings>(r#"{"close_tip_shown":true}"#)?.notifications_enabled
    );
    s.notifications_enabled = false;
    s.hidden_projects.insert("/hidden".into());
    s.close_tip_shown = true;
    s.last_notified_id = Some(80);
    s.save(&path)?;
    let actual = AppSettings::load(&path)?;
    assert_eq!(actual.last_notified_id, Some(80));
    assert!(!actual.notifications_enabled);
    assert!(actual.close_tip_shown);
    assert!(actual.hidden_projects.contains("/hidden"));
    std::fs::write(&path, "invalid")?;
    assert!(AppSettings::load(&path).is_err());
    assert_eq!(std::fs::read_to_string(path)?, "invalid");
    Ok(())
}
