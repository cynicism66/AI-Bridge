use crate::{app_settings::AppSettings, copy_options::*, test_support::Directory};
use anyhow::Result;
use std::{fs, process::Command};
#[test]
fn existing_projects_default_off_and_gitignore_preserves_bytes_and_negation() -> Result<()> {
    let d = Directory::new();
    let p = d.dir("repo");
    assert!(Command::new("git")
        .arg("init")
        .arg(&p)
        .output()?
        .status
        .success());
    let settings = d.0.join("app.json");
    assert!(AppSettings::load(&settings)?.record_copies.is_empty());
    let ignore = p.join(".gitignore");
    fs::write(&ignore, b"# comment\r\n*.tmp\r\n")?;
    set_copy_options(p.to_str().unwrap(), &settings, true, false)?;
    assert!(!protection(&p)?.ignored);
    assert_eq!(fs::read(&ignore)?, b"# comment\r\n*.tmp\r\n");
    add_ignore(p.to_str().unwrap())?;
    assert_eq!(fs::read(&ignore)?, b"# comment\r\n*.tmp\r\n.bridge/\r\n");
    add_ignore(p.to_str().unwrap())?;
    assert_eq!(fs::read(&ignore)?, b"# comment\r\n*.tmp\r\n.bridge/\r\n");
    fs::write(&ignore, b".*\n!.bridge/\n")?;
    assert!(!protection(&p)?.ignored);
    add_ignore(p.to_str().unwrap())?;
    assert!(protection(&p)?.ignored);
    fs::write(&ignore, b".*\n")?;
    add_ignore(p.to_str().unwrap())?;
    assert_eq!(fs::read(&ignore)?, b".*\n");
    let plain = d.dir("plain");
    add_ignore(plain.to_str().unwrap())?;
    assert!(!plain.join(".gitignore").exists());
    Ok(())
}
#[test]
fn failed_gitignore_confirmation_does_not_enable_copy() -> Result<()> {
    let d = Directory::new();
    let p = d.dir("repo");
    assert!(Command::new("git")
        .arg("init")
        .arg(&p)
        .output()?
        .status
        .success());
    fs::create_dir(p.join(".gitignore"))?;
    let settings = d.0.join("app.json");
    assert!(set_copy_options(p.to_str().unwrap(), &settings, true, true).is_err());
    assert!(!settings.exists());
    Ok(())
}

#[test]
fn init_preview_is_read_only_and_confirmation_commits_copy_options() -> Result<()> {
    use crate::{database::Database, initialization::InitOptions, test_support::key, Bridge};
    let d = Directory::new();
    let p = key(&d.dir("project"));
    let settings = d.0.join("app.json");
    let b = Bridge {
        database: Database::for_test(d.0.join("db")),
        agent: "codex".into(),
        session_id: "copy".into(),
    };
    b.set_enabled(true, Some(&p))?;
    let roles = vec!["规划审查=claude".into(), "执行=codex".into()];
    let mut draft = b.prepare_init(
        &p,
        InitOptions {
            template: "任务书流程",
            template_file: None,
            roles: &roles,
            goal: "测试",
            write_rules: false,
            no_kickoff: true,
        },
    )?;
    draft.stage_record_copy(&settings, true, false)?;
    assert!(!settings.exists());
    assert!(!std::path::Path::new(&p).join(".bridge").exists());
    draft.commit()?;
    assert!(AppSettings::load(&settings)?.record_copies[&p].enabled);
    b.purge_project(&p, &settings, true, true)?;
    assert!(!AppSettings::load(&settings)?.record_copies.contains_key(&p));
    Ok(())
}
