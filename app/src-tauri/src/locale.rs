use std::{collections::HashMap, sync::OnceLock};
pub fn text(key: &str) -> &'static str {
    static STRINGS: OnceLock<HashMap<String, String>> = OnceLock::new();
    STRINGS
        .get_or_init(|| {
            let source = include_str!("../../src/i18n/zh-CN.ts");
            let json = source
                .trim()
                .strip_prefix("export const zh = ")
                .unwrap()
                .strip_suffix(" as const;")
                .unwrap();
            serde_json::from_str(json).expect("invalid zh-CN resource")
        })
        .get(key)
        .map(String::as_str)
        .unwrap_or(key_fallback())
}
fn key_fallback() -> &'static str {
    "AI Bridge"
}

#[cfg(test)]
mod tests {
    #[test]
    fn shared_resource_is_valid_json_for_native_consumers() {
        assert_eq!(super::text("actionTitle"), "等你处理");
        assert_eq!(super::text("copyTitle"), "项目内协作记录副本");
    }
}
