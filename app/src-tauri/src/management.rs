use crate::{
    commands::{error, key, lock, Reply},
    locale::text,
    state::Shared,
};
use bridge_core::{
    history_page::{HistoryFilter, HistoryPage},
    init_draft::PreparedInit,
    initialization::InitOptions,
    permission_edit::PermissionPatch,
    transfer::PreparedTransfer,
};
use serde::Deserialize;
use serde_json::{json, Value};
use tauri::State;

pub enum Pending {
    Init(Box<PreparedInit>),
    Transfer(Box<PreparedTransfer>),
}
#[derive(Deserialize)]
pub struct InitRequest {
    template: String,
    template_file: Option<String>,
    roles: Vec<String>,
    goal: String,
    write_rules: bool,
    no_kickoff: bool,
}
#[tauri::command]
pub fn management(state: State<'_, Shared>, project: String) -> Reply<Value> {
    lock(&state)?
        .bridge
        .management(&key(&project)?)
        .map_err(error)
}
#[tauri::command]
pub fn template_info(name: String, file: Option<String>) -> Reply<Value> {
    bridge_core::templates::load(&name, file.as_deref().map(std::path::Path::new))
        .map(|t| json!(t))
        .map_err(error)
}
#[tauri::command]
pub fn preview_init(
    state: State<'_, Shared>,
    project: String,
    request: InitRequest,
) -> Reply<Value> {
    let mut s = lock(&state)?;
    s.pending = None;
    let draft = s
        .bridge
        .prepare_init(
            &key(&project)?,
            InitOptions {
                template: &request.template,
                template_file: request.template_file.as_deref().map(std::path::Path::new),
                roles: &request.roles,
                goal: &request.goal,
                write_rules: request.write_rules,
                no_kickoff: request.no_kickoff,
            },
        )
        .map_err(error)?;
    s.preview_serial += 1;
    let id = s.preview_serial;
    let result = json!({"id":id,"charter":draft.charter});
    s.pending = Some((id, Pending::Init(Box::new(draft))));
    Ok(result)
}
#[tauri::command]
pub fn preview_transfer(
    state: State<'_, Shared>,
    project: String,
    out: Option<String>,
    to: Option<String>,
) -> Reply<Value> {
    let mut s = lock(&state)?;
    s.pending = None;
    let draft = s
        .bridge
        .prepare_transfer(&key(&project)?, out.as_deref(), to.as_deref())
        .map_err(error)?;
    s.preview_serial += 1;
    let id = s.preview_serial;
    let result = json!({"id":id,"preview":draft.view});
    s.pending = Some((id, Pending::Transfer(Box::new(draft))));
    Ok(result)
}
#[tauri::command]
pub fn confirm_preview(state: State<'_, Shared>, id: u64) -> Reply<String> {
    let mut s = lock(&state)?;
    if !s.pending.as_ref().is_some_and(|(n, _)| *n == id) {
        return Err(text("previewExpired").into());
    }
    let (_, draft) = s.pending.take().unwrap();
    let result = match draft {
        Pending::Init(d) => d.commit(),
        Pending::Transfer(d) => d.commit(),
    }
    .map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(result)
}
#[tauri::command]
pub fn cancel_preview(state: State<'_, Shared>, id: u64) -> Reply<()> {
    let mut s = lock(&state)?;
    if s.pending.as_ref().is_some_and(|(n, _)| *n == id) {
        s.pending = None;
    }
    Ok(())
}
#[tauri::command]
pub fn set_role(
    state: State<'_, Shared>,
    project: String,
    agent: String,
    slot: String,
) -> Reply<String> {
    let mut s = lock(&state)?;
    let result = s
        .bridge
        .role_text(&key(&project)?, &agent, &slot)
        .map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(result)
}
#[tauri::command]
pub fn set_permission(
    state: State<'_, Shared>,
    project: String,
    slot: String,
    patch: PermissionPatch,
) -> Reply<String> {
    let mut s = lock(&state)?;
    let result = s
        .bridge
        .permission_text(&key(&project)?, &slot, patch)
        .map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(result)
}
#[tauri::command]
pub fn validate_write(rules: Vec<String>) -> Reply<()> {
    bridge_core::permission_edit::validate_write(&rules).map_err(error)
}
#[tauri::command]
pub fn history_page(
    state: State<'_, Shared>,
    project: String,
    filter: HistoryFilter,
) -> Reply<HistoryPage> {
    lock(&state)?
        .bridge
        .history_page(&key(&project)?, filter)
        .map_err(error)
}
#[tauri::command]
pub fn app_settings(state: State<'_, Shared>) -> Reply<Value> {
    let s = lock(&state)?;
    Ok(
        json!({"preferences":s.settings,"database":s.bridge.database.path,"version":env!("CARGO_PKG_VERSION")}),
    )
}
#[derive(Deserialize)]
pub struct SettingsPatch {
    notifications_enabled: Option<bool>,
    unhide: Option<String>,
    #[serde(default)]
    reset_close_tip: bool,
}
#[tauri::command]
pub fn save_settings(state: State<'_, Shared>, patch: SettingsPatch) -> Reply<()> {
    let mut s = lock(&state)?;
    let mut prefs = crate::project_commands::fresh_settings(&s.settings_path)?;
    if let Some(enabled) = patch.notifications_enabled {
        prefs.notifications_enabled = enabled;
        prefs.last_notified_id = Some(s.bridge.latest_message_id().map_err(error)?);
    }
    if let Some(project) = patch.unhide {
        prefs.hidden_projects.remove(&key(&project)?);
    }
    if patch.reset_close_tip {
        prefs.close_tip_shown = false;
    }
    prefs.save(&s.settings_path).map_err(error)?;
    s.settings = prefs;
    s.refresh(true).map_err(error)?;
    Ok(())
}
