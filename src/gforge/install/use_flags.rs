use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::fs::File;
use std::process::Command;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    _current_value: &str,
) -> Result<Option<String>> {
    let output = Command::new("sh")
        .arg("-c")
        .arg("cat /usr/portage/profiles/use.desc 2>/dev/null | grep '^[a-z]' | awk '{print $1 \" - \" $2}' | sort -u || true")
        .output()
        .ok();
    let choices: Vec<String> = if let Some(out) = output {
        String::from_utf8_lossy(&out.stdout)
            .lines()
            .map(|l| l.trim().to_string())
            .filter(|l| !l.is_empty())
            .collect()
    } else {
        vec!["X".into(), "wayland".into(), "pulseaudio".into()]
    };

    if choices.is_empty() {
        return Ok(None);
    }

    let resp = widgets::multiselect::run(
        Some(term),
        "USE Flags".into(),
        "Space to toggle, Enter to confirm.".into(),
        choices,
        Some("Search flags...".into()),
        None,
        None,
    )?;

    if resp.cancelled {
        return Ok(None);
    }

    let selected = resp
        .result
        .and_then(|v| v.as_array().cloned())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str())
                .map(|s| s.split(" - ").next().unwrap_or(s))
                .collect::<Vec<_>>()
                .join(" ")
        })
        .unwrap_or_default();

    Ok(Some(selected))
}