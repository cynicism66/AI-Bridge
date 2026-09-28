use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::{Path, PathBuf},
};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub record_copies: BTreeMap<String, RecordCopy>,
    pub notifications_enabled: bool,
    pub hidden_projects: BTreeSet<String>,
    pub close_tip_shown: bool,
    pub last_notified_id: Option<i64>,
}
impl Default for AppSettings {
    fn default() -> Self {
        Self {
            notifications_enabled: true,
            record_copies: BTreeMap::new(),
            hidden_projects: BTreeSet::new(),
            close_tip_shown: false,
            last_notified_id: None,
        }
    }
}
impl AppSettings {
    pub fn from_env_path() -> Result<PathBuf> {
        let home = std::env::var_os("BRIDGE_APP_HOME")
            .or_else(|| std::env::var_os(if cfg!(windows) { "USERPROFILE" } else { "HOME" }))
            .context("无法确定软件设置目录")?;
        Ok(Self::path(Path::new(&home)))
    }
    pub fn path(home: &Path) -> PathBuf {
        home.join(".bridge/app.json")
    }
    pub fn load(path: &Path) -> Result<Self> {
        match std::fs::read(path) {
            Ok(bytes) => serde_json::from_slice(&bytes).context("软件设置文件格式错误"),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Self::default()),
            Err(e) => Err(e).context("无法读取软件设置"),
        }
    }
    pub fn save(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        crate::atomic_file::write(path, &serde_json::to_vec_pretty(self)?)
    }
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
pub struct RecordCopy {
    pub enabled: bool,
    pub last_event_id: i64,
}
