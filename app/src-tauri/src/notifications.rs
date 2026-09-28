//! Tauri 的 Windows 通知插件不暴露 Activated；用 WinRT 保留桌面点击跳转。
use crate::{locale::text, state::Shared};
use tauri::Manager;

fn xml_text(s: &str) -> String {
    s.chars()
        .filter(|c| *c >= ' ' || matches!(c, '\n' | '\r' | '\t'))
        .collect::<String>()
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&apos;")
}
#[cfg(windows)]
pub fn show(
    app: &tauri::AppHandle,
    title: &str,
    body: &str,
    project: Option<String>,
) -> anyhow::Result<()> {
    use windows::{
        core::{IInspectable, HSTRING},
        Data::Xml::Dom::XmlDocument,
        Foundation::TypedEventHandler,
        UI::Notifications::{ToastNotification, ToastNotificationManager},
    };
    let xml = XmlDocument::new()?;
    xml.LoadXml(&HSTRING::from(format!("<toast launch=\"{}\"><visual><binding template=\"ToastGeneric\"><text>{}</text><text>{}</text></binding></visual></toast>", xml_text(project.as_deref().unwrap_or_default()),
        xml_text(title), xml_text(body))))?;
    let toast = ToastNotification::CreateToastNotification(&xml)?;
    let app = app.clone();
    toast.Activated(&TypedEventHandler::<ToastNotification, IInspectable>::new(
        move |_, _| {
            crate::notification_activation::navigate(&app, project.clone());
            Ok(())
        },
    ))?;
    ToastNotificationManager::CreateToastNotifierWithId(&HSTRING::from(
        "io.github.cynicism66.ai-bridge",
    ))?
    .Show(&toast)?;
    Ok(())
}
#[cfg(not(windows))]
pub fn register(_: &tauri::AppHandle) -> anyhow::Result<()> {
    Ok(())
}
#[cfg(not(windows))]
pub fn show(_: &tauri::AppHandle, _: &str, _: &str, _: Option<String>) -> anyhow::Result<()> {
    anyhow::bail!(text("notSupported"))
}

pub fn deliver(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let state = app.state::<Shared>();
    let mut s = state
        .lock()
        .map_err(|_| anyhow::anyhow!(text("stateUnavailable")))?;
    loop {
        let batch = s
            .bridge
            .notification_messages(s.settings.last_notified_id.unwrap_or(0))?;
        if batch.is_empty() {
            break;
        }
        for m in batch {
            // 在发送前持久化高水位；系统通知失败也不在每轮重复骚扰。
            // 异常退出可能跳过一条提示，消息和未读角标始终保留。
            let mut settings = s.settings.clone();
            settings.last_notified_id = Some(m.id);
            settings.save(&s.settings_path)?;
            s.settings = settings;
            if show(
                app,
                &format!("{} · {}", text("appName"), m.sender),
                &format!(
                    "{}\n{}",
                    m.project,
                    m.content.chars().take(160).collect::<String>()
                ),
                Some(m.project),
            )
            .is_err()
            {
                s.error = Some(text("notificationFailed").into());
            }
        }
    }
    Ok(())
}
#[cfg(test)]
mod tests {
    #[test]
    fn notification_content_cannot_inject_xml() {
        assert_eq!(super::xml_text("<&\"'>\0"), "&lt;&amp;&quot;&apos;&gt;");
    }
}
