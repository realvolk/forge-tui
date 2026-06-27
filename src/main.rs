use std::io::{self, BufRead, Write};
use std::fs;
use std::process;
use anyhow::Result;

mod contract;
mod layout;
mod theme;
mod watermark;
mod widgets;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut mode: Option<String> = None;
    let mut input_file: Option<String> = None;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => {
                i += 1;
                if i < args.len() {
                    mode = Some(args[i].clone());
                }
            }
            "--input" | "-i" => {
                i += 1;
                if i < args.len() {
                    input_file = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    if mode.as_deref() != Some("widget") {
        eprintln!("Usage: forge-tui --mode widget [--input <file>]");
        process::exit(1);
    }

    let input = if let Some(ref path) = input_file {
        fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read input file '{}': {}", path, e))?
            .trim()
            .to_string()
    } else {
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        input.trim().to_string()
    };

    if input.is_empty() {
        eprintln!("{}", serde_json::to_string(&contract::Response {
            result: None,
            cancelled: true,
            error: Some("Empty input".into()),
        })?);
        process::exit(1);
    }

    let request: contract::Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{}", serde_json::to_string(&contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("Invalid request JSON: {}", e)),
            })?);
            process::exit(1);
        }
    };

    match widgets::dispatch(request) {
        Ok(response) => {
            // ONLY JSON to stdout. Nothing else.
            let json = serde_json::to_string(&response)?;
            let mut stdout = io::stdout();
            stdout.write_all(json.as_bytes())?;
            stdout.write_all(b"\n")?;
            stdout.flush()?;
            process::exit(if response.cancelled { 1 } else { 0 });
        }
        Err(e) => {
            eprintln!("{}", serde_json::to_string(&contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("{}", e)),
            })?);
            process::exit(1);
        }
    }
}