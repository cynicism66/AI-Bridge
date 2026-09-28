//! 为未打包的 Windows 应用注册自己的通知身份和 COM 点击处理器。
//! 只写入 AI Bridge 自己的 HKCU 键；不需要管理员权限或开始菜单快捷方式。
use crate::{locale::text, state::Shared};
use std::ffi::c_void;
use tauri::Manager;
use windows::{
    core::*,
    Win32::{
        Foundation::{CLASS_E_NOAGGREGATION, E_POINTER},
        System::{Com::*, Registry::*},
        UI::{Notifications::*, Shell::SetCurrentProcessExplicitAppUserModelID},
    },
};

pub const APP_ID: &str = "io.github.cynicism66.ai-bridge";
const CLASS: GUID = GUID::from_u128(0x6e498951_8dc6_492c_8e1d_b32d6b40d87b);
const CLASS_TEXT: &str = "{6E498951-8DC6-492C-8E1D-B32D6B40D87B}";

pub fn navigate(app: &tauri::AppHandle, project: Option<String>) {
    if let Ok(mut s) = app.state::<Shared>().lock() {
        // 通知参数仅用于导航，必须已经存在于本地项目列表；从不执行它。
        let target = project.map(|p| match p.strip_prefix("attention:") {
            Some(path) => (path.to_owned(), true),
            None => (p, false),
        });
        if let Some((p, attention)) =
            target.filter(|(p, _)| s.projects.iter().any(|known| &known.project == p))
        {
            s.target = Some(p);
            s.target_action = attention;
        }
    }
    crate::show(app);
}

#[implement(INotificationActivationCallback)]
struct Activation(tauri::AppHandle);
impl INotificationActivationCallback_Impl for Activation_Impl {
    fn Activate(
        &self,
        _: &PCWSTR,
        args: &PCWSTR,
        _: *const NOTIFICATION_USER_INPUT_DATA,
        _: u32,
    ) -> Result<()> {
        let project = if args.is_null() {
            None
        } else {
            unsafe { args.to_string().ok() }.filter(|s| !s.is_empty())
        };
        navigate(&self.0, project);
        Ok(())
    }
}
#[implement(IClassFactory)]
struct Factory(tauri::AppHandle);
impl IClassFactory_Impl for Factory_Impl {
    fn CreateInstance(
        &self,
        outer: Ref<'_, IUnknown>,
        iid: *const GUID,
        out: *mut *mut c_void,
    ) -> Result<()> {
        if !outer.is_null() {
            return Err(CLASS_E_NOAGGREGATION.into());
        }
        if iid.is_null() || out.is_null() {
            return Err(E_POINTER.into());
        }
        let callback: INotificationActivationCallback = Activation(self.0.clone()).into();
        unsafe {
            *out = std::ptr::null_mut();
            callback.query(iid, out).ok()
        }
    }
    fn LockServer(&self, _: BOOL) -> Result<()> {
        Ok(())
    }
}
fn registry(path: &str, name: &str, value: &str) -> Result<()> {
    unsafe {
        let mut key = HKEY::default();
        RegCreateKeyExW(
            HKEY_CURRENT_USER,
            &HSTRING::from(path),
            Some(0),
            None,
            REG_OPTION_NON_VOLATILE,
            KEY_SET_VALUE,
            None,
            &mut key,
            None,
        )
        .ok()?;
        let bytes: Vec<u8> = value
            .encode_utf16()
            .chain(Some(0))
            .flat_map(u16::to_le_bytes)
            .collect();
        let result = RegSetValueExW(key, &HSTRING::from(name), Some(0), REG_SZ, Some(&bytes)).ok();
        let _ = RegCloseKey(key);
        result
    }
}
pub fn register(app: &tauri::AppHandle) -> anyhow::Result<()> {
    let path = format!("Software\\Classes\\AppUserModelId\\{APP_ID}");
    registry(&path, "DisplayName", text("appName"))?;
    registry(&path, "CustomActivator", CLASS_TEXT)?;
    let exe = std::env::current_exe()?;
    registry(
        &format!("Software\\Classes\\CLSID\\{CLASS_TEXT}\\LocalServer32"),
        "",
        &format!("\"{}\" --toast-activated", exe.display()),
    )?;
    unsafe {
        SetCurrentProcessExplicitAppUserModelID(&HSTRING::from(APP_ID))?;
        // Tauri 主线程已初始化为 STA；S_FALSE 仍为成功。
        CoInitializeEx(None, COINIT_APARTMENTTHREADED).ok()?;
        let factory: IClassFactory = Factory(app.clone()).into();
        let cookie =
            CoRegisterClassObject(&CLASS, &factory, CLSCTX_LOCAL_SERVER, REGCLS_MULTIPLEUSE)?;
        app.manage(Registration(cookie));
    }
    Ok(())
}
struct Registration(u32);
impl Drop for Registration {
    fn drop(&mut self) {
        unsafe {
            let _ = CoRevokeClassObject(self.0);
        }
    }
}
