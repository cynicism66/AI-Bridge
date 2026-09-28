use crate::{atomic_file, file_batch::Batch, redact, repo_files};
use anyhow::{bail, Context, Result};
use std::{
    fs,
    io::Write,
    path::{Path, PathBuf},
};

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
        .map(|p| redact::scan(&p.to_string_lossy()).text)
        .collect::<Vec<_>>()
        .join("\n");
    let mut message = format!("预览文件：{}\n即将写入：\n{paths}\n", preview.display());
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
