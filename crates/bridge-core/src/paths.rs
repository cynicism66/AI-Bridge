use anyhow::{bail, Result};

const ABSOLUTE: &str = "project 必须是项目根目录的绝对路径";

pub fn explicit_project(path: &str, windows: bool) -> Result<String> {
    if !windows {
        return Ok(path.into());
    }
    let bytes = path.as_bytes();
    if bytes.len() >= 3 && bytes[0] == b'/' && bytes[1].is_ascii_alphabetic() && bytes[2] == b'/' {
        return Ok(format!("{}:{}", bytes[1] as char, &path[2..]));
    }
    let slash = path.replace('\\', "/");
    if slash.starts_with('/') && !slash.starts_with("//") {
        bail!("Windows 项目路径必须带盘符或使用 UNC 路径；Git Bash 路径请用 /d/x 格式");
    }
    if bytes.len() >= 2 && bytes[1] == b':' && !slash[2..].starts_with('/') {
        bail!(ABSOLUTE);
    }
    Ok(slash)
}

fn absolute(path: &str, windows: bool) -> bool {
    if windows {
        let b = path.as_bytes();
        (b.len() >= 3 && b[1] == b':' && b[2] == b'/') || path.starts_with("//")
    } else {
        path.starts_with('/')
    }
}

fn collapse(path: &str, windows: bool) -> String {
    let path = if windows {
        path.replace('\\', "/").to_lowercase()
    } else {
        path.to_owned()
    };
    let mut prefix = "";
    let mut remaining = path.as_str();
    if windows && path.as_bytes().get(1) == Some(&b':') {
        prefix = &path[..2];
        remaining = &path[2..];
    }
    let rooted = remaining.starts_with('/');
    let double = prefix.is_empty() && remaining.starts_with("//") && !remaining.starts_with("///");
    let mut parts = Vec::new();
    // UNC 的服务器和共享名属于根，不能被 .. 消掉。
    let floor = if windows && double { 2 } else { 0 };
    for part in remaining.split('/') {
        match part {
            "" | "." => (),
            ".." if parts.len() > floor && parts.last() != Some(&"..") => {
                parts.pop();
            }
            ".." if !rooted => parts.push(part),
            ".." => (),
            _ => parts.push(part),
        }
    }
    let root = if double {
        "//"
    } else if rooted {
        "/"
    } else {
        ""
    };
    let result = format!("{prefix}{root}{}", parts.join("/"));
    if result.is_empty() {
        ".".into()
    } else {
        result
    }
}

pub fn project_on(path: &str, windows: bool) -> Result<String> {
    if path.trim().is_empty() {
        bail!("缺少 project 参数：请填当前项目根目录的绝对路径");
    }
    let path = explicit_project(path.trim(), windows)?;
    if !absolute(&path, windows) {
        bail!(ABSOLUTE);
    }
    Ok(collapse(&path, windows)
        .replace('\\', "/")
        .trim_end_matches('/')
        .into())
}

pub fn project(path: &str) -> Result<String> {
    project_on(path, cfg!(windows))
}

pub fn cli_project(path: &str) -> Result<String> {
    let path = explicit_project(path, cfg!(windows))?;
    let path = if absolute(&path, cfg!(windows)) {
        path
    } else {
        std::env::current_dir()?
            .join(path)
            .to_string_lossy()
            .into_owned()
    };
    project(&path)
}

pub fn file_on(project: &str, path: &str, windows: bool) -> String {
    let path = path.trim().replace('\\', "/");
    let full = collapse(&path, windows);
    if absolute(&full, windows) {
        if let Some(relative) = full.strip_prefix(&format!("{project}/")) {
            return relative.into();
        }
    }
    full
}

pub fn file(project: &str, path: &str) -> String {
    file_on(project, path, cfg!(windows))
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn project_rules() -> Result<()> {
        for (input, expected) in [
            (" C:\\Work\\Demo\\ ", "c:/work/demo"),
            ("/d/x/../y", "d:/y"),
            ("\\\\Server\\Share\\a\\..\\x", "//server/share/x"),
        ] {
            assert_eq!(project_on(input, true)?, expected);
        }
        for invalid in ["", ".", "../x", "\\x", "/x", "/abc/x", "C:x"] {
            assert!(project_on(invalid, true).is_err(), "{invalid}");
        }
        assert_eq!(project_on("/Work/Demo/../X", false)?, "/Work/X");
        Ok(())
    }
    #[test]
    fn file_rules() {
        for (input, expected) in [
            ("SRC\\A.PY", "src/a.py"),
            ("C:\\Work\\src\\..\\A.py", "a.py"),
            ("C:/Work2/A.py", "c:/work2/a.py"),
            ("D:/Work/A.py", "d:/work/a.py"),
            ("src/../../outside.py", "../outside.py"),
            ("./", "."),
        ] {
            assert_eq!(file_on("c:/work", input, true), expected);
        }
        assert_eq!(file_on("/work", "/work/Src/A.py", false), "Src/A.py");
        assert_eq!(file_on("/work", "/outside/A.py", false), "/outside/A.py");
    }
}
