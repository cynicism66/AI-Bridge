use crate::{locale::text, state::Shared};
use tauri::{
    image::Image,
    menu::{CheckMenuItem, Menu, MenuItem, PredefinedMenuItem},
    tray::{MouseButton, MouseButtonState, TrayIconBuilder, TrayIconEvent},
    Manager,
};

pub fn icon(global: bool, unread: bool) -> Image<'static> {
    let mut rgba = vec![0_u8; 32 * 32 * 4];
    for y in 0..32 {
        for x in 0..32 {
            let bridge = ((6..10).contains(&x) || (22..26).contains(&x)) && (8..27).contains(&y)
                || (6..26).contains(&x) && (16..20).contains(&y)
                || ((x as i32 - 16).pow(2) + (y as i32 - 18).pow(2) < 95 && y < 13);
            let dot = global && unread && (x as i32 - 26).pow(2) + (y as i32 - 6).pow(2) <= 25;
            if bridge || dot {
                let color = if dot {
                    [220, 45, 66, 255]
                } else if global {
                    [24, 126, 209, 255]
                } else {
                    [135, 139, 145, 255]
                };
                rgba[(y * 32 + x) * 4..(y * 32 + x) * 4 + 4].copy_from_slice(&color);
            }
        }
    }
    Image::new_owned(rgba, 32, 32)
}
pub fn setup(app: &tauri::AppHandle) -> tauri::Result<()> {
    TrayIconBuilder::with_id("bridge")
        .tooltip(text("appName"))
        .icon(icon(true, false))
        .show_menu_on_left_click(false)
        .on_tray_icon_event(|tray, event| {
            if matches!(
                event,
                TrayIconEvent::Click {
                    button: MouseButton::Left,
                    button_state: MouseButtonState::Up,
                    ..
                }
            ) {
                if let Some(w) = tray.app_handle().get_webview_window("main") {
                    if w.is_visible().unwrap_or(false) {
                        let _ = w.hide();
                    } else {
                        crate::show(tray.app_handle());
                    }
                }
            }
        })
        .on_menu_event(|app, event| {
            let id = event.id.as_ref();
            match id {
                "open" => crate::show(app),
                "exit" => app.exit(0),
                _ => {
                    let result = (|| -> anyhow::Result<()> {
                        let state = app.state::<Shared>();
                        let mut s = state
                            .lock()
                            .map_err(|_| anyhow::anyhow!(text("stateUnavailable")))?;
                        if id == "global" {
                            s.bridge.set_enabled(!s.global, None)?;
                        } else if let Some(project) = id.strip_prefix("project:") {
                            let enabled = s.bridge.switch_state(Some(project))?.project_enabled;
                            s.bridge.set_enabled(!enabled, Some(project))?;
                        }
                        s.refresh(true)?;
                        Ok(())
                    })();
                    if let Err(e) = result {
                        crate::report(app, e.to_string());
                    }
                }
            }
        })
        .build(app)?;
    refresh(app)
}
pub fn refresh(app: &tauri::AppHandle) -> tauri::Result<()> {
    let (global, unread, projects) = {
        let state = app.state::<Shared>();
        let s = state.lock().expect("state lock");
        (
            s.global,
            s.projects.iter().any(|p| p.unread > 0),
            s.projects
                .iter()
                .map(|p| (p.project.clone(), p.enabled))
                .chain(s.discovered.iter().map(|p| (p.project.clone(), false)))
                .collect::<Vec<_>>(),
        )
    };
    let menu = Menu::new(app)?;
    for (project, enabled) in projects {
        // Windows 菜单把 & 当快捷键标记；显示路径必须转义。
        menu.append(&CheckMenuItem::with_id(
            app,
            format!("project:{project}"),
            project.replace('&', "&&"),
            true,
            enabled,
            None::<&str>,
        )?)?;
    }
    menu.append(&PredefinedMenuItem::separator(app)?)?;
    menu.append(&CheckMenuItem::with_id(
        app,
        "global",
        text("globalSwitch"),
        true,
        global,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "open",
        text("openWindow"),
        true,
        None::<&str>,
    )?)?;
    menu.append(&MenuItem::with_id(
        app,
        "exit",
        text("exit"),
        true,
        None::<&str>,
    )?)?;
    if let Some(tray) = app.tray_by_id("bridge") {
        tray.set_icon(Some(icon(global, unread)))?;
        tray.set_menu(Some(menu))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    #[test]
    fn three_tray_states_have_distinct_pixels() {
        let off = super::icon(false, false);
        let normal = super::icon(true, false);
        let unread = super::icon(true, true);
        assert_ne!(off.rgba(), normal.rgba());
        assert_ne!(normal.rgba(), unread.rgba());
        assert_eq!(super::icon(false, true).rgba(), off.rgba());
    }
}
