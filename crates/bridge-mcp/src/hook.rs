use anyhow::Result;
use bridge_core::{hook::Input, Bridge};
use std::io::{Read, Write};

pub fn run(bridge: &Bridge, agent: &str) {
    if let Err(error) = deliver(bridge, agent) {
        // 钩子失败只报告到 stderr；绝不阻止宿主结束当前轮次。
        eprintln!("Bridge 钩子：{error}");
    }
}
fn deliver(bridge: &Bridge, agent: &str) -> Result<()> {
    let mut bytes = Vec::new();
    std::io::stdin().lock().read_to_end(&mut bytes)?;
    let input: Input = serde_json::from_slice(&bytes)
        .map_err(|_| anyhow::anyhow!("标准输入必须是包含 cwd、session_id 的有效 JSON"))?;
    bridge.deliver_hook(&input, agent, |reply| {
        let mut stdout = std::io::stdout().lock();
        serde_json::to_writer(&mut stdout, reply)?;
        stdout.write_all(b"\n")?;
        stdout.flush()?;
        Ok(())
    })
}
