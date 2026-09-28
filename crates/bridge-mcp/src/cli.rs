use anyhow::{bail, Result};
use bridge_core::{paths, Bridge};
use clap::{Args, Parser, Subcommand};

#[cfg(test)]
#[path = "cli_tests.rs"]
mod tests;

#[derive(Parser)]
#[command(name = "bridge", about = "Bridge：由用户控制的协作公告板")]
struct Cli {
    #[command(subcommand)]
    command: Option<Command>,
}

#[derive(Args)]
struct Project {
    /// 项目目录，默认当前目录
    project: Option<String>,
}

impl Project {
    fn resolve(&self) -> Result<String> {
        paths::cli_project(self.project.as_deref().unwrap_or("."))
    }
}

#[derive(Args)]
struct Switch {
    #[command(flatten)]
    project: Project,
    /// 设置全局总开关
    #[arg(long = "global")]
    global: bool,
}

#[derive(Subcommand)]
enum Command {
    /// 开启项目或全局开关
    On(Switch),
    /// 关闭项目或全局开关
    Off(Switch),
    /// 显示全局开关及所有项目状态
    Status,
    /// 从列表移除并关闭协作，保留全部协作数据（仅用户操作）
    Forget(Project),
    /// 彻底删除 Bridge 项目数据并隐藏，不修改项目文件或其他 AI 配置（仅用户操作）
    Purge {
        #[command(flatten)]
        project: Project,
        #[arg(long)]
        yes: bool,
        #[arg(long)]
        force: bool,
    },
    /// 设置项目内指定 AI 的开关
    Agent {
        #[arg(num_args = 2..=3, value_names = ["项目或AI", "AI或开关", "开关"])]
        values: Vec<String>,
    },
    /// 初始化项目协作（仅用户操作）
    Init {
        #[command(flatten)]
        project: Project,
        #[arg(long)]
        template: String,
        #[arg(long = "role", required = true)]
        roles: Vec<String>,
        #[arg(long)]
        goal: String,
        #[arg(long)]
        template_file: Option<std::path::PathBuf>,
        #[arg(long)]
        write_rules: bool,
        #[arg(long)]
        no_kickoff: bool,
    },
    /// 调整职务，目标已占用时交换双方职务（仅用户操作）
    Role {
        #[arg(num_args = 2..=3, value_names = ["项目或AI", "AI或职务", "职务"])]
        values: Vec<String>,
    },
    /// 编辑项目职务权限（仅用户操作；未指定的列表保留，空字符串清空）
    Permission {
        #[arg(required = true, num_args = 1..=2, value_names = ["项目或职务", "职务"])]
        values: Vec<String>,
        #[arg(long, action = clap::ArgAction::Append)]
        allow: Option<Vec<String>>,
        #[arg(long, action = clap::ArgAction::Append)]
        deny: Option<Vec<String>>,
        #[arg(long, action = clap::ArgAction::Append)]
        write: Option<Vec<String>>,
        #[arg(long, conflicts_with_all = ["allow", "deny", "write"])]
        reset: bool,
    },
    /// 导出协作记录，预览后由用户确认
    Export {
        #[command(flatten)]
        project: Project,
        #[arg(long)]
        out: Option<String>,
        #[arg(long)]
        yes: bool,
    },
    /// 交接给一个 AI 全盘接手，预览后由用户确认
    Handover {
        #[command(flatten)]
        project: Project,
        #[arg(long)]
        to: String,
        #[arg(long)]
        yes: bool,
    },
    /// 查看公告板
    Show {
        #[command(flatten)]
        project: Project,
        /// 展开超过两小时未活跃的会话
        #[arg(long)]
        include_older: bool,
    },
    /// 读取给 human 的未读消息
    Read(Project),
    /// 以 human 身份发消息
    Post {
        // 与 Python argparse 一致：一个位置参数为内容，两个为项目和内容。
        #[arg(num_args = 1..=2, value_names = ["项目或内容", "内容"])]
        values: Vec<String>,
        #[arg(long, default_value = "all", value_parser = ["all", "claude", "codex"])]
        to: String,
    },
    /// 查看项目交互历史
    History {
        #[command(flatten)]
        project: Project,
        #[arg(long, default_value_t = 50, allow_hyphen_values = true)]
        limit: i64,
        #[arg(long)]
        agent: Option<String>,
        #[arg(long, value_parser = ["status", "message", "claim", "release", "expire", "switch", "agent_switch", "init", "role", "handover", "permission", "forget"])]
        kind: Option<String>,
    },
}

pub fn run(bridge: &Bridge) -> Result<Option<String>> {
    let Some(command) = Cli::parse().command else {
        return Ok(None);
    };
    let result = match command {
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
    Ok(Some(result))
}
