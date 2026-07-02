use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs::File;

fn detect_march() -> String {
    "-march=native".into()
}

fn detect_optimal_jobs() -> String {
    let nproc = std::thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(4);
    format!("-j{}", nproc)
}

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    current_cflags: &str,
    current_makeopts: &str,
    current_rustflags: &str,
) -> Result<(String, String, String)> {
    let suggested_cflags = format!("{} -O2 -pipe", detect_march());
    let cflags = widgets::input::run(
        Some(term),
        "CFLAGS".into(),
        format!("Compiler flags.\nSuggested: {}", suggested_cflags),
        Some(current_cflags.to_string()),
        None,
        None,
    )?;
    let cflags = if cflags.cancelled {
        current_cflags.to_string()
    } else {
        cflags
            .result
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    };

    let suggested_makeopts = detect_optimal_jobs();
    let makeopts = widgets::input::run(
        Some(term),
        "MAKEOPTS".into(),
        format!("Parallel jobs.\nSuggested: {}", suggested_makeopts),
        Some(current_makeopts.to_string()),
        None,
        None,
    )?;
    let makeopts = if makeopts.cancelled {
        current_makeopts.to_string()
    } else {
        makeopts
            .result
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    };

    let suggested_rustflags = "-C target-cpu=native".to_string();
    let rustflags = widgets::input::run(
        Some(term),
        "RUSTFLAGS".into(),
        format!("Rust compiler flags.\nSuggested: {}", suggested_rustflags),
        Some(current_rustflags.to_string()),
        None,
        None,
    )?;
    let rustflags = if rustflags.cancelled {
        current_rustflags.to_string()
    } else {
        rustflags
            .result
            .and_then(|v| v.as_str().map(String::from))
            .unwrap_or_default()
    };

    Ok((cflags, makeopts, rustflags))
}

pub fn run_per_package(term: &mut Terminal<CrosstermBackend<File>>) -> Result<bool> {
    let resp = widgets::yesno::run(
        Some(term),
        "Per-Package CFLAGS".into(),
        "Set custom compiler flags for specific packages?".into(),
        Some(false),
    )?;
    if !resp.cancelled && resp.result.and_then(|v| v.as_bool()).unwrap_or(false) {
        let edit = widgets::text::run(
            Some(term),
            "Package env".into(),
            Some("/mnt/etc/portage/env/custom-cflags".into()),
            None,
        )?;
        return Ok(!edit.cancelled);
    }
    Ok(false)
}