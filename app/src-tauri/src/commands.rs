use crate::{locale::text, state::Shared};
use bridge_core::{
    desktop::{HumanVia, MessagePage, ProjectDetail},
    repository,
};
use serde_json::{json, Value};
use tauri::{Manager, State};

pub(crate) type Reply<T> = Result<T, String>;
pub(crate) fn error(e: impl std::fmt::Display) -> String {
    e.to_string()
}
pub(crate) fn lock(s: &Shared) -> Reply<std::sync::MutexGuard<'_, crate::state::AppState>> {
    s.lock().map_err(|_| text("stateUnavailable").into())
}
pub(crate) fn key(project: &str) -> Reply<String> {
    repository::resolve(project).map(|p| p.key).map_err(error)
}

#[tauri::command]
pub fn snapshot(state: State<'_, Shared>) -> Reply<Value> {
    let mut s = lock(&state)?;
    s.refresh(false).map_err(error)?;
    Ok(
        json!({"actions":s.actions,"projects":s.projects,"discovered":s.discovered,"global":s.global,"revision":s.revision}),
    )
}
#[tauri::command]
pub fn check_changes(app: tauri::AppHandle, state: State<'_, Shared>) -> Reply<Value> {
    let mut s = lock(&state)?;
    s.refresh(false).map_err(error)?;
    let visible = app
        .get_webview_window("main")
        .is_some_and(|w| w.is_visible().unwrap_or(false));
    Ok(
        json!({"revision":s.revision,"visible":visible,"target":s.target.take(),"targetAction":std::mem::take(&mut s.target_action),"error":s.error.take()}),
    )
}
#[tauri::command]
pub fn project_detail(state: State<'_, Shared>, project: String) -> Reply<ProjectDetail> {
    lock(&state)?
        .bridge
        .project_detail(&key(&project)?)
        .map_err(error)
}
#[tauri::command]
pub fn message_page(
    state: State<'_, Shared>,
    project: String,
    before: Option<i64>,
) -> Reply<MessagePage> {
    lock(&state)?
        .bridge
        .message_page(&key(&project)?, before, 50)
        .map_err(error)
}
#[tauri::command]
pub fn send_message(
    state: State<'_, Shared>,
    project: String,
    content: String,
    to: String,
) -> Reply<i64> {
    if !["all", "claude", "codex"].contains(&to.as_str()) {
        return Err(text("invalidRecipient").into());
    }
    lock(&state)?
        .bridge
        .post_human(&key(&project)?, &content, &to, HumanVia::Gui)
        .map_err(error)
}
#[tauri::command]
pub fn mark_read(state: State<'_, Shared>, project: String, through: i64) -> Reply<()> {
    lock(&state)?
        .bridge
        .mark_human_read(&key(&project)?, through)
        .map_err(error)
}
#[tauri::command]
pub fn set_switch(
    state: State<'_, Shared>,
    project: Option<String>,
    agent: Option<String>,
    enabled: bool,
) -> Reply<Option<String>> {
    let mut s = lock(&state)?;
    let project = project.as_deref().map(key).transpose()?;
    if let (Some(project), Some(agent)) = (&project, agent) {
        s.bridge
            .set_agent_enabled(project, &agent, enabled)
            .map_err(error)?;
    } else {
        s.bridge
            .set_enabled(enabled, project.as_deref())
            .map_err(error)?;
    }
    s.refresh(true).map_err(error)?;
    Ok(project)
}
#[tauri::command]
pub fn hide_project(state: State<'_, Shared>, project: String) -> Reply<()> {
    let mut s = lock(&state)?;
    let mut settings = crate::project_commands::fresh_settings(&s.settings_path)?;
    settings.hidden_projects.insert(key(&project)?);
    settings.save(&s.settings_path).map_err(error)?;
    s.settings = settings;
    s.refresh(true).map_err(error)?;
    Ok(())
}
#[tauri::command]
pub fn window_ready(app: tauri::AppHandle) {
    crate::show(&app);
}
