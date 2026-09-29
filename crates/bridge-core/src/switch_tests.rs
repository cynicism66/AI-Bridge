use crate::{database::Database, test_support::Directory, Bridge};
use anyhow::Result;
#[test]
fn two_switches_and_legacy_ai_setting_is_ignored() -> Result<()> {
    let tmp = Directory::new();
    let bridge = Bridge {
        database: Database::for_test(tmp.0.join("test.db")),
        agent: "codex".into(),
        session_id: "test".into(),
    };
    let initial = bridge.switch_state(Some("/p"))?;
    assert!(initial.global_enabled && !initial.project_enabled);
    assert!(!bridge.database.path.exists());
    bridge
        .database
        .open()?
        .execute("INSERT INTO agent_settings VALUES ('/p','codex',0)", [])?;
    for global in [false, true] {
        for project in [false, true] {
            bridge.set_enabled(global, None)?;
            bridge.set_enabled(project, Some("/p"))?;
            assert_eq!(
                bridge.switch_state(Some("/p"))?.enabled(),
                global && project
            );
        }
    }
    Ok(())
}
