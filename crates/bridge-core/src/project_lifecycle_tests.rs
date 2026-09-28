use crate::{
    app_settings::AppSettings,
    database::Database,
    initialization::InitOptions,
    test_support::{key, Directory},
    Bridge,
};
use anyhow::Result;
use rusqlite::params;

fn fixture() -> Result<(Directory, Bridge, String, String, std::path::PathBuf)> {
    let dir = Directory::new();
    let a = key(&dir.dir("project-a"));
    let b = key(&dir.dir("project-b"));
    let bridge = Bridge {
        database: Database::for_test(dir.0.join("db")),
        agent: "codex".into(),
        session_id: "test".into(),
    };
    let settings = AppSettings::path(&dir.dir("home"));
    for project in [&a, &b] {
        bridge.set_enabled(true, Some(project))?;
        bridge.init_text(
            project,
            InitOptions {
                template: "独立开发",
                template_file: None,
                roles: &["独立开发=codex".into()],
                goal: "测试",
                write_rules: true,
                no_kickoff: false,
            },
        )?;
        let conn = bridge.database.open()?;
        conn.execute("INSERT INTO sessions VALUES (?,?,'codex',1,?,'main',1,'2099-01-01 00:00:00','2099-01-01 00:00:00',1)",params![project,project,project])?;
        conn.execute(
            "INSERT INTO status VALUES (?,'codex','任务','','需要确认','','2099-01-01 00:00:00',1,?)",
            params![project, project],
        )?;
        conn.execute("INSERT INTO claims VALUES (?,'held','codex','','2099-01-01 00:00:00','2099-01-02 00:00:00',1,?)",params![project,project])?;
        conn.execute("INSERT INTO agent_settings VALUES (?,'codex',1)", [project])?;
        conn.execute(
            "INSERT INTO permission_overrides VALUES (?,'独立开发','[]','[]','[]')",
            [project],
        )?;
        conn.execute(
            "INSERT INTO reads (message_id,agent) SELECT id,'codex' FROM messages WHERE project=?",
            [project],
        )?;
    }
    Ok((dir, bridge, a, b, settings))
}

fn counts(bridge: &Bridge, project: &str) -> Result<Vec<i64>> {
    let conn = bridge.database.open()?;
    // 枚举真实表，新增表时此测试也会要求清理，避免手写清单漏测。
    let tables = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?.query_map([], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    tables
        .iter()
        .map(|table| {
            let predicate = if table == "reads" {
                "message_id IN (SELECT id FROM messages WHERE project=?)"
            } else if table == "settings" {
                "scope=?"
            } else {
                "project=?"
            };
            Ok(conn.query_row(
                &format!("SELECT COUNT(*) FROM {table} WHERE {predicate}"),
                [project],
                |r| r.get(0),
            )?)
        })
        .collect()
}

type ProjectRows = Vec<Vec<Vec<rusqlite::types::Value>>>;
fn snapshot(bridge: &Bridge, project: &str) -> Result<ProjectRows> {
    let conn = bridge.database.open()?;
    let tables = conn.prepare("SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name")?.query_map([], |r| r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
    tables
        .iter()
        .map(|table| {
            let predicate = match table.as_str() {
                "reads" => "message_id IN (SELECT id FROM messages WHERE project=?)",
                "settings" => "scope=?",
                _ => "project=?",
            };
            let mut stmt = conn.prepare(&format!(
                "SELECT * FROM {table} WHERE {predicate} ORDER BY rowid"
            ))?;
            let columns = stmt.column_count();
            let rows = stmt
                .query_map([project], |row| (0..columns).map(|i| row.get(i)).collect())?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            Ok(rows)
        })
        .collect()
}

#[test]
fn forget_and_add_preserve_data_files_and_other_settings() -> Result<()> {
    let (dir, b, a, other, settings) = fixture()?;
    let files = ["AGENTS.md", "CLAUDE.md"]
        .map(|name| std::fs::read(std::path::Path::new(&a).join(name)).unwrap());
    let mut prefs = AppSettings {
        notifications_enabled: false,
        ..AppSettings::default()
    };
    prefs.hidden_projects.insert(other.clone());
    prefs.save(&settings)?;
    let before = counts(&b, &a)?;
    let other_before = counts(&b, &other)?;
    b.forget_project(&a, &settings)?;
    assert!(!b.switch_state(Some(&a))?.project_enabled);
    assert!(AppSettings::load(&settings)?.hidden_projects.contains(&a));
    let after = counts(&b, &a)?;
    // 只有事件数增长，任何已有表行都没有丢失。
    assert!(before.iter().zip(after).all(|(old, new)| new >= *old));
    assert!(b
        .history(&a, 100, None, Some("forget"))?
        .contains("从列表移除"));
    b.add_project(&a, &settings)?;
    assert!(!b.switch_state(Some(&a))?.project_enabled);
    assert!(b.switch_state(Some(&a))?.initialized);
    let prefs = AppSettings::load(&settings)?;
    assert!(!prefs.hidden_projects.contains(&a));
    assert!(prefs.hidden_projects.contains(&other));
    assert!(!prefs.notifications_enabled);
    assert_eq!(counts(&b, &other)?, other_before);
    for (name, bytes) in ["AGENTS.md", "CLAUDE.md"].into_iter().zip(files) {
        assert_eq!(std::fs::read(std::path::Path::new(&a).join(name))?, bytes);
    }
    assert!(dir.0.is_dir());
    Ok(())
}

#[test]
fn forget_rolls_back_if_settings_cannot_be_saved() -> Result<()> {
    let (d, b, p, _, _) = fixture()?;
    let before = counts(&b, &p)?;
    let settings = d.dir("invalid-settings");
    assert!(b.forget_project(&p, &settings).is_err());
    assert!(b.switch_state(Some(&p))?.project_enabled);
    assert_eq!(counts(&b, &p)?, before);
    Ok(())
}

#[test]
fn purge_clears_every_project_table_but_preserves_other_project_and_files() -> Result<()> {
    let (dir, b, p, other, settings) = fixture()?;
    let other_before = snapshot(&b, &other)?;
    let mut prefs = AppSettings {
        notifications_enabled: false,
        last_notified_id: Some(42),
        ..AppSettings::default()
    };
    prefs.hidden_projects.insert(other.clone());
    prefs.save(&settings)?;
    let claude = dir.0.join("home/.claude.json");
    let codex = dir.dir("home/.codex").join("config.toml");
    std::fs::write(&claude, br#"{"projects":{"test":{}},"unchanged":true}"#)?;
    std::fs::write(&codex, b"[projects.test]\ntrust_level = 'trusted'\n")?;
    let config_bytes = [std::fs::read(&claude)?, std::fs::read(&codex)?];
    assert!(counts(&b, &p)?.iter().all(|n| *n > 0));
    let root = std::path::Path::new(&p);
    std::fs::write(root.join("keep.txt"), b"untouched")?;
    let rules = [
        std::fs::read(root.join("AGENTS.md"))?,
        std::fs::read(root.join("CLAUDE.md"))?,
    ];
    assert!(b
        .purge_project(&p, &settings, true, true)?
        .contains("已彻底删除"));
    let conn = b.database.open()?;
    assert!(counts(&b, &p)?.iter().all(|n| *n == 0));
    assert_eq!(snapshot(&b, &other)?, other_before);
    let after = AppSettings::load(&settings)?;
    assert!(after.hidden_projects.contains(&p));
    assert!(after.hidden_projects.contains(&other));
    assert!(!after.notifications_enabled);
    assert_eq!(after.last_notified_id, Some(42));
    assert_eq!(
        [std::fs::read(&claude)?, std::fs::read(&codex)?],
        config_bytes
    );
    let orphan_reads: i64 = conn.query_row(
        "SELECT COUNT(*) FROM reads WHERE message_id NOT IN (SELECT id FROM messages)",
        [],
        |r| r.get(0),
    )?;
    assert_eq!(orphan_reads, 0);
    assert_eq!(std::fs::read(root.join("keep.txt"))?, b"untouched");
    assert_eq!(
        [
            std::fs::read(root.join("AGENTS.md"))?,
            std::fs::read(root.join("CLAUDE.md"))?
        ],
        rules
    );
    assert_eq!(
        conn.query_row("PRAGMA user_version", [], |r| r.get::<_, i64>(0))?,
        8
    );
    b.add_project(&p, &settings)?;
    assert!(!AppSettings::load(&settings)?.hidden_projects.contains(&p));
    assert!(!b.switch_state(Some(&p))?.initialized);
    assert!(!b.switch_state(Some(&p))?.project_enabled);
    Ok(())
}

#[test]
fn purge_settings_failure_rolls_back_every_row() -> Result<()> {
    let (dir, b, p, _, _) = fixture()?;
    let before = snapshot(&b, &p)?;
    assert!(b
        .purge_project(&p, &dir.dir("invalid-settings"), true, true)
        .is_err());
    assert_eq!(snapshot(&b, &p)?, before);
    Ok(())
}

#[test]
fn purge_confirmation_and_failure_are_atomic() -> Result<()> {
    let (_d, b, p, _, _) = fixture()?;
    let before = counts(&b, &p)?;
    let mut conn = b.database.open()?;
    for (yes, force) in [(false, false), (false, true), (true, false)] {
        let tx = conn.transaction()?;
        assert!(crate::project_lifecycle::purge_data(&tx, &p, yes, force).is_err());
    }
    assert_eq!(counts(&b, &p)?, before);
    conn.execute_batch("CREATE TRIGGER fail_purge BEFORE DELETE ON project_init BEGIN SELECT RAISE(ABORT,'blocked'); END")?;
    {
        let tx = conn.transaction()?;
        assert!(crate::project_lifecycle::purge_data(&tx, &p, true, true).is_err());
    }
    assert_eq!(counts(&b, &p)?, before);
    conn.execute_batch("DROP TRIGGER fail_purge")?;
    conn.execute(
        "UPDATE sessions SET last_active='2000-01-01 00:00:00' WHERE project=?",
        [&p],
    )?;
    let tx = conn.transaction()?;
    crate::project_lifecycle::purge_data(&tx, &p, true, false)?;
    tx.commit()?;
    assert!(counts(&b, &p)?.iter().all(|n| *n == 0));
    Ok(())
}
