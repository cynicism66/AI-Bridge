use anyhow::{bail, Result};
use bridge_core::{
    paths,
    wait::{self, Poll, Reader},
    Bridge,
};
use clap::Args;
use std::{
    io::{self, Write},
    time::{Duration, Instant},
};

#[derive(Args)]
pub struct Options {
    /// 项目目录，默认当前目录
    project: Option<String>,
    /// 接收消息的 AI 名称
    #[arg(long)]
    agent: String,
    /// 等待秒数，省略时不超时
    #[arg(long)]
    timeout: Option<u64>,
    /// 只读轮询间隔（秒），最小为 1
    #[arg(long, default_value_t=3, value_parser=clap::value_parser!(u64).range(1..))]
    interval: u64,
    /// 持续打印新消息，不在首条后退出
    #[arg(long)]
    follow: bool,
}
pub fn run(bridge: &Bridge, options: Options) -> Result<i32> {
    let agent = options.agent.trim().to_lowercase();
    if agent.is_empty()
        || matches!(agent.as_str(), "all" | "bridge")
        || agent.chars().any(char::is_control)
    {
        bail!("--agent 必须是有效的接收方名称");
    }
    let project = paths::cli_project(options.project.as_deref().unwrap_or("."))?;
    let mut out = io::stdout().lock();
    if let Some(reason) = wait::disabled(bridge, &project, &agent)? {
        writeln!(out, "{reason}")?;
        out.flush()?;
        return Ok(3);
    }
    let mut reader = Reader::new(bridge, &project, &agent)?;
    let start = Instant::now();
    let timeout = options.timeout.map(Duration::from_secs);
    loop {
        if timeout.is_some_and(|limit| start.elapsed() >= limit) {
            writeln!(out, "等待超时")?;
            out.flush()?;
            return Ok(2);
        }
        match reader.poll()? {
            Poll::Disabled(reason) => {
                writeln!(out, "{reason}")?;
                out.flush()?;
                return Ok(3);
            }
            Poll::Messages(messages) => {
                for message in messages {
                    writeln!(out, "{}", wait::line(&message))?;
                    out.flush()?;
                    if !options.follow {
                        return Ok(0);
                    }
                }
            }
        }
        let interval = Duration::from_secs(options.interval);
        std::thread::sleep(
            timeout
                .map(|t| interval.min(t.saturating_sub(start.elapsed())))
                .unwrap_or(interval),
        );
    }
}
