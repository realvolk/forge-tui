use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::fs::File;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    current_value: &str,
) -> Result<String> {
    let resp = widgets::menu::run(
        Some(term),
        "Kernel".into(),
        "Select kernel:".into(),
        Value::Array(
            vec![
                "gentoo-kernel",
                "gentoo-kernel-bin",
                "gentoo-sources",
                "gentoo-sources-genkernel",
            ]
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect(),
        ),
        Some(current_value.to_string()),
        None,
    )?;
    if resp.cancelled {
        return Ok(current_value.to_string());
    }
    let kernel = resp
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    if kernel.contains("sources") {
        let method = if kernel.contains("genkernel") {
            "genkernel".to_string()
        } else {
            let m = widgets::menu::run(
                Some(term),
                "Config Method".into(),
                "Choose configuration method:".into(),
                Value::Array(
                    vec!["genkernel", "manual"]
                        .iter()
                        .map(|s| Value::String(s.to_string()))
                        .collect(),
                ),
                Some("genkernel".into()),
                None,
            )?;
            if m.cancelled {
                return Ok(current_value.to_string());
            }
            m.result
                .and_then(|v| v.as_str().map(String::from))
                .unwrap_or("genkernel".into())
        };

        if method == "manual" {
            let defconfigs = vec![
                "none", "intel-laptop", "amd-desktop", "qemu-kvm",
                "virtualbox", "nvme-minimal",
            ];
            let _def = widgets::menu::run(
                Some(term),
                "Defconfig".into(),
                "Start from a pre-built minimal config?".into(),
                Value::Array(
                    defconfigs
                        .iter()
                        .map(|s| Value::String(s.to_string()))
                        .collect(),
                ),
                Some("none".into()),
                None,
            )?;
        }

        let _dracut = widgets::yesno::run(
            Some(term),
            "Dracut".into(),
            "Use dracut instead of genkernel for initramfs?".into(),
            Some(false),
        )?;

        let _installkernel = widgets::yesno::run(
            Some(term),
            "installkernel".into(),
            "Use sys-kernel/installkernel to automate kernel installation?".into(),
            Some(true),
        )?;
    }

    let _microcode = widgets::menu::run(
        Some(term),
        "Microcode".into(),
        "Select CPU microcode:".into(),
        Value::Array(
            vec![
                "none",
                "sys-firmware/intel-microcode",
                "sys-firmware/amd-microcode",
            ]
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect(),
        ),
        Some("none".into()),
        None,
    )?;

    Ok(kernel)
}

pub fn run_config_diff(term: &mut Terminal<CrosstermBackend<File>>) -> Result<()> {
    let output = std::process::Command::new("sh")
        .arg("-c")
        .arg(
            "default=''; \
             for f in /usr/src/linux/arch/x86/configs/x86_64_defconfig /usr/src/linux/arch/x86/configs/generic-64_defconfig; do \
               [[ -f \"$f\" ]] && { default=\"$f\"; break; }; \
             done; \
             if [[ -n \"$default\" && -f /usr/src/linux/.config ]]; then \
               diff -u \"$default\" /usr/src/linux/.config 2>/dev/null | head -200; \
             else \
               echo 'No config to compare.'; \
             fi",
        )
        .output();

    let diff = if let Ok(out) = output {
        String::from_utf8_lossy(&out.stdout).to_string()
    } else {
        "Could not generate diff.".to_string()
    };

    widgets::msg::run(Some(term), "Kernel Config Diff".into(), diff)?;
    Ok(())
}