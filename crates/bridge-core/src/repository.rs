use crate::paths::{explicit_project, project_on};
use anyhow::{bail, Context, Result};
use std::path::{Path, PathBuf};

#[derive(Clone, Debug)]
pub struct Project {
    pub key: String,
    pub worktree: String,
    pub branch: String,
}

fn normalized(path: &Path) -> Result<String> {
    let value = path.to_string_lossy();
    let value = if let Some(unc) = value.strip_prefix(r"\\?\UNC\") {
        format!(r"\\{unc}")
    } else {
        value.strip_prefix(r"\\?\").unwrap_or(&value).to_owned()
    };
    project_on(&value, cfg!(windows))
}

fn directory(base: &Path, value: &str) -> Result<PathBuf> {
    if value.trim().is_empty() {
        bail!("Git 目录路径为空");
    }
    let path = base.join(value.trim());
    if !path.is_dir() {
        bail!("Git 目录不存在：{}", path.display());
    }
    Ok(std::fs::canonicalize(path)?)
}

fn metadata(root: &Path, marker: &Path) -> Result<Project> {
    let gitdir = if marker.is_dir() {
        std::fs::canonicalize(marker)?
    } else {
        let content = std::fs::read_to_string(marker)?;
        let value = content
            .trim()
            .strip_prefix("gitdir:")
            .context(".git 文件缺少 gitdir:")?;
        directory(root, value)?
    };
    let common_file = gitdir.join("commondir");
    let common = if common_file.exists() {
        directory(&gitdir, &std::fs::read_to_string(common_file)?)?
    } else {
        gitdir.clone()
    };
    let key = if common.file_name().is_some_and(|name| name == ".git") {
        common.parent().context("Git 公共目录没有上级目录")?
    } else {
        &common
    };
    let head = std::fs::read_to_string(gitdir.join("HEAD")).unwrap_or_default();
    let head = head.trim();
    let branch = if let Some(name) = head.strip_prefix("ref: refs/heads/") {
        name.to_owned()
    } else if head.is_empty() {
        "-".into()
    } else {
        head.chars().take(7).collect()
    };
    Ok(Project {
        key: normalized(key)?,
        worktree: normalized(root)?,
        branch,
    })
}

pub fn resolve(input: &str) -> Result<Project> {
    let key = project_on(input, cfg!(windows))?;
    let fallback = Project {
        key: key.clone(),
        worktree: key,
        branch: "-".into(),
    };
    let input = explicit_project(input.trim(), cfg!(windows))?;
    let start = Path::new(&input);
    // 无须用户安装 Git；失败不影响按原路径使用 Bridge。
    for root in start.ancestors() {
        let marker = root.join(".git");
        let bare = root.join("HEAD").is_file()
            && root.join("objects").is_dir()
            && (root.join("refs").is_dir() || root.join("packed-refs").is_file());
        let marker = if marker.exists() {
            marker
        } else if bare {
            root.into()
        } else {
            continue;
        };
        match metadata(root, &marker) {
            Ok(project) => return Ok(project),
            Err(error) => {
                eprintln!(
                    "Bridge：无法解析 Git 项目，退回原路径：{}",
                    error.to_string().replace(['\r', '\n'], " ")
                );
                return Ok(fallback);
            }
        }
    }
    Ok(fallback)
}

#[cfg(test)]
#[path = "repository_tests.rs"]
mod tests;
