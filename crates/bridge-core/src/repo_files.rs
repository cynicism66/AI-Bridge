use anyhow::{bail, Context, Result};
use std::{
    fs,
    path::{Component, Path, PathBuf},
};

pub fn output(root: &Path, value: &str) -> Result<PathBuf> {
    let root = fs::canonicalize(root).context("项目目录不存在")?;
    fn display_path(path: &str) -> String {
        let path = path.replace('\\', "/");
        if let Some(unc) = path.strip_prefix("//?/UNC/") {
            format!("//{unc}")
        } else {
            path.strip_prefix("//?/").unwrap_or(&path).to_owned()
        }
    }
    let value = display_path(value);
    let root_text = display_path(&root.to_string_lossy());
    let relative = if Path::new(&value).is_absolute() {
        let prefix = format!("{}/", root_text.trim_end_matches('/'));
        let matches = value.get(..prefix.len()).is_some_and(|part| {
            if cfg!(windows) {
                part.eq_ignore_ascii_case(&prefix)
            } else {
                part == prefix
            }
        });
        if !matches {
            bail!("输出必须位于项目目录内");
        }
        &value[prefix.len()..]
    } else {
        &value
    };
    let relative = Path::new(relative);
    let mut out = root;
    if relative.as_os_str().is_empty() {
        bail!("输出路径不能为空");
    }
    for c in relative.components() {
        let Component::Normal(part) = c else {
            bail!("输出路径必须是仓库内文件，不能包含 ..");
        };
        if part.to_string_lossy().eq_ignore_ascii_case(".git")
            || part.to_string_lossy().contains(':')
        {
            bail!("不能写入 Git 元数据或特殊路径");
        }
        out.push(part);
        if fs::symlink_metadata(&out).is_ok_and(|m| m.file_type().is_symlink()) {
            bail!("输出路径不能经过符号链接");
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if fs::symlink_metadata(&out).is_ok_and(|m| m.file_attributes() & 0x400 != 0) {
                bail!("输出路径不能经过重解析点");
            }
        }
    }
    Ok(out)
}
pub fn remotes(root: &Path) -> Result<Vec<String>> {
    let config = if root.join(".git").is_dir() {
        root.join(".git/config")
    } else {
        root.join("config")
    };
    match fs::read_to_string(config) {
        Ok(s) => Ok(parse_remotes(&s)),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(e.into()),
    }
}
pub fn parse_remotes(source: &str) -> Vec<String> {
    let mut remote = false;
    let mut urls = Vec::new();
    for raw in source.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with(['#', ';']) {
            continue;
        }
        if line.starts_with('[') {
            let section = line
                .split(']')
                .next()
                .unwrap_or("")
                .trim_start_matches('[')
                .trim();
            remote = section
                .get(..6)
                .is_some_and(|s| s.eq_ignore_ascii_case("remote"))
                && section
                    .as_bytes()
                    .get(6)
                    .is_some_and(|c| c.is_ascii_whitespace() || *c == b'.');
            continue;
        }
        if !remote {
            continue;
        }
        if let Some((key, value)) = line.split_once('=') {
            if key.trim().eq_ignore_ascii_case("url") {
                let mut quoted = false;
                let mut escaped = false;
                let mut url = String::new();
                for c in value.trim().chars() {
                    if escaped {
                        url.push(c);
                        escaped = false;
                        continue;
                    }
                    if c == '\\' {
                        escaped = true;
                        continue;
                    }
                    if c == '"' {
                        quoted = !quoted;
                        continue;
                    }
                    if !quoted && (c == '#' || c == ';') {
                        break;
                    }
                    url.push(c);
                }
                let url = url.trim();
                if !url.is_empty() {
                    urls.push(url.into());
                }
            }
        }
    }
    urls
}
fn collect(dir: &Path, paths: &mut Vec<PathBuf>) -> Result<()> {
    if !dir.exists() {
        return Ok(());
    }
    for entry in fs::read_dir(dir)? {
        let entry = entry?;
        let kind = entry.file_type()?;
        if kind.is_symlink() {
            continue;
        }
        #[cfg(windows)]
        {
            use std::os::windows::fs::MetadataExt;
            if entry.metadata()?.file_attributes() & 0x400 != 0 {
                continue;
            }
        }
        if kind.is_dir() {
            collect(&entry.path(), paths)?;
        } else if kind.is_file()
            && entry
                .path()
                .extension()
                .is_some_and(|e| e.eq_ignore_ascii_case("md"))
        {
            paths.push(entry.path());
        }
    }
    Ok(())
}
pub fn index(root: &Path) -> Result<String> {
    let mut paths = Vec::new();
    let docs = root.join("docs");
    if output(root, "docs").is_ok() {
        collect(&docs, &mut paths)?;
    }
    for name in ["README", "README.md", "AGENTS.md"] {
        let p = root.join(name);
        if p.is_file() && output(root, name).is_ok() {
            paths.push(p);
        }
    }
    paths.sort();
    let mut lines = Vec::new();
    for path in paths {
        let bytes = fs::read(&path)?;
        let text = String::from_utf8_lossy(&bytes);
        let first = text
            .lines()
            .next()
            .unwrap_or("")
            .trim_start_matches('\u{feff}')
            .trim()
            .trim_start_matches('#')
            .trim();
        lines.push(format!(
            "- {}：{}",
            path.strip_prefix(root)?
                .to_string_lossy()
                .replace('\\', "/"),
            first
        ));
    }
    if output(root, ".bridge").is_ok_and(|p| p.is_dir()) {
        lines.push("- .bridge/：本地未打码的协作记录副本（请勿提交到公开仓库）".into());
    }
    Ok(lines.join("\n"))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn remote_sections_quotes_comments_and_multiple_urls() {
        let s="[core]\nurl=ignore\n[remote \"origin\"]\n url = https://example.com/a.git # comment\n[REMOTE \"two\"]\nURL = \"https://example.com/a#b\"\n[remote.third]\nurl=git@example.com:x.git\n[remotely]\nurl=ignore";
        assert_eq!(
            parse_remotes(s),
            [
                "https://example.com/a.git",
                "https://example.com/a#b",
                "git@example.com:x.git"
            ]
        );
    }
}
