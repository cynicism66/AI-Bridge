use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use std::{
    collections::BTreeSet,
    path::{Path, PathBuf},
};

#[derive(Default, Clone, Debug, Serialize, Deserialize)]
#[serde(default)]
pub struct AppSettings {
    pub hidden_projects: BTreeSet<String>,
    pub close_tip_shown: bool,
    pub last_notified_id: Option<i64>,
}
impl AppSettings {
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
