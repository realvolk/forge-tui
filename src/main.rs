use std::io::{self, BufRead, Write};
use std::fs;
use std::os::fd::{AsRawFd, FromRawFd, OwnedFd};
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

    let original_stdout = unsafe {
        let fd = libc::dup(1);
        if fd < 0 {
            eprintln!("{}", serde_json::to_string(&contract::Response {
                result: None,
                cancelled: true,
                error: Some("Failed to dup stdout".into()),
            })?);
            process::exit(1);
        }
        OwnedFd::from_raw_fd(fd)
    };

    let tty = fs::OpenOptions::new().read(true).write(true).open("/dev/tty")?;
    let tty_fd = tty.as_raw_fd();
    if unsafe { libc::dup2(tty_fd, 1) } < 0 {
        eprintln!("{}", serde_json::to_string(&contract::Response {
            result: None,
            cancelled: true,
            error: Some("Failed to redirect stdout to /dev/tty".into()),
        })?);
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
        write_json_response(&contract::Response {
            result: None,
            cancelled: true,
            error: Some("Empty input".into()),
        }, &original_stdout);
        process::exit(1);
    }

    let request: contract::Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            write_json_response(&contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("Invalid request JSON: {}", e)),
            }, &original_stdout);
            process::exit(1);
        }
    };

    match widgets::dispatch(request) {
        Ok(response) => {
            write_json_response(&response, &original_stdout);
            process::exit(if response.cancelled { 1 } else { 0 });
        }
        Err(e) => {
            write_json_response(&contract::Response {
                result: None,
                cancelled: true,
                error: Some(format!("{}", e)),
            }, &original_stdout);
            process::exit(1);
        }
    }
}

fn write_json_response(response: &contract::Response, original_stdout: &OwnedFd) {
    let json = serde_json::to_string(response).unwrap_or_default();
    let old_fd = original_stdout.as_raw_fd();
    unsafe {
        // Temporarily restore the original stdout to write JSON
        libc::dup2(old_fd, 1);
        let _ = io::stdout().write_all(json.as_bytes());
        let _ = io::stdout().write_all(b"\n");
        let _ = io::stdout().flush();
    }
}