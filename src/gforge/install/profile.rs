use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::fs::File;

fn get_profiles() -> Vec<String> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg("chroot /mnt eselect profile list 2>/dev/null || eselect profile list 2>/dev/null")
        .output();

    if let Ok(out) = output {
        let s = String::from_utf8_lossy(&out.stdout);
        s.lines()
            .filter_map(|line| {
                let line = line.trim();
                if line.is_empty() { return None; }
                let path = line
                    .split(']')
                    .nth(1)
                    .unwrap_or("")
                    .split('(')
                    .next()
                    .unwrap_or("")
                    .trim();
                if path.is_empty() { None } else { Some(path.to_string()) }
            })
            .collect()
    } else {
        vec!["default/linux/amd64/17.1".into()]
    }
}

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    current: &str,
) -> Result<Option<String>> {
    let profiles = get_profiles();

    let resp = widgets::menu::run(
        Some(term),
        "Portage Profile".into(),
        "Select system profile:".into(),
        Value::Array(profiles.iter().map(|s| Value::String(s.clone())).collect()),
        Some(current.to_string()),
        None,
    )?;

    if resp.cancelled { return Ok(None); }
    Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
}

pub fn run_inheritance(
    term: &mut Terminal<CrosstermBackend<File>>,
    profile_path: &str,
) -> Result<()> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(format!(
            "current='{}'; while [[ -n \"$current\" ]]; do echo \"$current\"; \
             parent_file=\"/var/db/repos/gentoo/profiles/$current/parent\"; \
             [[ -f \"$parent_file\" ]] && current=$(head -n1 \"$parent_file\") || current=''; done",
            profile_path
        ))
        .output();

    let chain = if let Ok(out) = output {
        String::from_utf8_lossy(&out.stdout).to_string()
    } else {
        "Could not determine inheritance chain.".to_string()
    };

    widgets::msg::run(Some(term), "Profile Inheritance".into(), chain)?;
    Ok(())
}