use anyhow::{bail, Result};
use bridge_core::{paths, Bridge};
use clap::Parser;

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;

#[path = "cli_args.rs"]
mod args;
use args::{Cli, Command};

pub enum Output {
    Serve,
    Text(String),
    Exit(i32),
}

pub fn run(bridge: &Bridge) -> Result<Output> {
    let Some(command) = Cli::parse().command else {
        return Ok(Output::Serve);
    };
    let result = match command {
        Command::Wait(options) => return Ok(Output::Exit(crate::wait::run(bridge, options)?)),
        Command::On(ref args) | Command::Off(ref args) => {
            if args.global && args.project.project.is_some() {
                bail!("--global 不能同时指定项目");
            }
            let project = if args.global {
                None
            } else {
                Some(args.project.resolve()?)
            };
            let exists = project
                .as_ref()
                .is_some_and(|p| std::path::Path::new(p).is_dir());
            bridge.toggle_text(
                matches!(command, Command::On(_)),
                project.as_deref(),
                exists,
            )?
        }
        Command::Agent { values } => {
            let (project, agent, state) = if values.len() == 2 {
                (".", &values[0], &values[1])
            } else {
                (values[0].as_str(), &values[1], &values[2])
            };
            let enabled = match state.as_str() {
                "on" => true,
                "off" => false,
                _ => bail!("AI 开关必须是 on 或 off"),
            };
            bridge.agent_toggle_text(&paths::cli_project(project)?, agent, enabled)?
        }
        Command::Init {
            project,
            template,
            roles,
            goal,
            template_file,
            write_rules,
            no_kickoff,
        } => bridge.init_text(
            &project.resolve()?,
            bridge_core::initialization::InitOptions {
                template: &template,
                template_file: template_file.as_deref(),
                roles: &roles,
                goal: &goal,
                write_rules,
                no_kickoff,
            },
        )?,
        Command::Role { values } => {
            let (project, agent, slot) = if values.len() == 2 {
                (".", &values[0], &values[1])
            } else {
                (values[0].as_str(), &values[1], &values[2])
            };
            bridge.role_text(&paths::cli_project(project)?, agent, slot)?
        }
        Command::Permission {
            values,
            allow,
            deny,
            write,
            reset,
        } => {
            let (project, slot) = if values.len() == 1 {
                (".", &values[0])
            } else {
                (values[0].as_str(), &values[1])
            };
            bridge.permission_text(
                &paths::cli_project(project)?,
                slot,
                bridge_core::permission_edit::PermissionPatch {
                    allow,
                    deny,
                    write,
                    reset,
                },
            )?
        }
        Command::Export { project, out, yes } => {
            bridge.transfer_text(&project.resolve()?, out.as_deref(), None, yes)?
        }
        Command::Handover { project, to, yes } => {
            bridge.transfer_text(&project.resolve()?, None, Some(&to), yes)?
        }
        Command::Status => bridge.status_text()?,
        Command::Forget(project) => bridge.forget_project(
            &project.resolve()?,
            &bridge_core::app_settings::AppSettings::from_env_path()?,
        )?,
        Command::Purge {
            project,
            yes,
            force,
        } => bridge.purge_project(
            &project.resolve()?,
            &bridge_core::app_settings::AppSettings::from_env_path()?,
            yes,
            force,
        )?,
        Command::Show {
            project,
            include_older,
        } => bridge.show_sessions_text(&project.resolve()?, include_older)?,
        Command::Read(args) => bridge.read_text(&args.resolve()?)?,
        Command::Post { values, to } => {
            let (project, content) = if values.len() == 1 {
                (".", values[0].as_str())
            } else {
                (values[0].as_str(), values[1].as_str())
            };
            bridge.post_text(&paths::cli_project(project)?, content, &to)?
        }
        Command::History {
            project,
            limit,
            agent,
            kind,
        } => bridge.history(
            &project.resolve()?,
            limit,
            agent.as_deref(),
            kind.as_deref(),
        )?,
    };
    Ok(Output::Text(result))
}
