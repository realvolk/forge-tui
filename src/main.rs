use std::io::{self, BufRead, Read, Write};
use std::fs;
use std::os::unix::net::UnixListener;
use std::process;
use anyhow::Result;

mod contract;
mod daemon;
mod layout;
mod theme;
mod tty;
mod watermark;
mod widgets;
mod artixforge;
mod gforge;

fn main() -> Result<()> {
    let args: Vec<String> = std::env::args().collect();

    let mut mode: Option<String> = None;
    let mut input_file: Option<String> = None;
    let mut output_file: Option<String> = None;
    let mut socket_path: Option<String> = None;
    let mut daemon_mode = false;
    let mut send_mode = false;
    let mut send_json: Option<String> = None;
    let mut batch_mode = false;

    let mut i = 1;
    while i < args.len() {
        match args[i].as_str() {
            "--mode" => { i += 1; if i < args.len() { mode = Some(args[i].clone()); } }
            "--input" | "-i" => { i += 1; if i < args.len() { input_file = Some(args[i].clone()); } }
            "--output" | "-o" => { i += 1; if i < args.len() { output_file = Some(args[i].clone()); } }
            "--daemon" => { daemon_mode = true; }
            "--send" => { send_mode = true; }
            "--socket" => { i += 1; if i < args.len() { socket_path = Some(args[i].clone()); } }
            "--batch" => { batch_mode = true; }
            arg if send_mode && send_json.is_none() => { send_json = Some(arg.to_string()); }
            _ => {}
        }
        i += 1;
    }

    if send_mode {
        let socket = socket_path.unwrap_or_else(|| "/tmp/forge-tui.sock".to_string());
        let json = send_json.unwrap_or_default();
        if json.is_empty() {
            eprintln!("Usage: forge-tui --send '<json>' [--socket <path>]");
            process::exit(1);
        }
        return send_to_daemon(&socket, &json);
    }

    if daemon_mode {
        let socket = socket_path.unwrap_or_else(|| "/tmp/forge-tui.sock".to_string());
        let _ = fs::remove_file(&socket);
        let listener = UnixListener::bind(&socket)?;
        return daemon::run(listener);
    }

    if batch_mode {
        let path = input_file.unwrap_or_else(|| "/dev/stdin".to_string());
        let content = if path == "/dev/stdin" {
            let mut input = String::new();
            io::stdin().read_to_string(&mut input)?;
            input
        } else {
            fs::read_to_string(&path)?
        };

        let lines: Vec<&str> = content.lines().collect();
        let mut stdout = io::stdout();

        for line in lines {
            let line = line.trim();
            if line.is_empty() { continue; }

            let request: contract::Request = match serde_json::from_str(line) {
                Ok(r) => r,
                Err(e) => {
                    let resp = contract::Response {
                        result: None,
                        cancelled: true,
                        error: Some(format!("Invalid request JSON: {}", e)),
                    };
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                    continue;
                }
            };

            match widgets::dispatch(request, None) {
                Ok(response) => {
                    writeln!(stdout, "{}", serde_json::to_string(&response)?)?;
                    stdout.flush()?;
                }
                Err(e) => {
                    let resp = contract::Response {
                        result: None,
                        cancelled: true,
                        error: Some(format!("{}", e)),
                    };
                    writeln!(stdout, "{}", serde_json::to_string(&resp)?)?;
                    stdout.flush()?;
                }
            }
        }
        return Ok(());
    }

    if mode.as_deref() != Some("widget") {
        eprintln!("Usage: forge-tui --mode widget [--input <file>] [--output <file>]");
        eprintln!("       forge-tui --daemon [--socket <path>]");
        eprintln!("       forge-tui --send '<json>' [--socket <path>]");
        eprintln!("       forge-tui --batch [--input <file>]");
        process::exit(1);
    }

    let input = if let Some(ref path) = input_file {
        fs::read_to_string(path)
            .map_err(|e| anyhow::anyhow!("Failed to read input file '{}': {}", path, e))?
            .trim().to_string()
    } else {
        let mut input = String::new();
        io::stdin().lock().read_line(&mut input)?;
        input.trim().to_string()
    };

    if input.is_empty() {
        write_response(&contract::Response { result: None, cancelled: true, error: Some("Empty input".into()) }, &output_file);
        process::exit(1);
    }

    let request: contract::Request = match serde_json::from_str(&input) {
        Ok(r) => r,
        Err(e) => {
            write_response(&contract::Response { result: None, cancelled: true, error: Some(format!("Invalid request JSON: {}", e)) }, &output_file);
            process::exit(1);
        }
    };

    match widgets::dispatch(request, None) {
        Ok(response) => { write_response(&response, &output_file); process::exit(if response.cancelled { 1 } else { 0 }); }
        Err(e) => {
            write_response(&contract::Response { result: None, cancelled: true, error: Some(format!("{}", e)) }, &output_file);
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

fn send_to_daemon(socket_path: &str, json: &str) -> Result<()> {
    use std::io::{BufRead, BufReader, Write};
    use std::os::unix::net::UnixStream;
    let mut stream = UnixStream::connect(socket_path)?;
    stream.write_all(json.as_bytes())?;
    stream.write_all(b"\n")?;
    stream.flush()?;
    let mut reader = BufReader::new(stream);
    let mut response = String::new();
    reader.read_line(&mut response)?;
    print!("{}", response);
    Ok(())
}