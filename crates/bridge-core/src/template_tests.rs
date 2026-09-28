use crate::templates::*;
use anyhow::Result;
#[test]
fn builtin_templates_and_literal_substitution() -> Result<()> {
    for (name, a, b) in [
        ("任务书流程", "规划审查", "执行"),
        ("结对流程", "开发甲", "开发乙"),
    ] {
        let t = load(name, None)?;
        let roles = assignments(&t, &[format!("{a}=claude"), format!("{b}=codex")])?;
        let c = charter(&t, "保留 {执行} 字样", &roles);
        assert!(c.contains("保留 {执行} 字样"));
        assert!(c.contains("claude"));
        assert!(c.ends_with(COMMON.trim_end()));
        assert_eq!(
            expand("{goal} {执行}", "{执行}", &roles),
            if name == "任务书流程" {
                "{执行} codex"
            } else {
                "{执行} {执行}"
            }
        );
        assert!(assignments(&t, &[format!("{a}=codex")]).is_err());
        assert!(assignments(&t, &[format!("{a}=codex"), format!("{b}=codex")]).is_err());
    }
    assert!(load("自定义", None).is_err());
    assert!(load("不存在", None).is_err());
    Ok(())
}

#[test]
fn initialization_state_transitions() -> Result<()> {
    let temp = crate::test_support::Directory::new();
    let b = crate::Bridge {
        database: crate::database::Database::for_test(temp.0.join("init.db")),
        agent: "codex".into(),
        session_id: "one".into(),
    };
    let roles = vec!["开发甲=claude".into(), "开发乙=codex".into()];
    let options = || crate::initialization::InitOptions {
        template: "结对流程",
        template_file: None,
        roles: &roles,
        goal: "测试",
        write_rules: false,
        no_kickoff: true,
    };
    assert!(b.init_text("/p", options()).is_err());
    b.set_enabled(true, Some("/p"))?;
    assert!(!b.access_state(Some("/p"), Some("codex"))?.enabled());
    b.init_text("/p", options())?;
    assert!(b.access_state(Some("/p"), Some("codex"))?.enabled());
    b.set_enabled(false, Some("/p"))?;
    assert!(b.collaboration_text("/p")?.contains("协作状态：关闭"));
    assert!(!b.access_state(Some("/p"), Some("codex"))?.enabled());
    b.set_enabled(true, Some("/p"))?;
    assert!(b.access_state(Some("/p"), Some("codex"))?.enabled());
    b.init_text("/p", options())?;
    assert_eq!(
        crate::collaboration::load(&b.database.open()?, "/p")?
            .unwrap()
            .version,
        2
    );
    Ok(())
}
