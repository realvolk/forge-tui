use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use std::collections::HashMap;
use std::fs::File;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    values: &HashMap<String, String>,
    boot_mode: &str,
) -> Result<()> {
    let mut warnings = Vec::new();

    if boot_mode == "bios" {
        warnings.push("BIOS boot mode -- UEFI-only features disabled");
    }
    if values.get("KERNEL_CHOICE").map_or(false, |k| k.contains("source"))
        && values.get("KERNEL_CONFIG_METHOD").map_or(false, |m| m == "manual")
    {
        warnings.push("Manual kernel config -- ensure essential drivers");
    }
    if values.get("USE_BINHOST").map_or(false, |v| v == "yes") {
        warnings.push("Binhost enabled -- prebuilt binaries will be used");
    }
    if let Some(use_flags) = values.get("GLOBAL_USE") {
        if use_flags.contains("lto") {
            warnings.push("USE=lto may break packages");
        }
        if use_flags.contains("systemd") && values.get("INIT").map_or(false, |i| i == "openrc") {
            warnings.push("USE=systemd with OpenRC may cause issues");
        }
    }

    if !warnings.is_empty() {
        let msg = warnings.iter().map(|w| format!(" - {}", w)).collect::<Vec<_>>().join("\n");
        widgets::msg::run(Some(term), "Sanity Warnings".into(), msg)?;
    }
    Ok(())
}