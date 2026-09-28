use anyhow::Result;
use bridge_core::{
    app_settings::AppSettings,
    desktop::{ChangeDetector, ProjectSummary},
    discovery::{discover, DiscoveredProject},
    Bridge,
};
use std::{collections::BTreeSet, path::PathBuf, sync::Mutex, time::SystemTime};

pub type Shared = Mutex<AppState>;
pub struct AppState {
    pub pending: Option<(u64, crate::management::Pending)>,
    pub preview_serial: u64,
    pub bridge: Bridge,
    pub settings: AppSettings,
    pub settings_path: PathBuf,
    pub home: PathBuf,
    pub projects: Vec<ProjectSummary>,
    pub discovered: Vec<DiscoveredProject>,
    pub global: bool,
    pub revision: u64,
    pub target: Option<String>,
    pub error: Option<String>,
    detector: ChangeDetector,
    version: i64,
    config_stamp: Vec<Option<SystemTime>>,
    next_expiry: Option<String>,
}
impl AppState {
    pub fn new() -> Result<Self> {
        let home = PathBuf::from(
            std::env::var_os("BRIDGE_APP_HOME")
                .or_else(|| std::env::var_os("USERPROFILE"))
                .ok_or_else(|| anyhow::anyhow!(crate::locale::text("stateUnavailable")))?,
        );
        let bridge = Bridge::from_env()?;
        let detector = ChangeDetector::new(&bridge)?;
        let settings_path = AppSettings::path(&home);
        let mut settings = AppSettings::load(&settings_path)?;
        // 首次运行以当前消息为基线，历史消息只显示未读角标，不弹成批旧通知。
        if settings.last_notified_id.is_none() {
            settings.last_notified_id = Some(bridge.latest_message_id()?);
            settings.save(&settings_path)?;
        }
        let mut state = Self {
            pending: None,
            preview_serial: 0,
            bridge,
            detector,
            home,
            settings,
            settings_path,
            projects: vec![],
            discovered: vec![],
            global: true,
            revision: 0,
            target: None,
            error: None,
            version: -1,
            config_stamp: vec![],
            next_expiry: None,
        };
        state.refresh(true)?;
        Ok(state)
    }
    pub fn refresh(&mut self, force: bool) -> Result<bool> {
        let version = self.detector.version()?;
        let stamp = [".claude.json", ".codex/config.toml"]
            .iter()
            .map(|p| {
                std::fs::metadata(self.home.join(p))
                    .ok()
                    .and_then(|m| m.modified().ok())
            })
            .collect::<Vec<_>>();
        let now = bridge_core::now()?;
        let expired = self.next_expiry.as_ref().is_some_and(|e| e < &now);
        if !force && version == self.version && stamp == self.config_stamp && !expired {
            return Ok(false);
        }
        self.projects = self.bridge.project_list()?;
        self.global = self.bridge.switch_state(None)?.global_enabled;
        let known: BTreeSet<_> = self.projects.iter().map(|p| p.project.clone()).collect();
        self.discovered = discover(&self.home, &known, &self.settings.hidden_projects);
        self.next_expiry = self.bridge.database.open()?.query_row(
            "SELECT MIN(t) FROM (SELECT expires_at AS t FROM claims UNION ALL SELECT datetime(last_active,'+2 hours') FROM sessions) WHERE t>=?",
            [now],
            |r| r.get(0),
        )?;
        self.version = version;
        self.config_stamp = stamp;
        self.revision += 1;
        Ok(true)
    }
}
