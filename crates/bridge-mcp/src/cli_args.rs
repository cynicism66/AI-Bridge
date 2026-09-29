use anyhow::Result;
use bridge_core::paths;
use clap::{Args, Parser, Subcommand};

#[derive(Parser)]
#[command(
    name = "bridge-mcp",
    about = "Bridge：两个 AI 共用的最小协作公告板",
    disable_help_subcommand = true
)]
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
    #[arg(long)]
    pub(super) global: bool,
}
#[derive(Subcommand)]
pub(super) enum Command {
    /// 开启项目或全局开关（由用户操作）
    On(Switch),
    /// 关闭项目或全局开关（由用户操作）
    Off(Switch),
    /// 显示全局开关及所有项目状态
    Status,
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
    /// 以 human 身份发消息（由用户操作）
    Post {
        #[arg(num_args = 1..=2, value_names = ["项目或内容", "内容"])]
        values: Vec<String>,
        #[arg(long, default_value = "all", value_parser = ["all", "claude", "codex"])]
        to: String,
    },
    /// 只读等待发给指定 AI 的新消息
    Wait(crate::wait::Options),
    /// Stop 钩子：从标准输入读 JSON，为收件窗口投递消息
    Hook {
        #[arg(long, value_parser = ["claude", "codex"])]
        agent: String,
    },
    /// 清除指定 AI 的收件窗口，下一个触发钩子的窗口接收（由用户操作）
    Rebind {
        #[command(flatten)]
        project: Project,
        #[arg(long, value_parser = ["claude", "codex"])]
        agent: String,
    },
}
