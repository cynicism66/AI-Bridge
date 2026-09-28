//! Bridge 的存储、工具和中文格式化；仅 Git 路径诊断写入 stderr，不使用 stdin/stdout。
mod agent_switch;
mod claims;
pub mod database;
mod format;
pub mod history;
pub mod human;
mod messages;
pub mod paths;
pub mod protocol;
pub mod repository;
mod sessions;
mod status;
mod tools;

use anyhow::Result;
use chrono::{Local, NaiveDateTime};
use database::Database;

pub const TIME_FMT: &str = "%Y-%m-%d %H:%M:%S";

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
