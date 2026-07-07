use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;

pub fn run(term: &mut Terminal<CrosstermBackend<File>>, current_value: &str) -> Result<String> {
    let mut dummy_values = HashMap::new();
    pick_inner(term, current_value, &mut dummy_values)
}

pub fn run_with_values(term: &mut Terminal<CrosstermBackend<File>>, current_value: &str, values: &mut HashMap<String, String>) -> Result<String> {
    pick_inner(term, current_value, values)
}

fn pick_inner(
    term: &mut Terminal<CrosstermBackend<File>>,
    current_value: &str,
    values: &mut HashMap<String, String>,
) -> Result<String> {
    let category = widgets::menu::run(
        Some(term),
        "Kernel".into(),
        "Select kernel:".into(),
        Value::Array(vec![
            Value::String("linux-* (standard)".into()),
            Value::String("linux-cachyos-*".into()),
            Value::String("linux-bazzite-bin".into()),
            Value::String("xanmod".into()),
            Value::String("tkg".into()),
            Value::String("linux-libre".into()),
        ]),
        None,
        None,
    )?;

    if category.cancelled {
        return Ok(current_value.to_string());
    }

    let choice = category
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    match choice.as_str() {
        "linux-* (standard)" => pick_standard(term, current_value),
        "linux-cachyos-*" => pick_cachyos(term, current_value),
        "tkg" => {
            crate::artixforge::install::tkg::run(term, values)?;
            Ok("tkg".to_string())
        }
        other => Ok(other.to_string()),
    }
}

fn pick_standard(term: &mut Terminal<CrosstermBackend<File>>, current: &str) -> Result<String> {
    let resp = widgets::menu::run(
        Some(term),
        "Standard Kernel".into(),
        "Select variant:".into(),
        Value::Array(vec![
            Value::String("linux".into()),
            Value::String("linux-zen".into()),
            Value::String("linux-lts".into()),
            Value::String("linux-hardened".into()),
        ]),
        None,
        None,
    )?;
    if resp.cancelled {
        Ok(current.to_string())
    } else {
        Ok(resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default())
    }
}

fn pick_cachyos(term: &mut Terminal<CrosstermBackend<File>>, current: &str) -> Result<String> {
    let resp = widgets::menu::run(
        Some(term),
        "CachyOS Kernel".into(),
        "Select variant:".into(),
        Value::Array(vec![
            Value::String("linux-cachyos (EEVDF)".into()),
            Value::String("linux-cachyos-bore (BORE)".into()),
            Value::String("linux-cachyos-eevdf".into()),
            Value::String("linux-cachyos-bmq (BMQ)".into()),
            Value::String("linux-cachyos-rt-bore (RT + BORE)".into()),
            Value::String("linux-cachyos-hardened (BORE + hardening)".into()),
            Value::String("linux-cachyos-lts (EEVDF, long-term)".into()),
            Value::String("linux-cachyos-server (EEVDF, server)".into()),
            Value::String("linux-cachyos-deckify (BORE, Steam Deck)".into()),
        ]),
        None,
        None,
    )?;
    if resp.cancelled {
        Ok(current.to_string())
    } else {
        let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
        Ok(val.split(' ').next().unwrap_or(&val).to_string())
    }
}