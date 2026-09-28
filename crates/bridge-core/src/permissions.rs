use crate::{collaboration, templates::Template, Bridge};
use anyhow::{bail, Result};

fn relative(path: &str) -> bool {
    !path.is_empty()
        && path != "."
        && !path.starts_with('/')
        && !path.contains(':')
        && !path.split('/').any(|p| p == ".." || p.is_empty())
}
pub fn validate(rule: &str) -> Result<()> {
    let rule = rule.replace('\\', "/");
    if !relative(&rule) || rule.contains('?') || rule.contains('[') || rule.contains(']') {
        bail!("无效 write 规则：{rule}；只能使用项目内相对路径");
    }
    let valid = if let Some(dir) = rule.strip_suffix("/**") {
        !dir.is_empty() && !dir.contains('*')
    } else if rule.contains('*') {
        let leaf = rule.rsplit('/').next().unwrap_or("");
        leaf.starts_with("*.") && leaf.len() > 2 && rule.matches('*').count() == 1
    } else {
        true
    };
    if !valid {
        bail!("不支持的 write 规则：{rule}");
    }
    Ok(())
}
pub fn matches(rule: &str, path: &str) -> bool {
    let rule = crate::paths::file("", rule);
    if let Some(dir) = rule.strip_suffix("/**") {
        return path == dir || path.starts_with(&format!("{dir}/"));
    }
    if let Some(ext) = rule.strip_prefix("*.") {
        return path
            .rsplit('/')
            .next()
            .unwrap_or("")
            .ends_with(&format!(".{ext}"));
    }
    if let Some((dir, leaf)) = rule.rsplit_once('/') {
        if let Some(ext) = leaf.strip_prefix("*.") {
            return path
                .rsplit_once('/')
                .is_some_and(|(parent, file)| parent == dir && file.ends_with(&format!(".{ext}")));
        }
    }
    rule == path
}
pub fn allowed(rules: &[String], path: &str) -> bool {
    relative(path) && (rules.is_empty() || rules.iter().any(|rule| matches(rule, path)))
}
impl Bridge {
    pub(crate) fn claim_permission(
        &self,
        project: &str,
        files: &[String],
    ) -> Result<Option<String>> {
        let conn = self.database.open()?;
        let Some(info) = collaboration::load(&conn, project)? else {
            return Ok(Some(crate::tools::PENDING_INIT.into()));
        };
        let t: Template = serde_json::from_str(&info.template_json)?;
        let roles = collaboration::roles(&conn, project)?;
        let role = roles
            .get(&self.agent)
            .and_then(|slot| t.roles.iter().find(|r| &r.slot == slot));
        let invalid = files
            .iter()
            .filter(|file| role.is_none_or(|r| !allowed(&r.write, file)))
            .cloned()
            .collect::<Vec<_>>();
        if invalid.is_empty() {
            return Ok(None);
        }
        let slot = role.map_or("未分配", |r| r.slot.as_str());
        let range = role.map_or("无（请用户分配职务）".into(), |r| {
            if r.write.is_empty() {
                "项目内路径（不限制）".into()
            } else {
                r.write.join("、")
            }
        });
        Ok(Some(format!("认领失败：以下文件超出你（{}，职务：{slot}）的可写范围：{}。你的可写范围：{range}。如确需修改，请先用 send_message 和规划方或用户协商。", self.agent, invalid.join("、"))))
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rule_boundaries() -> Result<()> {
        for (rule, yes, no) in [
            ("src/**", "src/deep/a.rs", "src2/a.rs"),
            ("*.rs", "a/b.rs", "a/b.rs.txt"),
            ("src/*.rs", "src/a.rs", "src/deep/a.rs"),
            ("Cargo.toml", "Cargo.toml", "sub/Cargo.toml"),
        ] {
            validate(rule)?;
            let yes = crate::paths::file("", yes);
            let no = crate::paths::file("", no);
            assert!(matches(rule, &yes));
            assert!(!matches(rule, &no));
        }
        for rule in ["../x", "/x", "c:/x", "src/**/x", "src/a*", "a?.rs"] {
            assert!(validate(rule).is_err());
        }
        for file in ["../x.rs", "/x.rs", "c:/x.rs"] {
            assert!(!allowed(&["*.rs".into()], file));
            assert!(!allowed(&[], file));
        }
        assert!(allowed(&[], "any/file"));
        Ok(())
    }
}
