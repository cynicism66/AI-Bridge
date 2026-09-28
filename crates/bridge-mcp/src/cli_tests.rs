use super::*;

#[test]
fn permission_parses_repeated_lists_and_explicit_empty() {
    let cli = Cli::try_parse_from([
        "bridge",
        "permission",
        "D:/example",
        "执行",
        "--allow",
        "写代码",
        "--allow",
        "测试",
        "--deny",
        "改审查意见",
        "--write",
        "",
    ])
    .unwrap();
    let Some(Command::Permission {
        values,
        allow,
        deny,
        write,
        reset,
    }) = cli.command
    else {
        panic!("应解析为权限命令");
    };
    assert_eq!(values, ["D:/example", "执行"]);
    assert_eq!(allow.unwrap(), ["写代码", "测试"]);
    assert_eq!(deny.unwrap(), ["改审查意见"]);
    assert_eq!(write.unwrap(), [""]);
    assert!(!reset);
    let cli = Cli::try_parse_from(["bridge", "permission", "执行", "--reset"]).unwrap();
    let Some(Command::Permission {
        values,
        allow,
        deny,
        write,
        reset,
    }) = cli.command
    else {
        panic!("应解析为权限命令");
    };
    assert_eq!(values, ["执行"]);
    assert!(allow.is_none() && deny.is_none() && write.is_none() && reset);
}

#[test]
fn permission_reset_rejects_edits_and_slot_is_required() {
    for flag in ["--allow", "--deny", "--write"] {
        let error =
            Cli::try_parse_from(["bridge", "permission", "执行", "--reset", flag, "docs/**"])
                .err()
                .unwrap();
        assert_eq!(error.kind(), clap::error::ErrorKind::ArgumentConflict);
    }
    assert!(Cli::try_parse_from(["bridge", "permission", "--reset"]).is_err());
}
