//! 仅从指定配置的项目路径投影发现项目；不访问凭证、登录或会话文件。
use anyhow::Result;
use serde::{
    de::{IgnoredAny, MapAccess, Visitor},
    Deserialize, Deserializer, Serialize,
};
use std::{collections::BTreeSet, fmt, path::Path};

#[derive(Default, Deserialize)]
struct ClaudeProjects {
    #[serde(default, deserialize_with = "project_keys")]
    projects: Vec<String>,
}
fn project_keys<'de, D: Deserializer<'de>>(deserializer: D) -> Result<Vec<String>, D::Error> {
    struct Keys;
    impl<'de> Visitor<'de> for Keys {
        type Value = Vec<String>;
        fn expecting(&self, f: &mut fmt::Formatter) -> fmt::Result {
            f.write_str("project map")
        }
        fn visit_map<M: MapAccess<'de>>(self, mut map: M) -> Result<Self::Value, M::Error> {
            let mut keys = Vec::new();
            while let Some(key) = map.next_key::<String>()? {
                map.next_value::<IgnoredAny>()?;
                keys.push(key);
            }
            Ok(keys)
        }
    }
    deserializer.deserialize_map(Keys)
}
#[derive(Default, Deserialize)]
struct CodexProjects {
    #[serde(default, deserialize_with = "project_keys")]
    projects: Vec<String>,
}
#[derive(Debug, Serialize, PartialEq)]
pub struct DiscoveredProject {
    pub project: String,
}

pub fn discover(
    home: &Path,
    known: &BTreeSet<String>,
    hidden: &BTreeSet<String>,
) -> Vec<DiscoveredProject> {
    // 反序列化器直接丢弃未声明字段及每个项目的值，不构造账号字段的 Value 树。
    // 解析错误不可携带配置片段进入日志或前端。
    let claude = std::fs::File::open(home.join(".claude.json"))
        .ok()
        .and_then(|f| serde_json::from_reader::<_, ClaudeProjects>(f).ok())
        .unwrap_or_default();
    let codex = std::fs::read_to_string(home.join(".codex/config.toml"))
        .ok()
        .and_then(|s| toml::from_str::<CodexProjects>(&s).ok())
        .unwrap_or_default();
    claude
        .projects
        .into_iter()
        .chain(codex.projects)
        .filter_map(|p| crate::repository::resolve(&p).ok().map(|p| p.key))
        .filter(|p| !known.contains(p) && !hidden.contains(p))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .map(|project| DiscoveredProject { project })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::test_support::{key, Directory};
    #[test]
    fn only_paths_leave_discovery_and_hidden_known_are_excluded() {
        let d = Directory::new();
        let a = d.dir("项目甲");
        let b = d.dir("项目乙");
        d.write(".claude.json", &serde_json::json!({"oauthAccount":{"secret":"NEVER-RETURN"},"projects":{a.to_str().unwrap():{"secret":"ALSO-PRIVATE"},b.to_str().unwrap():{}}}).to_string());
        d.write(
            ".codex/config.toml",
            &format!("[projects.'{}']\ntrust_level='trusted'\n", a.display()),
        );
        d.write(
            ".claude/.credentials.json",
            "NOT VALID JSON: must never open this",
        );
        let results = discover(&d.0, &BTreeSet::new(), &BTreeSet::new());
        assert_eq!(results.len(), 2);
        let json = serde_json::to_value(&results).unwrap();
        for row in json.as_array().unwrap() {
            assert_eq!(row.as_object().unwrap().len(), 1);
            assert!(row["project"].is_string());
        }
        assert!(!json.to_string().contains("PRIVATE"));
        assert!(!json.to_string().contains("oauth"));
        assert!(!json.to_string().contains("NEVER"));
        assert!(discover(&d.0, &BTreeSet::from([key(&a)]), &BTreeSet::from([key(&b)])).is_empty());
        d.write(".claude.json", "{\"oauthAccount\":\"PRIVATE-BROKEN");
        assert_eq!(discover(&d.0, &BTreeSet::new(), &BTreeSet::new()).len(), 1);
    }
}
