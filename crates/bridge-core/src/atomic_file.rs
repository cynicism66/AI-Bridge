use anyhow::{bail, Context, Result};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::Path,
};

#[cfg(windows)]
fn replace(source: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::ffi::OsStrExt;
    #[link(name = "kernel32")]
    extern "system" {
        fn MoveFileExW(source: *const u16, target: *const u16, flags: u32) -> i32;
    }
    let source: Vec<u16> = source.as_os_str().encode_wide().chain(Some(0)).collect();
    let target: Vec<u16> = target.as_os_str().encode_wide().chain(Some(0)).collect();
    // 同目录，不允许跨卷复制；替换目标并等待写入完成。
    if unsafe { MoveFileExW(source.as_ptr(), target.as_ptr(), 0x1 | 0x8) } == 0 {
        return Err(std::io::Error::last_os_error());
    }
    Ok(())
}
#[cfg(not(windows))]
fn replace(source: &Path, target: &Path) -> std::io::Result<()> {
    fs::rename(source, target)
}

pub fn write(path: &Path, bytes: &[u8]) -> Result<()> {
    if fs::symlink_metadata(path).is_ok_and(|m| m.file_type().is_symlink() || !m.is_file()) {
        bail!("目标必须是普通文件，不能是链接或目录");
    }
    let parent = path.parent().context("目标文件没有父目录")?;
    let tmp = parent.join(format!(".bridge-{}.tmp", crate::sessions::random_id()?));
    let mut owned = false;
    let result = (|| -> Result<()> {
        let mut f = OpenOptions::new().write(true).create_new(true).open(&tmp)?;
        owned = true;
        f.write_all(bytes)?;
        f.sync_all()?;
        drop(f);
        replace(&tmp, path).context("原子替换失败，原文件保持不变")?;
        Ok(())
    })();
    if result.is_err() && owned {
        let _ = fs::remove_file(&tmp);
    }
    result
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn write_and_fail_without_partial_or_temporary_files() -> Result<()> {
        let temp = crate::test_support::Directory::new();
        let p = temp.0.join("中文.md");
        write(&p, b"first")?;
        write(&p, b"second")?;
        assert_eq!(fs::read(&p)?, b"second");
        #[cfg(windows)]
        {
            use std::os::windows::fs::OpenOptionsExt;
            let _lock = OpenOptions::new().read(true).share_mode(0).open(&p)?;
            assert!(write(&p, b"cannot replace").is_err());
        }
        assert_eq!(fs::read(&p)?, b"second");
        assert!(write(&temp.0.join("missing/file"), b"x").is_err());
        assert_eq!(fs::read_dir(&temp.0)?.count(), 1);
        Ok(())
    }
}
