use crate::{database::Database, test_support::Directory, Bridge};
use anyhow::Result;
#[test]
fn every_three_level_switch_combination() -> Result<()> {
    let tmp = Directory::new();
    let bridge = Bridge {
        database: Database::for_test(tmp.0.join("test.db")),
        agent: "codex".into(),
        session_id: "test".into(),
    };
    let initial = bridge.access_state(Some("/p"), Some("codex"))?;
    assert!(initial.global_enabled && initial.agent_enabled && !initial.project_enabled);
    assert!(!bridge.database.path.exists());
    for global in [false, true] {
        for project in [false, true] {
            for agent in [false, true] {
                bridge.set_enabled(global, None)?;
                bridge.set_enabled(project, Some("/p"))?;
                bridge.agent_toggle_text("/p", "codex", agent)?;
                let state = bridge.access_state(Some("/p"), Some("codex"))?;
                assert_eq!(state.enabled(), global && project && agent);
                assert_eq!(
                    bridge.access_state(Some("/p"), Some("claude"))?.enabled(),
                    global && project
                );
            }
        }
    }
    Ok(())
}
