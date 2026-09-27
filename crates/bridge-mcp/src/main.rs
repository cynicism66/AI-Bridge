mod cli;

use anyhow::Result;
use bridge_core::{protocol, Bridge};
use std::io::{self, BufRead, Write};

fn serve(bridge: &Bridge) -> Result<()> {
    let input = io::stdin();
    let output = io::stdout();
    let mut output = output.lock();
    for line in input.lock().split(b'\n') {
        let line = line?;
        if line.iter().all(u8::is_ascii_whitespace) {
            continue;
        }
        let reply = match serde_json::from_slice(&line) {
            Ok(request) => protocol::response(bridge, &request),
            Err(_) => Some(protocol::parse_error()),
        };
        if let Some(reply) = reply {
            serde_json::to_writer(&mut output, &reply)?;
            output.write_all(b"\n")?;
            output.flush()?;
        }
    }
    Ok(())
}

fn run() -> Result<()> {
    let bridge = Bridge::from_env()?;
    if let Some(text) = cli::run(&bridge)? {
        let mut output = io::stdout().lock();
        output.write_all(text.as_bytes())?;
        output.write_all(b"\n")?;
        output.flush()?;
        Ok(())
    } else {
        serve(&bridge)
    }
}

fn main() {
    if let Err(error) = run() {
        if error
            .downcast_ref::<io::Error>()
            .is_some_and(|e| e.kind() == io::ErrorKind::BrokenPipe)
        {
            return;
        }
        eprintln!("出错：{error}");
        std::process::exit(2);
    }
}
