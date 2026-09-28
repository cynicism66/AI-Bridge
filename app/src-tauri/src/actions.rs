use crate::{
    commands::{error, key, lock, Reply},
    locale::text,
    state::Shared,
};
use tauri::{Manager, State};
#[tauri::command]
pub fn resolve_action(
    state: State<'_, Shared>,
    project: String,
    kind: String,
    id: i64,
) -> Reply<()> {
    let mut s = lock(&state)?;
    s.bridge
        .resolve_action(&key(&project)?, &kind, id)
        .map_err(error)?;
    s.refresh(true).map_err(error)?;
    Ok(())
}
pub fn notify(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let state = app.state::<Shared>();
    let mut s = state
        .lock()
        .map_err(|_| anyhow::anyhow!(text("stateUnavailable")))?;
    let keys = s.actions.iter().map(|a| a.key()).collect();
    let new: Vec<_> = s
        .actions
        .iter()
        .filter(|a| !s.settings.notified_actions.contains(&a.key()))
        .cloned()
        .collect();
    if s.settings.notified_actions != keys {
        let mut prefs = bridge_core::app_settings::AppSettings::load(&s.settings_path)?;
        prefs.notified_actions = keys;
        prefs.save(&s.settings_path)?;
        s.settings = prefs;
    }
    if s.global && s.settings.notifications_enabled {
        for item in new {
            if crate::notifications::show(
                app,
                &format!("{} · {}", text("actionTitle"), item.agent),
                &item.content.chars().take(160).collect::<String>(),
                Some(format!("attention:{}", item.project)),
            )
            .is_err()
            {
                s.error = Some(text("notificationFailed").into());
            }
        }
    }
    Ok(())
}
