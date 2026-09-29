// 最小公告板核心：消息、状态、认领、开关、会话和 Stop 钩子。
mod claims;
pub mod database;
mod format;
pub mod hook;
pub mod human;
mod messages;
pub mod paths;
pub mod protocol;
pub mod repository;
mod sessions;
mod status;
mod tools;
pub mod wait;

use anyhow::Result;
use chrono::{Local, NaiveDateTime};
use database::Database;

pub const TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";

pub fn display_path(path: &str) -> String {
    if path.as_bytes().get(1) == Some(&b':') {
        format!(
            "{}{}",
            path[..1].to_uppercase(),
            path[1..].replace('/', "\\")
        )
    } else if path.starts_with("//") {
        path.replace('/', "\\")
    } else {
        path.into()
    }
}

pub fn now() -> Result<String> {
    match std::env::var("BRIDGE_FAKE_NOW") {
        Ok(value) if !value.is_empty() => Ok(NaiveDateTime::parse_from_str(&value, TIME_FMT)?
            .format(TIME_FMT)
            .to_string()),
        _ => Ok(Local::now().format(TIME_FMT).to_string()),
    }
}

pub struct Bridge {
    pub database: Database,
    pub agent: String,
    pub session_id: String,
}
impl Bridge {
    pub fn from_env() -> Result<Self> {
        let agent = std::env::var("BRIDGE_AGENT")
            .unwrap_or_default()
            .trim()
            .to_lowercase();
        Ok(Self {
            database: Database::from_env()?,
            session_id: sessions::random_id()?,
            agent: if agent.is_empty() {
                "unknown".into()
            } else {
                agent
            },
        })
    }
}
#[cfg(test)]
mod switch_tests;
#[cfg(test)]
mod test_support;
