use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::fs::File;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    current: &str,
) -> Result<Option<String>> {
    let variants = vec![
        "openrc",
        "desktop-openrc",
        "systemd",
        "desktop-systemd",
        "hardened-openrc",
        "musl-openrc",
        "selinux-openrc",
    ];
    let resp = widgets::menu::run(
        Some(term),
        "Stage3 Variant".into(),
        "Select stage3 tarball:".into(),
        Value::Array(variants.iter().map(|s| Value::String(s.to_string())).collect()),
        Some(current.to_string()),
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
}