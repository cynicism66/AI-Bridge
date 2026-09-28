use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};
pub const START: &str = "<!-- AI Bridge 章程开始（由 bridge 生成，请勿手动修改此区块） -->";
pub const END: &str = "<!-- AI Bridge 章程结束 -->";
fn positions(data: &[u8], marker: &[u8]) -> Vec<usize> {
    data.windows(marker.len())
        .enumerate()
        .filter(|(_, w)| *w == marker)
        .map(|(i, _)| i)
        .collect()
}
pub fn replace(original: &[u8], charter: &str) -> Result<Vec<u8>> {
    if charter.contains(START) || charter.contains(END) {
        bail!("章程不能包含规则文件标记");
    }
    let starts = positions(original, START.as_bytes());
    let ends = positions(original, END.as_bytes());
    let block = format!("{START}\n{charter}\n{END}").into_bytes();
    match (starts.as_slice(), ends.as_slice()) {
        ([], []) => {
            let mut result = original.to_vec();
            if !result.is_empty() && !result.ends_with(b"\n") {
                result.push(b'\n');
            }
            result.extend(block);
            Ok(result)
        }
        ([start], [end]) if start < end => {
            let mut result = original[..*start].to_vec();
            result.extend(block);
            result.extend_from_slice(&original[end + END.len()..]);
            Ok(result)
        }
        _ => bail!("规则文件的章程标记不完整、重复或顺序错误，请用户修复后再写入"),
    }
}
pub struct RulesFiles {
    files: Vec<(PathBuf, Option<Vec<u8>>, Vec<u8>)>,
}
impl RulesFiles {
    pub fn prepare(root: &Path, charter: &str) -> Result<Self> {
        let mut files = Vec::new();
        for name in ["AGENTS.md", "CLAUDE.md"] {
            let path = root.join(name);
            if std::fs::symlink_metadata(&path).is_ok_and(|m| m.file_type().is_symlink()) {
                bail!(
                    "规则文件不能是符号链接：{}",
                    crate::display_path(&path.to_string_lossy())
                );
            }
            let original = match std::fs::read(&path) {
                Ok(bytes) => Some(bytes),
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => None,
                Err(e) => return Err(e).context("读取规则文件失败"),
            };
            let result = replace(original.as_deref().unwrap_or(&[]), charter)?;
            files.push((path, original, result));
        }
        Ok(Self { files })
    }
    pub fn write(&self) -> Result<()> {
        for (index, (path, _, result)) in self.files.iter().enumerate() {
            if let Err(error) = crate::atomic_file::write(path, result) {
                self.restore_count(index)?;
                return Err(error).context("写入章程失败，已恢复原规则文件");
            }
        }
        Ok(())
    }
    pub fn append_to(&self, batch: &mut crate::file_batch::Batch) -> Result<()> {
        for (path, _, bytes) in &self.files {
            batch.add(path.clone(), bytes.clone())?;
        }
        Ok(())
    }
    pub fn restore(&self) -> Result<()> {
        self.restore_count(self.files.len())
    }
    fn restore_count(&self, count: usize) -> Result<()> {
        for (path, original, _) in &self.files[..count] {
            match original {
                Some(bytes) => crate::atomic_file::write(path, bytes)?,
                None => match std::fs::remove_file(path) {
                    Ok(()) => (),
                    Err(e) if e.kind() == std::io::ErrorKind::NotFound => (),
                    Err(e) => return Err(e.into()),
                },
            }
        }
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_bytes_and_is_idempotent() -> Result<()> {
        let prefix = b"\xef\xbb\xbf# user\r\n\xff\r\n";
        let suffix = b"\r\nuser tail\r\n";
        let mut original = prefix.to_vec();
        original.extend(format!("{START}old{END}").as_bytes());
        original.extend(suffix);
        let result = replace(&original, "新章程")?;
        assert!(result.starts_with(prefix));
        assert!(result.ends_with(suffix));
        assert_eq!(replace(&result, "新章程")?, result);
        let added = replace(prefix, "内容")?;
        assert!(added.starts_with(prefix));
        assert_eq!(replace(&added, "内容")?, added);
        assert!(replace(START.as_bytes(), "x").is_err());
        assert!(replace(format!("{START}{START}{END}").as_bytes(), "x").is_err());
        Ok(())
    }
}
