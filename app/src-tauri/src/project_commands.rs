use crate::{
    commands::{error, key, lock, Reply},
    state::Shared,
};
use bridge_core::{app_settings::AppSettings, project_lifecycle::PurgeInfo};
use tauri::State;

#[tauri::command]
pub fn add_project(state: State<'_, Shared>, project: String) -> Reply<String> {
    let mut s = lock(&state)?;
    let project = key(&project)?;
    s.bridge
        .add_project(&project, &s.settings_path)
        .map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(project)
}
#[tauri::command]
pub fn forget_project(state: State<'_, Shared>, project: String) -> Reply<String> {
    let mut s = lock(&state)?;
    let project = key(&project)?;
    let result = s
        .bridge
        .forget_project(&project, &s.settings_path)
        .map_err(error)?;
    s.pending = None;
    s.refresh(true).map_err(error)?;
    Ok(result)
}
#[tauri::command]
pub fn purge_project(state: State<'_, Shared>, project: String, force: bool) -> Reply<String> {
    let mut s = lock(&state)?;
    let project = key(&project)?;
    let result = s
        .bridge
        .purge_project(&project, &s.settings_path, true, force)
        .map_err(error)?;
    s.pending = None;
    if s.target.as_deref() == Some(&project) {
        s.target = None;
    }
    s.refresh(true).map_err(error)?;
    Ok(result)
}
#[tauri::command]
pub fn purge_info(state: State<'_, Shared>, project: String) -> Reply<PurgeInfo> {
    lock(&state)?
        .bridge
        .purge_info(&key(&project)?)
        .map_err(error)
}
#[tauri::command]
pub fn open_project_folder(project: String) -> Reply<()> {
    // 仅接受存在的目录；独立参数交给资源管理器，绝不经过 shell 解析。
    let path = std::fs::canonicalize(key(&project)?).map_err(error)?;
    if !path.is_dir() {
        return Err(crate::locale::text("folderMissing").into());
    }
    let system = std::env::var_os("SystemRoot").ok_or(crate::locale::text("folderMissing"))?;
    std::process::Command::new(std::path::Path::new(&system).join("explorer.exe"))
        .arg(bridge_core::display_path(&path.to_string_lossy()))
        .spawn()
        .map_err(error)?;
    Ok(())
}
pub fn fresh_settings(path: &std::path::Path) -> Reply<AppSettings> {
    AppSettings::load(path).map_err(error)
}
