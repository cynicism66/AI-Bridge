use crate::atomic_file;
use anyhow::{bail, Result};
use std::{
    fs,
    path::{Path, PathBuf},
};

pub fn original(path: &Path) -> Result<Option<Vec<u8>>> {
    match fs::symlink_metadata(path) {
        Ok(m) if !m.is_file() || m.file_type().is_symlink() => {
            bail!("目标必须是普通文件，不能是链接或目录")
        }
        Ok(_) => Ok(Some(fs::read(path)?)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
        Err(e) => Err(e.into()),
    }
}
struct Entry {
    path: PathBuf,
    old: Option<Vec<u8>>,
    new: Vec<u8>,
}
#[derive(Default)]
pub struct Batch {
    files: Vec<Entry>,
    applied: usize,
    created: Vec<PathBuf>,
}
impl Batch {
    pub fn add(&mut self, path: PathBuf, new: Vec<u8>) -> Result<()> {
        if self.files.iter().any(|e| e.path == path) {
            bail!("重复的输出文件");
        }
        self.files.push(Entry {
            old: original(&path)?,
            path,
            new,
        });
        Ok(())
    }
    pub fn paths(&self) -> Vec<&Path> {
        self.files.iter().map(|e| e.path.as_path()).collect()
    }
    fn parent(&mut self, path: &Path) -> Result<()> {
        if path.is_dir() {
            return Ok(());
        }
        if let Some(parent) = path.parent() {
            self.parent(parent)?;
        }
        fs::create_dir(path)?;
        self.created.push(path.into());
        Ok(())
    }
    pub fn apply(&mut self) -> Result<()> {
        for e in &self.files {
            if original(&e.path)? != e.old {
                bail!("预览后目标文件发生变化，请重新执行并确认");
            }
        }
        let result = (|| -> Result<()> {
            for i in 0..self.files.len() {
                let parent = self.files[i].path.parent().unwrap().to_owned();
                self.parent(&parent)?;
                let e = &self.files[i];
                atomic_file::write(&e.path, &e.new)?;
                self.applied += 1;
            }
            Ok(())
        })();
        if let Err(error) = result {
            self.rollback()?;
            return Err(error);
        }
        Ok(())
    }
    pub fn rollback(&mut self) -> Result<()> {
        let mut errors = Vec::new();
        for e in self.files[..self.applied].iter().rev() {
            let result = match &e.old {
                Some(bytes) => atomic_file::write(&e.path, bytes),
                None => fs::remove_file(&e.path).map_err(Into::into),
            };
            if let Err(e) = result {
                errors.push(e.to_string());
            }
        }
        self.applied = 0;
        for dir in self.created.iter().rev() {
            let _ = fs::remove_dir(dir);
        }
        self.created.clear();
        if !errors.is_empty() {
            bail!("恢复原文件失败：{}", errors.join("；"));
        }
        Ok(())
    }
}
