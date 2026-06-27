use std::io::{self, BufRead, Write};
use std::fs;
use std::process;
use anyhow::Result;

mod contract;
mod layout;
mod theme;
mod tty;
mod watermark;
mod widgets;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut mode: Option<String> = None;
    let mut input_file: Option<String> = None;
    let mut output_file: Option<String> = None;

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
            "--output" | "-o" => {
                i += 1;
                if i < args.len() {
                    output_file = Some(args[i].clone());
                }
            }
            _ => {}
        }
        i += 1;
    }

    if mode.as_deref() != Some("widget") {
        eprintln!("Usage: forge-tui --mode widget [--input <file>] [--output <file>]");
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
        let resp = contract::Response {
            result: None,
            cancelled: true,
            error: Some("Empty input".into()),
        };
        write_response(&resp, &output_file);
        process::exit(1);
    }

    let request: contract::Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            let resp = contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("Invalid request JSON: {}", e)),
            };
            write_response(&resp, &output_file);
            process::exit(1);
        }
    };

    match widgets::dispatch(request) {
        Ok(response) => {
            write_response(&response, &output_file);
            process::exit(if response.cancelled { 1 } else { 0 });
        }
        Err(e) => {
            let resp = contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("{}", e)),
            };
            write_response(&resp, &output_file);
            process::exit(1);
        }
    }
}

fn write_response(response: &contract::Response, output_file: &Option<String>) {
    let json = serde_json::to_string(response).unwrap_or_default();
    if let Some(ref path) = output_file {
        let _ = fs::write(path, json + "\n");
    } else {
        println!("{}", json);
    }
}