use crate::{app_settings::AppSettings, file_batch::Batch};
use anyhow::{bail, Context, Result};
use serde::Serialize;
use std::{path::Path, process::Command};

#[derive(Serialize)]
pub struct CopyProtection {
    pub git: bool,
    pub ignored: bool,
}
pub fn protection(root: &Path) -> Result<CopyProtection> {
    if !root.join(".git").exists() {
        return Ok(CopyProtection {
            git: false,
            ignored: false,
        });
    }
    let mut command = Command::new("git");
    command
        .current_dir(root)
        .args(["check-ignore", "--quiet", "--no-index", "--", ".bridge/"]);
    #[cfg(windows)]
    {
        use std::os::windows::process::CommandExt;
        command.creation_flags(0x08000000);
    }
    let output = command
        .output()
        .context("无法调用 Git 检查副本忽略规则；请安装 Git 或手动核对 .gitignore")?;
    match output.status.code() {
        Some(0) => Ok(CopyProtection {
            git: true,
            ignored: true,
        }),
        Some(1) => Ok(CopyProtection {
            git: true,
            ignored: false,
        }),
        _ => bail!(
            "Git 无法检查副本忽略规则：{}",
            String::from_utf8_lossy(&output.stderr).trim()
        ),
    }
}
pub(crate) fn stage_ignore(root: &Path, batch: &mut Batch) -> Result<()> {
    let state = protection(root)?;
    if !state.git || state.ignored {
        return Ok(());
    }
    let path = crate::repo_files::output(root, ".gitignore")?;
    let mut bytes = crate::file_batch::original(&path)?.unwrap_or_default();
    let newline: &[u8] = if bytes.windows(2).any(|w| w == b"\r\n") {
        b"\r\n"
    } else {
        b"\n"
    };
    if !bytes.is_empty() && !bytes.ends_with(b"\n") {
        bytes.extend(newline);
    }
    bytes.extend(b".bridge/");
    bytes.extend(newline);
    batch.add(path, bytes)
}
pub fn set_copy_options(
    project: &str,
    settings: &Path,
    enabled: bool,
    add_ignore: bool,
) -> Result<()> {
    let mut batch = Batch::default();
    let mut prefs = AppSettings::load(settings)?;
    prefs
        .record_copies
        .entry(project.into())
        .or_default()
        .enabled = enabled;
    batch.add(settings.into(), serde_json::to_vec_pretty(&prefs)?)?;
    if add_ignore {
        stage_ignore(Path::new(project), &mut batch)?;
    }
    batch.apply()
}
pub fn add_ignore(project: &str) -> Result<()> {
    let mut batch = Batch::default();
    stage_ignore(Path::new(project), &mut batch)?;
    batch.apply()
}
