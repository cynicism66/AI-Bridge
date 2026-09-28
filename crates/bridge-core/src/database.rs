use anyhow::{bail, Result};
use rusqlite::{types::ValueRef, Connection, ErrorCode, OpenFlags, Params};
use serde_json::{Map, Value};
use std::{
    collections::HashSet,
    path::PathBuf,
    sync::{Mutex, OnceLock},
    time::Duration,
};

pub const VERSION: i64 = 6;
pub const MIGRATIONS: [&str; 6] = [
    include_str!("../../../migrations/001.sql"),
    include_str!("../../../migrations/002.sql"),
    include_str!("../../../migrations/003.sql"),
    include_str!("../../../migrations/004.sql"),
    include_str!("../../../migrations/005.sql"),
    include_str!("../../../migrations/006.sql"),
];
static INITIALIZED: OnceLock<Mutex<HashSet<PathBuf>>> = OnceLock::new();

pub struct Database {
    pub path: PathBuf,
    create_parent: bool,
}

pub fn check_version(conn: &Connection) -> Result<i64> {
    let version = conn.query_row("PRAGMA user_version", [], |row| row.get::<_, i64>(0))?;
    if version > VERSION {
        bail!("数据库版本 {version} 高于当前支持的版本 {VERSION}，请升级 Bridge 后再写入。");
    }
    Ok(version)
}

pub fn migrate(conn: &mut Connection) -> Result<()> {
    let transaction = conn.transaction_with_behavior(rusqlite::TransactionBehavior::Immediate)?;
    let version = check_version(&transaction)?;
    for number in version + 1..=VERSION {
        transaction.execute_batch(MIGRATIONS[number as usize - 1])?;
        transaction.pragma_update(None, "user_version", number)?;
    }
    transaction.commit()?;
    Ok(())
}

impl Database {
    #[cfg(test)]
    pub(crate) fn for_test(path: PathBuf) -> Self {
        Self {
            path,
            create_parent: false,
        }
    }
    pub fn from_env() -> Result<Self> {
        let custom = std::env::var_os("BRIDGE_DB").filter(|v| !v.is_empty());
        let create_parent = custom.is_none();
        let path = if let Some(custom) = custom {
            PathBuf::from(custom)
        } else {
            let home = std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" })
                .ok_or_else(|| anyhow::anyhow!("无法确定用户主目录"))?;
            PathBuf::from(home).join(".bridge").join("bridge.db")
        };
        Ok(Self {
            path,
            create_parent,
        })
    }

    pub fn open(&self) -> Result<Connection> {
        if self.create_parent {
            if let Some(parent) = self.path.parent().filter(|p| !p.as_os_str().is_empty()) {
                std::fs::create_dir_all(parent)?;
            }
        }
        let mut conn = Connection::open(&self.path)?;
        conn.busy_timeout(Duration::from_secs(15))?;
        check_version(&conn)?;
        let key = std::fs::canonicalize(&self.path)?;
        let mut initialized = INITIALIZED
            .get_or_init(Mutex::default)
            .lock()
            .map_err(|_| anyhow::anyhow!("数据库初始化锁不可用"))?;
        if !initialized.contains(&key) {
            for attempt in 0..10 {
                match conn.query_row("PRAGMA journal_mode=WAL", [], |row| row.get::<_, String>(0)) {
                    Ok(_) => break,
                    Err(rusqlite::Error::SqliteFailure(code, _))
                        if matches!(
                            code.code,
                            ErrorCode::DatabaseBusy | ErrorCode::DatabaseLocked
                        ) && attempt < 9 =>
                    {
                        std::thread::sleep(Duration::from_millis(20 * (attempt + 1)));
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            migrate(&mut conn)?;
            initialized.insert(key);
        }
        Ok(conn)
    }

    pub fn read_only(&self) -> Result<Connection> {
        let conn = Connection::open_with_flags(&self.path, OpenFlags::SQLITE_OPEN_READ_ONLY)?;
        conn.busy_timeout(Duration::from_secs(15))?;
        Ok(conn)
    }
}

pub fn query(conn: &Connection, sql: &str, parameters: impl Params) -> Result<Vec<Value>> {
    let mut statement = conn.prepare(sql)?;
    let names: Vec<String> = statement
        .column_names()
        .iter()
        .map(|s| s.to_string())
        .collect();
    let rows = statement.query_map(parameters, |row| {
        let mut result = Map::new();
        for (index, name) in names.iter().enumerate() {
            let value = match row.get_ref(index)? {
                ValueRef::Null => Value::Null,
                ValueRef::Integer(n) => Value::from(n),
                ValueRef::Real(n) => Value::from(n),
                ValueRef::Text(s) | ValueRef::Blob(s) => {
                    Value::String(String::from_utf8_lossy(s).into_owned())
                }
            };
            result.insert(name.clone(), value);
        }
        Ok(Value::Object(result))
    })?;
    Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn legacy_backfill_and_idempotent_migrations() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch(MIGRATIONS[0])?;
        conn.execute("INSERT INTO messages VALUES (1, '/p', 'claude', 'all', '旧消息', '2020-01-01 00:00:00')", [])?;
        migrate(&mut conn)?;
        migrate(&mut conn)?;
        assert_eq!(check_version(&conn)?, VERSION);
        assert_eq!(query(&conn, "SELECT * FROM events", [])?.len(), 1);
        assert_eq!(query(&conn, "SELECT * FROM messages", [])?.len(), 1);
        conn.pragma_update(None, "user_version", VERSION + 1)?;
        assert!(migrate(&mut conn)
            .unwrap_err()
            .to_string()
            .contains("请升级 Bridge"));
        Ok(())
    }

    #[test]
    fn failed_migration_rolls_back_version_and_tables() -> Result<()> {
        let mut conn = Connection::open_in_memory()?;
        conn.execute_batch("CREATE TABLE status (wrong TEXT)")?;
        assert!(migrate(&mut conn).is_err());
        assert_eq!(check_version(&conn)?, 0);
        assert!(query(&conn, "SELECT * FROM messages", []).is_err());
        Ok(())
    }
}
