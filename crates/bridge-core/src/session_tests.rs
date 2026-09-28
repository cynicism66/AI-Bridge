use super::*;
#[test]
fn numbers_reuse_gaps_and_expire_after_two_hours() -> Result<()> {
    let mut conn = Connection::open_in_memory()?;
    crate::database::migrate(&mut conn)?;
    let p = Project {
        key: "/p".into(),
        worktree: "/p".into(),
        branch: "main".into(),
    };
    let t = "2026-01-01 00:00:00";
    assert_eq!(touch(&mut conn, "one", "codex", &p, t)?, 1);
    conn.execute(
        "INSERT INTO claims VALUES ('/p','held','codex','',?,'2030-01-01 00:00:00',1,'one')",
        [t],
    )?;
    assert_eq!(touch(&mut conn, "two", "codex", &p, t)?, 2);
    assert_eq!(touch(&mut conn, "three", "codex", &p, t)?, 3);
    conn.execute("DELETE FROM sessions WHERE id='two'", [])?;
    assert_eq!(touch(&mut conn, "four", "codex", &p, t)?, 2);
    assert_eq!(
        touch(&mut conn, "five", "codex", &p, "2026-01-01 02:00:00")?,
        4
    );
    assert_eq!(
        touch(&mut conn, "six", "codex", &p, "2026-01-01 02:00:01")?,
        1
    );
    assert_eq!(
        touch(&mut conn, "one", "codex", &p, "2026-01-01 02:00:02")?,
        2
    );
    assert_eq!(
        conn.query_row(
            "SELECT session_no FROM claims WHERE session_id='one'",
            [],
            |r| r.get::<_, i64>(0)
        )?,
        2
    );
    assert_eq!(touch(&mut conn, "claude", "claude", &p, t)?, 1);
    assert_eq!(touch(&mut conn, "three", "codex", &p, t)?, 3);
    Ok(())
}
#[test]
fn random_identifiers_are_distinct() -> Result<()> {
    let a = random_id()?;
    let b = random_id()?;
    assert_eq!(a.len(), 32);
    assert_ne!(a, b);
    Ok(())
}

#[test]
fn reuse_removes_legacy_status_but_preserves_claim_ownership_and_history() -> Result<()> {
    let mut conn = Connection::open_in_memory()?;
    crate::database::migrate(&mut conn)?;
    let p = Project {
        key: "/p".into(),
        worktree: "/p".into(),
        branch: "main".into(),
    };
    let legacy = "legacy:/p:claude";
    let old = "2026-01-01 00:00:00";
    touch(&mut conn, legacy, "claude", &p, old)?;
    conn.execute(
        "INSERT INTO status VALUES ('/p','claude','旧任务','','','',?,1,?)",
        params![old, legacy],
    )?;
    conn.execute(
        "INSERT INTO claims VALUES ('/p','held','claude','',?,'2030-01-01 00:00:00',1,?)",
        params![old, legacy],
    )?;
    assert_eq!(
        touch(&mut conn, "current", "claude", &p, "2026-01-01 02:00:01")?,
        1
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM status", [], |r| r.get::<_, i64>(0))?,
        0
    );
    assert_eq!(
        conn.query_row("SELECT id FROM sessions", [], |r| r.get::<_, String>(0))?,
        "current"
    );
    assert_eq!(
        conn.query_row("SELECT session_id FROM claims", [], |r| r
            .get::<_, String>(0))?,
        legacy
    );
    assert_eq!(
        conn.query_row("SELECT COUNT(*) FROM events WHERE kind='status'", [], |r| r
            .get::<_, i64>(0))?,
        1
    );
    Ok(())
}
