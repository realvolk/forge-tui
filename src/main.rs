use std::io::{self, BufRead};
use std::process;
use anyhow::Result;

mod contract;
mod layout;
mod theme;
mod watermark;
mod widgets;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();
    if args.len() != 3 || args[1] != "--mode" || args[2] != "widget" {
        eprintln!("Usage: forge-tui --mode widget");
        process::exit(1);
    }

    let stdin = io::stdin();
    let mut input = String::new();
    stdin.lock().read_line(&mut input)?;
    let input = input.trim().to_string();

    if input.is_empty() {
        let resp = contract::Response {
            result: None,
            cancelled: true,
            error: Some("Empty input".into()),
        };
        println!("{}", serde_json::to_string(&resp)?);
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
            println!("{}", serde_json::to_string(&resp)?);
            process::exit(1);
        }
    };

    match widgets::dispatch(request) {
        Ok(response) => {
            println!("{}", serde_json::to_string(&response)?);
            process::exit(if response.cancelled { 1 } else { 0 });
        }
        Err(e) => {
            let resp = contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("{}", e)),
            };
            println!("{}", serde_json::to_string(&resp)?);
            process::exit(1);
        }
    }
}