use crate::{
    commands::{error, key, lock, Reply},
    state::{AppState, Shared},
};
use bridge_core::{app_settings::AppSettings, copy_options};
use serde_json::{json, Value};
use tauri::State;

impl AppState {
    pub fn sync_copies(&mut self) {
        if self.copy_revision == self.revision && self.copy_errors.is_empty() {
            return;
        }
        let old_errors = self.copy_errors.clone();
        self.copy_errors.clear();
        if self.global {
            for project in self.projects.iter().filter(|p| p.enabled) {
                if !self
                    .settings
                    .record_copies
                    .get(&project.project)
                    .is_some_and(|p| p.enabled)
                {
                    continue;
                }
                let result = (|| -> anyhow::Result<()> {
                    if let Some(id) = self.bridge.sync_record_copy(&project.project)? {
                        let mut prefs = AppSettings::load(&self.settings_path)?;
                        if let Some(copy) = prefs.record_copies.get_mut(&project.project) {
                            if copy.last_event_id != id {
                                copy.last_event_id = id;
                                prefs.save(&self.settings_path)?;
                                self.settings = prefs;
                            }
                        }
                    }
                    Ok(())
                })();
                if let Err(e) = result {
                    self.copy_errors
                        .insert(project.project.clone(), e.to_string());
                }
            }
        }
        if self.copy_errors != old_errors {
            self.revision += 1;
        }
        self.copy_revision = self.revision;
    }
}
#[tauri::command]
pub fn copy_settings(state: State<'_, Shared>, project: String) -> Reply<Value> {
    let s = lock(&state)?;
    let project = key(&project)?;
    let copy = s
        .settings
        .record_copies
        .get(&project)
        .cloned()
        .unwrap_or_default();
    let protection = copy_options::protection(std::path::Path::new(&project));
    let (git, ignored, warning) = match protection {
        Ok(p) => (p.git, p.ignored, None),
        Err(e) => (
            std::path::Path::new(&project).join(".git").exists(),
            false,
            Some(e.to_string()),
        ),
    };
    Ok(
        json!({"enabled":copy.enabled,"last_event_id":copy.last_event_id,"git":git,"ignored":ignored,"warning":warning,"error":s.copy_errors.get(&project)}),
    )
}
#[tauri::command]
pub fn save_copy_settings(
    state: State<'_, Shared>,
    project: String,
    enabled: bool,
    add_ignore: bool,
) -> Reply<()> {
    let mut s = lock(&state)?;
    copy_options::set_copy_options(&key(&project)?, &s.settings_path, enabled, add_ignore)
        .map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(())
}
#[tauri::command]
pub fn ignore_record_copy(state: State<'_, Shared>, project: String) -> Reply<()> {
    let mut s = lock(&state)?;
    copy_options::add_ignore(&key(&project)?).map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(())
}
