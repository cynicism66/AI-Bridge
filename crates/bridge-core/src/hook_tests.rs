use super::*;
use crate::{
    database::{query, Database},
    test_support::Directory,
};
fn fixture() -> Result<(Directory, Bridge, Input, String)> {
    let d = Directory::new();
    d.write("repo/.git/HEAD", "ref: refs/heads/main");
    let cwd = d.0.join("repo").to_string_lossy().into_owned();
    let key = repository::resolve(&cwd)?.key;
    let b = Bridge {
        database: Database::for_test(d.0.join("test.db")),
        agent: "codex".into(),
        session_id: "mcp".into(),
    };
    b.set_enabled(true, Some(&key))?;
    Ok((
        d,
        b,
        Input {
            cwd,
            session_id: "window".into(),
            stop_hook_active: false,
        },
        key,
    ))
}
#[test]
fn failed_output_rolls_back_binding_and_reads() -> Result<()> {
    let (_d, b, input, key) = fixture()?;
    b.send(&key, "claude", "codex", "待审查")?;
    let result = deliver_at(&b, &input, "codex", "2090-01-02 03:00:00", |_| {
        anyhow::bail!("模拟管道断开")
    });
    assert!(result.is_err());
    let conn = b.database.read_only()?;
    assert!(query(&conn, "SELECT * FROM reads", [])?.is_empty());
    assert!(query(&conn, "SELECT * FROM hook_receivers", [])?.is_empty());
    let mut delivered = false;
    deliver_at(&b, &input, "codex", "2090-01-02 03:00:01", |v| {
        delivered = v["decision"] == "block";
        Ok(())
    })?;
    assert!(delivered);
    assert_eq!(query(&conn, "SELECT * FROM reads", [])?.len(), 1);
    Ok(())
}
#[test]
fn active_owner_heartbeat_and_rebind_work_even_when_disabled() -> Result<()> {
    let (_d, b, mut input, key) = fixture()?;
    b.rebind(&key, "codex")?;
    b.set_enabled(false, None)?;
    input.stop_hook_active = true;
    deliver_at(&b, &input, "codex", "2090-01-02 03:00:00", |_| {
        panic!("关闭时不可投递")
    })?;
    deliver_at(&b, &input, "codex", "2090-01-02 04:00:00", |_| {
        panic!("续跑时不可投递")
    })?;
    let rows = query(&b.database.read_only()?, "SELECT * FROM hook_receivers", [])?;
    assert_eq!(rows[0]["session_id"], "window");
    assert_eq!(rows[0]["last_active"], "2090-01-02 04:00:00");
    assert_eq!(rows[0]["pending_rebind"], 0);
    Ok(())
}
