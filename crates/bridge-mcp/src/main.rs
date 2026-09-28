mod cli;
mod wait;

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

fn run() -> Result<i32> {
    let bridge = Bridge::from_env()?;
    match cli::run(&bridge)? {
        cli::Output::Exit(code) => Ok(code),
        cli::Output::Serve => {
            serve(&bridge)?;
            Ok(0)
        }
        cli::Output::Text(text) => {
            let mut output = io::stdout().lock();
            output.write_all(text.as_bytes())?;
            output.write_all(b"\n")?;
            output.flush()?;
            Ok(0)
        }
    }
}

fn main() {
    match run() {
        Ok(code) => std::process::exit(code),
        Err(error) => {
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
}
