use super::*;
#[test]
fn only_minimal_commands_are_available() {
    use clap::CommandFactory;
    let mut cli = Cli::command();
    cli.build();
    let names: Vec<_> = cli.get_subcommands().map(|c| c.get_name()).collect();
    assert_eq!(
        names,
        ["on", "off", "status", "show", "read", "post", "wait", "hook", "rebind"]
    );
    for removed in [
        "agent",
        "init",
        "role",
        "permission",
        "export",
        "handover",
        "history",
        "forget",
        "purge",
        "connect",
    ] {
        assert!(Cli::try_parse_from(["bridge-mcp", removed]).is_err());
    }
    for name in ["hook", "rebind"] {
        assert!(Cli::try_parse_from(["bridge-mcp", name, "--agent", "codex"]).is_ok());
        assert!(Cli::try_parse_from(["bridge-mcp", name, "--agent", "human"]).is_err());
    }
}
