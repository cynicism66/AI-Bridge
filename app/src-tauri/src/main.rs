#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]
mod browser;
mod commands;
mod locale;
mod management;
#[cfg(windows)]
mod notification_activation;
mod notifications;
mod project_commands;
mod state;
mod tray;
use state::Shared;
use tauri::Manager;
use tauri_plugin_dialog::DialogExt;

fn backdrop(app: &tauri::AppHandle) {
    if let Some(w) = app.get_webview_window("main") {
        let mica = w.set_effects(
            tauri::window::EffectsBuilder::new()
                .effect(tauri::window::Effect::Mica)
                .build(),
        );
        if mica.is_err() {
            let color = if w.theme().ok() == Some(tauri::Theme::Dark) {
                tauri::window::Color(31, 34, 41, 255)
            } else {
                tauri::window::Color(249, 250, 252, 255)
            };
            let _ = w.set_background_color(Some(color));
        }
    }
}

pub fn show(app: &tauri::AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.unminimize();
        let _ = window.show();
        let _ = window.set_focus();
    }
}
pub fn report(app: &tauri::AppHandle, error: String) {
    if let Ok(mut s) = app.state::<Shared>().lock() {
        s.error = Some(error);
    }
}
fn main() {
    tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| show(app)))
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_notification::init())
        .invoke_handler(tauri::generate_handler![
            commands::snapshot,
            commands::check_changes,
            commands::project_detail,
            commands::message_page,
            commands::send_message,
            commands::mark_read,
            commands::set_switch,
            commands::hide_project,
            commands::window_ready,
            project_commands::add_project,
            project_commands::forget_project,
            project_commands::purge_info,
            project_commands::purge_project,
            project_commands::open_project_folder,
            management::management,
            management::template_info,
            management::preview_init,
            management::preview_transfer,
            management::confirm_preview,
            management::cancel_preview,
            management::set_role,
            management::set_permission,
            management::validate_write,
            management::history_page,
            management::app_settings,
            management::save_settings
        ])
        .setup(|app| {
            app.manage(Shared::new(state::AppState::new()?));
            // 调试隔离实例不注册系统通知；release 永远执行正常注册。
            let isolated = cfg!(debug_assertions)
                && std::env::var_os("BRIDGE_UI_TEST").is_some()
                && std::env::var_os("BRIDGE_DB").is_some()
                && std::env::var_os("BRIDGE_APP_HOME").is_some();
            // 隔离调试实例可验证与 release 相同的原生限制，且不注册系统通知。
            let allow_debug = cfg!(debug_assertions)
                && !(isolated && std::env::var_os("BRIDGE_UI_TEST_RELEASE").is_some());
            browser::configure(app.handle(), allow_debug)?;
            if !isolated && notification_activation::register(app.handle()).is_err() {
                report(app.handle(), locale::text("notificationFailed").into());
            }
            tray::setup(app.handle())?;
            backdrop(app.handle());
            let app = app.handle().clone();
            std::thread::spawn(move || {
                let mut last_revision = 0;
                loop {
                    let result = (|| -> anyhow::Result<()> {
                        let revision = {
                            let state = app.state::<Shared>();
                            let mut s = state
                                .lock()
                                .map_err(|_| anyhow::anyhow!(locale::text("stateUnavailable")))?;
                            s.refresh(false)?;
                            s.revision
                        };
                        if revision != last_revision {
                            tray::refresh(&app)?;
                            notifications::deliver(&app)?;
                            last_revision = revision;
                        }
                        Ok(())
                    })();
                    if let Err(e) = result {
                        report(&app, e.to_string());
                    }
                    let visible = app
                        .get_webview_window("main")
                        .is_some_and(|w| w.is_visible().unwrap_or(false));
                    std::thread::sleep(std::time::Duration::from_millis(if visible {
                        1500
                    } else {
                        5000
                    }));
                }
            });
            Ok(())
        })
        .on_window_event(|window, event| {
            if matches!(event, tauri::WindowEvent::ThemeChanged(_)) {
                backdrop(window.app_handle());
            }
            if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                api.prevent_close();
                let _ = window.hide();
                let result = (|| -> anyhow::Result<()> {
                    let state = window.state::<Shared>();
                    let mut s = state
                        .lock()
                        .map_err(|_| anyhow::anyhow!(locale::text("stateUnavailable")))?;
                    if !s.settings.close_tip_shown {
                        let mut settings =
                            bridge_core::app_settings::AppSettings::load(&s.settings_path)?;
                        settings.close_tip_shown = true;
                        settings.save(&s.settings_path)?;
                        s.settings = settings;
                        drop(s);
                        window
                            .app_handle()
                            .dialog()
                            .message(locale::text("trayHintBody"))
                            .title(locale::text("trayHint"))
                            .show(|_| {});
                    }
                    Ok(())
                })();
                if let Err(e) = result {
                    report(window.app_handle(), e.to_string());
                }
            }
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|e| panic!("{}: {e}", locale::text("startupFailed")));
}
