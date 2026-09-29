use anyhow::{bail, Result};
use bridge_core::{paths, Bridge};
use clap::Parser;
#[path = "cli_args.rs"]
mod args;
#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;
use args::{Cli, Command};

pub enum Output {
    Serve(Bridge),
    Text(String),
    Exit(i32),
}
pub fn run() -> Result<Output> {
    let command = Cli::parse().command;
    let bridge = match Bridge::from_env() {
        Ok(bridge) => bridge,
        Err(error) if matches!(command, Some(Command::Hook { .. })) => {
            eprintln!("Bridge 钩子：{error}");
            return Ok(Output::Exit(0));
        }
        Err(error) => return Err(error),
    };
    let Some(command) = command else {
        return Ok(Output::Serve(bridge));
    };
    let result = match command {
        Command::Hook { agent } => {
            crate::hook::run(&bridge, &agent);
            return Ok(Output::Exit(0));
        }
        Command::Rebind { project, agent } => bridge.rebind(&project.resolve()?, &agent)?,
        Command::Wait(options) => return Ok(Output::Exit(crate::wait::run(&bridge, options)?)),
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
        Command::Status => bridge.status_text()?,
        Command::Show {
            project,
            include_older,
        } => bridge.show_sessions_text(&project.resolve()?, include_older)?,
        Command::Read(project) => bridge.read_text(&project.resolve()?)?,
        Command::Post { values, to } => {
            let (project, content) = if values.len() == 1 {
                (".", values[0].as_str())
            } else {
                (values[0].as_str(), values[1].as_str())
            };
            bridge.post_text(&paths::cli_project(project)?, content, &to)?
        }
    };
    Ok(Output::Text(result))
}
