//! WebView2 原生限制覆盖浏览器菜单和不经过 DOM 的加速键。
use tauri::Manager;

pub fn configure(app: &tauri::AppHandle, allow_debug: bool) -> tauri::Result<()> {
    let window = app.get_webview_window("main").expect("main window");
    let handle = app.clone();
    window.with_webview(move |view| {
        let result = (|| -> webview2_core::Result<()> {
            use webview2_com::Microsoft::Web::WebView2::Win32::ICoreWebView2Settings3;
            use webview2_core::Interface;
            unsafe {
                let settings = view.controller().CoreWebView2()?.Settings()?;
                settings.SetAreDefaultContextMenusEnabled(false)?;
                settings.SetAreDevToolsEnabled(allow_debug)?;
                settings
                    .cast::<ICoreWebView2Settings3>()?
                    .SetAreBrowserAcceleratorKeysEnabled(allow_debug)?;
            }
            Ok(())
        })();
        if result.is_err() {
            crate::report(&handle, crate::locale::text("browserSettingsFailed").into());
        }
    })
}
