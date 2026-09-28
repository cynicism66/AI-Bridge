use anyhow::Result;
use bridge_core::paths;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(name = "bridge", about = "Bridge：由用户控制的协作公告板")]
pub(super) struct Cli {
    #[command(subcommand)]
    pub(super) command: Option<Command>,
}

#[derive(Args)]
pub(super) struct Project {
    /// 项目目录，默认当前目录
    pub(super) project: Option<String>,
}

impl Project {
    pub(super) fn resolve(&self) -> Result<String> {
        paths::cli_project(self.project.as_deref().unwrap_or("."))
    }
}

#[derive(Args)]
pub(super) struct Switch {
    #[command(flatten)]
    pub(super) project: Project,
    /// 设置全局总开关
    #[arg(long = "global")]
    pub(super) global: bool,
}

#[derive(Subcommand)]
pub(super) enum Command {
    /// 只读等待发给指定 AI 的新消息
    Wait(crate::wait::Options),
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
