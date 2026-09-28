use crate::{atomic_file, file_batch::Batch, redact, repo_files};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

// 仅用于显示；文件读写与路径校验继续使用原始路径。
pub fn display_path(path: &Path) -> String {
    let text = path.to_string_lossy();
    let text = if let Some(unc) = text.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        text.strip_prefix(r"\\?\").unwrap_or(&text).to_owned()
    };
    if text.as_bytes().get(1) == Some(&b':') && text.as_bytes()[0].is_ascii_alphabetic() {
        format!(
            "{}{}",
            text[..1].to_ascii_uppercase(),
            text[1..].replace('/', "\\")
        )
    } else if text.starts_with("//") || text.starts_with(r"\\") {
        text.replace('/', "\\")
    } else {
        text
    }
}

pub fn create(root: &Path, body: &str, batch: &Batch) -> Result<(PathBuf, String)> {
    let root = fs::canonicalize(root)?;
    let temp = fs::canonicalize(std::env::temp_dir())?;
    if temp.starts_with(&root) {
        bail!("临时目录位于仓库内，请把 TEMP/TMP 设置到仓库外再导出");
    }
    let dir = temp.join(format!(
        "ai-bridge-preview-{}",
        crate::sessions::random_id()?
    ));
    fs::create_dir(&dir)?;
    let preview = dir.join("预览.md");
    if let Err(e) = atomic_file::write(&preview, body.as_bytes()) {
        let _ = fs::remove_dir(&dir);
        return Err(e);
    }
    let paths = batch
        .paths()
        .iter()
        .map(|p| redact::scan(&display_path(p)).text)
        .collect::<Vec<_>>()
        .join("\n");
    let mut message = format!(
        "预览文件：{}\n即将写入：\n{paths}\n",
        display_path(&preview)
    );
    for url in repo_files::remotes(&root)? {
        message.push_str(&format!(
            "此仓库有远程地址 {}，提交并推送后内容可能公开，请仔细检查预览文件\n",
            redact::scan(&url).text
        ));
    }
    Ok((preview, message))
}
pub fn confirm(message: &str, yes: bool) -> Result<bool> {
    let mut out = std::io::stdout().lock();
    out.write_all(message.as_bytes())?;
    if yes {
        out.write_all("已使用 --yes，跳过询问。\n".as_bytes())?;
        out.flush()?;
        return Ok(true);
    }
    out.write_all("确认写入？(y/N) ".as_bytes())?;
    out.flush()?;
    let mut input = String::new();
    std::io::stdin()
        .read_line(&mut input)
        .context("读取确认输入失败")?;
    Ok(input.trim() == "y")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn display_paths_preserve_drive_and_unc_roots() {
        for (raw, expected) in [
            (r"\\?\C:\目录\预览.md", r"C:\目录\预览.md"),
            (r"\\?\UNC\server\share\预览.md", r"\\server\share\预览.md"),
            (r"C:\目录\预览.md", r"C:\目录\预览.md"),
            (r"\\server\share\预览.md", r"\\server\share\预览.md"),
            (r"d:/bridge-demo\AGENTS.md", r"D:\bridge-demo\AGENTS.md"),
            ("//server/share/预览.md", r"\\server\share\预览.md"),
        ] {
            assert_eq!(display_path(Path::new(raw)), expected);
        }
    }
}
