use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    values: &mut HashMap<String, String>,
) -> Result<()> {
    let current_depth = values.get("KERNEL_CONFIG_DEPTH").cloned().unwrap_or_else(|| "auto".into());
    let current_gpu = values.get("KERNEL_ADV_GPU").cloned().unwrap_or_default();
    let current_net = values.get("KERNEL_ADV_NET").cloned().unwrap_or_default();
    let current_fs = values.get("KERNEL_ADV_FS").cloned().unwrap_or_else(|| "ext4".into());
    let current_snd = values.get("KERNEL_ADV_SOUND").cloned().unwrap_or_default();
    let current_usb = values.get("KERNEL_ADV_USB").cloned().unwrap_or_default();
    let current_sec = values.get("KERNEL_ADV_SECURITY").cloned().unwrap_or_default();
    let current_virt = values.get("KERNEL_ADV_VIRT").cloned().unwrap_or_default();
    let current_dbg = values.get("KERNEL_ADV_DEBUG").cloned().unwrap_or_default();
    let current_preempt = values.get("KERNEL_PREEMPT").cloned().unwrap_or_else(|| "voluntary".into());
    let current_timer = values.get("KERNEL_TIMER").cloned().unwrap_or_else(|| "250".into());
    let current_gov = values.get("KERNEL_GOVERNOR").cloned().unwrap_or_else(|| "schedutil".into());

    loop {
        let category = widgets::menu::run(
            Some(term),
            "Kernel Configuration".into(),
            "Select category:".into(),
            Value::Array(vec![
                "Configuration Depth".into(),
                "GPU Drivers".into(),
                "Network".into(),
                "Filesystems".into(),
                "Sound".into(),
                "USB".into(),
                "Security".into(),
                "Virtualization".into(),
                "Debug".into(),
                "Preemption Model".into(),
                "Timer Frequency".into(),
                "CPU Governor".into(),
                "Done".into(),
            ]),
            None,
            None,
        )?;

        if category.cancelled {
            return Ok(());
        }

        let choice = category.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();

        match choice.as_str() {
            "Configuration Depth" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Configuration Depth".into(),
                    "How much control?".into(),
                    Value::Array(vec![
                        "localmodconfig".into(),
                        "auto".into(),
                        "manual".into(),
                        "menuconfig".into(),
                    ]),
                    Some(current_depth.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or(current_depth.clone());
                    if new_val == "menuconfig" {
                        let config_path = "/mnt/artix-poweruser/work/linux-custom/src/.config";
                        if std::path::Path::new(config_path).exists() {
                            crate::artixforge::poweruser::kconfig::run(term, config_path)?;
                        }
                    }
                    values.insert("KERNEL_CONFIG_DEPTH".into(), new_val);
                }
            }
            "GPU Drivers" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "GPU Drivers".into(),
                    "Select:".into(),
                    vec!["intel".into(), "amd".into(), "nvidia".into(), "virtio".into(), "vesa".into(), "simpledrm".into()],
                    None, None, None,
                    Some(current_gpu.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_GPU".into(), new_val);
                }
            }
            "Network" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "Network".into(),
                    "Select:".into(),
                    vec!["intel".into(), "realtek".into(), "broadcom".into(), "atheros".into(), "virtio".into(),
                         "intel-wifi".into(), "ath-wifi".into(), "realtek-wifi".into(), "bt".into()],
                    None, None, None,
                    Some(current_net.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_NET".into(), new_val);
                }
            }
            "Filesystems" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "Filesystems".into(),
                    "Select:".into(),
                    vec!["ext4".into(), "btrfs".into(), "xfs".into(), "f2fs".into(), "exfat".into(), "ntfs3".into(), "overlay".into()],
                    None, None, None,
                    Some(current_fs.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_FS".into(), new_val);
                }
            }
            "Sound" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "Sound".into(),
                    "Select:".into(),
                    vec!["intel-hda".into(), "amd-hda".into(), "usb-audio".into()],
                    None, None, None,
                    Some(current_snd.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_SOUND".into(), new_val);
                }
            }
            "USB" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "USB".into(),
                    "Select:".into(),
                    vec!["storage".into(), "hid".into(), "serial".into()],
                    None, None, None,
                    Some(current_usb.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_USB".into(), new_val);
                }
            }
            "Security" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "Security".into(),
                    "Select:".into(),
                    vec!["selinux".into(), "apparmor".into(), "lockdown".into()],
                    None, None, None,
                    Some(current_sec.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_SECURITY".into(), new_val);
                }
            }
            "Virtualization" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "Virtualization".into(),
                    "Select:".into(),
                    vec!["kvm".into(), "vhost".into()],
                    None, None, None,
                    Some(current_virt.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_VIRT".into(), new_val);
                }
            }
            "Debug" => {
                let resp = widgets::checklist::run(
                    Some(term),
                    "Debug".into(),
                    "Select:".into(),
                    vec!["ftrace".into(), "perf".into(), "kprobes".into(), "ebpf".into()],
                    None, None, None,
                    Some(current_dbg.split_whitespace().map(|s| s.to_string()).collect()),
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
                        .unwrap_or_default();
                    values.insert("KERNEL_ADV_DEBUG".into(), new_val);
                }
            }
            "Preemption Model" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Preemption Model".into(),
                    "Select:".into(),
                    Value::Array(vec!["voluntary".into(), "full".into(), "rt".into()]),
                    Some(current_preempt.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or(current_preempt.clone());
                    values.insert("KERNEL_PREEMPT".into(), new_val);
                }
            }
            "Timer Frequency" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Timer Frequency".into(),
                    "Select Hz:".into(),
                    Value::Array(vec!["100".into(), "250".into(), "300".into(), "1000".into()]),
                    Some(current_timer.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or(current_timer.clone());
                    values.insert("KERNEL_TIMER".into(), new_val);
                }
            }
            "CPU Governor" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "CPU Governor".into(),
                    "Select default:".into(),
                    Value::Array(vec!["schedutil".into(), "ondemand".into(), "performance".into()]),
                    Some(current_gov.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let new_val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or(current_gov.clone());
                    values.insert("KERNEL_GOVERNOR".into(), new_val);
                }
            }
            "Done" => return Ok(()),
            _ => {}
        }
    }
}