use crate::{
    database::Database,
    initialization::InitOptions,
    permission_edit::PermissionPatch,
    test_support::{key, Directory},
    Bridge,
};
use anyhow::Result;
use serde_json::json;

fn fixture() -> Result<(Directory, Bridge, String)> {
    let d = Directory::new();
    let project = key(&d.dir("project"));
    let b = Bridge {
        database: Database::for_test(d.0.join("db")),
        agent: "claude".into(),
        session_id: "current".into(),
    };
    b.set_enabled(true, Some(&project))?;
    b.init_text(
        &project,
        InitOptions {
            roles: &roles(),
            ..options()
        },
    )?;
    Ok((d, b, project))
}
fn options() -> InitOptions<'static> {
    InitOptions {
        template: "任务书流程",
        template_file: None,
        roles: &[],
        goal: "测试",
        write_rules: true,
        no_kickoff: true,
    }
}
// 每次构造赋值，避免共享环境变量或真实数据库。
fn roles() -> Vec<String> {
    vec!["规划审查=claude".into(), "执行=codex".into()]
}

#[test]
fn permissions_apply_to_claims_charter_roles_and_reset() -> Result<()> {
    let (_d, b, p) = fixture()?;
    b.permission_text(
        &p,
        "规划审查",
        PermissionPatch {
            write: Some(vec!["src/**".into()]),
            allow: Some(vec!["专用规则".into()]),
            ..Default::default()
        },
    )?;
    assert_eq!(b.management(&p)?["charter"]["version"], 2);
    assert!(b.claim_permission(&p, &["docs/a.md".into()])?.is_some());
    assert!(b.claim_permission(&p, &["src/a.rs".into()])?.is_none());
    assert!(std::fs::read_to_string(format!("{p}/AGENTS.md"))?.contains("专用规则"));
    let full = b.tool("bridge_overview", &json!({"project":p}))?;
    assert!(full.contains("== 协作章程 v2 =="));
    assert!(b
        .tool("bridge_overview", &json!({"project":p}))?
        .contains("本会话已显示过"));
    let before = b.management(&p)?;
    assert!(b
        .permission_text(
            &p,
            "规划审查",
            PermissionPatch {
                write: Some(vec!["../escape".into()]),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(before, b.management(&p)?);
    b.role_text(&p, "claude", "执行")?;
    assert!(b.management(&p)?["charter"]["summary"]
        .as_str()
        .unwrap()
        .contains("专用规则"));
    b.permission_text(
        &p,
        "规划审查",
        PermissionPatch {
            reset: true,
            ..Default::default()
        },
    )?;
    assert_eq!(
        b.management(&p)?["roles"][0]["write"],
        json!(["docs/**", "AGENTS.md"])
    );
    assert!(b
        .history(&p, 100, None, Some("permission"))?
        .contains("重置职务"));
    Ok(())
}

#[test]
fn init_preview_detects_database_and_file_changes_without_partial_writes() -> Result<()> {
    let (_d, b, p) = fixture()?;
    let r = roles();
    let draft = b.prepare_init(
        &p,
        InitOptions {
            roles: &r,
            ..options()
        },
    )?;
    b.send(&p, "codex", "human", "并发变化")?;
    assert!(draft
        .commit()
        .unwrap_err()
        .to_string()
        .contains("数据发生变化"));
    let draft = b.prepare_init(
        &p,
        InitOptions {
            roles: &r,
            ..options()
        },
    )?;
    std::fs::write(format!("{p}/AGENTS.md"), "用户新增内容")?;
    assert!(draft
        .commit()
        .unwrap_err()
        .to_string()
        .contains("目标文件发生变化"));
    assert_eq!(b.management(&p)?["charter"]["version"], 1);
    assert_eq!(
        std::fs::read_to_string(format!("{p}/AGENTS.md"))?,
        "用户新增内容"
    );
    std::fs::write(format!("{p}/AGENTS.md"), crate::rules_files::START)?;
    assert!(b
        .prepare_init(
            &p,
            InitOptions {
                roles: &r,
                ..options()
            }
        )
        .is_err());
    assert_eq!(b.management(&p)?["charter"]["version"], 1);
    Ok(())
}

#[test]
fn transfer_gui_preview_masks_and_rejects_stale_confirmations() -> Result<()> {
    let (_d, b, p) = fixture()?;
    b.send(
        &p,
        "codex",
        "human",
        "GITHUB_TOKEN=sample-secret author=Alice",
    )?;
    let draft = b.prepare_transfer(&p, None, None)?;
    assert!(!draft.view.body.contains("sample-secret"));
    assert!(draft.view.body.contains("author=Alice"));
    assert!(!std::path::Path::new(&format!("{p}/docs/bridge/协作记录.md")).exists());
    b.send(&p, "codex", "human", "changed")?;
    assert!(draft
        .commit()
        .unwrap_err()
        .to_string()
        .contains("数据发生变化"));
    let draft = b.prepare_transfer(&p, None, None)?;
    std::fs::create_dir_all(format!("{p}/docs/bridge"))?;
    std::fs::write(format!("{p}/docs/bridge/协作记录.md"), "user")?;
    assert!(draft
        .commit()
        .unwrap_err()
        .to_string()
        .contains("目标文件发生变化"));
    b.prepare_transfer(&p, None, None)?.commit()?;
    assert!(
        !std::fs::read_to_string(format!("{p}/docs/bridge/协作记录.md"))?.contains("sample-secret")
    );
    Ok(())
}

#[test]
fn permission_write_failure_restores_database_and_files() -> Result<()> {
    let (_d, b, p) = fixture()?;
    let before = b.management(&p)?;
    let first = std::fs::read(format!("{p}/AGENTS.md"))?;
    std::fs::write(format!("{p}/CLAUDE.md"), crate::rules_files::START)?;
    assert!(b
        .permission_text(
            &p,
            "执行",
            PermissionPatch {
                write: Some(vec!["src/**".into()]),
                ..Default::default()
            }
        )
        .is_err());
    assert_eq!(before, b.management(&p)?);
    assert_eq!(first, std::fs::read(format!("{p}/AGENTS.md"))?);
    Ok(())
}
