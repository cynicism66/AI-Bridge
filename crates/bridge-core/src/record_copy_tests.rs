use crate::{
    database::Database,
    record_frames::{decode, HEADER},
    test_support::{key, Directory},
    Bridge,
};
use anyhow::Result;
use std::{fs, io::Write};
fn fixture() -> Result<(Directory, Bridge, String)> {
    let dir = Directory::new();
    let project = key(&dir.dir("project"));
    let bridge = Bridge {
        database: Database::for_test(dir.0.join("test.db")),
        agent: "codex".into(),
        session_id: "test".into(),
    };
    bridge.set_enabled(true, Some(&project))?;
    Ok((dir, bridge, project))
}
fn event(b: &Bridge, p: &str, stamp: &str, text: &str) -> Result<()> {
    b.database.open()?.execute(
        "INSERT INTO events(project,agent,kind,detail,created_at) VALUES (?,'codex','message',?,?)",
        rusqlite::params![
            p,
            serde_json::json!({"recipient":"human","content":text}).to_string(),
            stamp
        ],
    )?;
    Ok(())
}
#[test]
fn monthly_copy_matches_history_and_recovers_all_partial_frame_boundaries() -> Result<()> {
    let (_d, b, p) = fixture()?;
    event(
        &b,
        &p,
        "2090-01-02 00:00:00",
        "第一行\n<!-- bridge-event fake -->\n第二行",
    )?;
    event(&b, &p, "2090-02-02 00:00:00", "二月")?;
    b.sync_record_copy(&p)?;
    let path = format!("{p}/.bridge/协作记录-2090-01.md");
    let good = fs::read(&path)?;
    let (records, end) = decode(&good)?;
    assert_eq!(end, good.len());
    assert_eq!(records.len(), 1);
    let all = b.history(&p, 999, None, None)?;
    assert!(all.contains(records[0].body.trim_end().replace("\n  ", "\n").as_str()));
    for cut in HEADER.len()..good.len() {
        fs::write(&path, &good[..cut])?;
        b.sync_record_copy(&p)?;
        assert_eq!(fs::read(&path)?, good, "cut={cut}");
    }
    b.sync_record_copy(&p)?;
    assert_eq!(fs::read(&path)?, good);
    assert!(fs::metadata(format!("{p}/.bridge/协作记录-2090-02.md")).is_ok());
    Ok(())
}
#[test]
fn off_pauses_and_on_catches_up_and_older_time_preserves_order() -> Result<()> {
    let (_d, b, p) = fixture()?;
    event(&b, &p, "2090-01-03 00:00:00", "三日")?;
    b.sync_record_copy(&p)?;
    let path = format!("{p}/.bridge/协作记录-2090-01.md");
    let old = fs::read(&path)?;
    b.set_enabled(false, Some(&p))?;
    event(&b, &p, "2090-01-02 00:00:00", "二日")?;
    assert_eq!(b.sync_record_copy(&p)?, None);
    assert_eq!(fs::read(&path)?, old);
    b.set_enabled(true, Some(&p))?;
    b.sync_record_copy(&p)?;
    let (records, _) = decode(&fs::read(path)?)?;
    assert_eq!(records.len(), 2);
    assert!(records[0].body.contains("二日"));
    b.set_enabled(false, None)?;
    assert_eq!(b.sync_record_copy(&p)?, None);
    Ok(())
}
#[test]
fn failures_preserve_corrupt_files_and_do_not_prevent_other_projects() -> Result<()> {
    let (d, b, p) = fixture()?;
    fs::write(format!("{p}/.bridge"), "不是目录")?;
    assert!(b.sync_record_copy(&p).is_err());
    let other = key(&d.dir("other"));
    b.set_enabled(true, Some(&other))?;
    event(&b, &other, "2090-01-01 00:00:00", "正常")?;
    b.sync_record_copy(&other)?;
    let path = format!("{other}/.bridge/协作记录-2090-01.md");
    let mut f = fs::OpenOptions::new().append(true).open(&path)?;
    f.write_all(b"corrupt\n")?;
    drop(f);
    let old = fs::read(&path)?;
    assert!(b.sync_record_copy(&other).is_err());
    assert_eq!(fs::read(&path)?, old);
    assert!(crate::repo_files::index(std::path::Path::new(&other))?.contains("本地未打码"));
    Ok(())
}

#[cfg(windows)]
#[test]
fn readonly_copy_reports_error_without_losing_data_then_catches_up() -> Result<()> {
    let (_d, b, p) = fixture()?;
    event(&b, &p, "2090-01-01 00:00:00", "先写入")?;
    b.sync_record_copy(&p)?;
    let path = std::path::PathBuf::from(format!("{p}/.bridge/协作记录-2090-01.md"));
    let old = fs::read(&path)?;
    let original = fs::metadata(&path)?.permissions();
    let mut readonly = original.clone();
    readonly.set_readonly(true);
    fs::set_permissions(&path, readonly)?;
    event(&b, &p, "2090-01-02 00:00:00", "稍后补齐")?;
    let result = b.sync_record_copy(&p);
    let unchanged = fs::read(&path)? == old;
    fs::set_permissions(&path, original)?;
    assert!(result.is_err());
    assert!(unchanged);
    b.sync_record_copy(&p)?;
    assert_eq!(decode(&fs::read(path)?)?.0.len(), 2);
    Ok(())
}
