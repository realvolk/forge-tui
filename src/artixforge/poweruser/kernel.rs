use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
    values: &HashMap<String, String>,
) -> Result<String> {
    // Kernel config depth
    let depth = widgets::menu::run(Some(term), "Kernel Configuration".into(),
        "How much control?".into(),
        Value::Array(vec!["localmodconfig","auto","manual","menuconfig"].iter().map(|s| Value::String(s.to_string())).collect()),
        Some(values.get("KERNEL_CONFIG_DEPTH").cloned().unwrap_or("auto".into())), None)?;

    let depth_val = if depth.cancelled { "auto".to_string() }
        else { depth.result.and_then(|v| v.as_str().map(String::from)).unwrap_or("auto".into()) };

    // Hardware categories (simplified — real version has GPU/Network/FS/Sound/USB/Security/Virt/Debug)
    let gpu = widgets::checklist::run(Some(term), "GPU Drivers".into(), "Select:".into(),
        vec!["intel".into(),"amd".into(),"nvidia".into(),"virtio".into(),"vesa".into(),"simpledrm".into()],
        None, None, None, None)?;
    let gpu_val = if gpu.cancelled { String::new() } else {
        gpu.result.and_then(|v| v.as_array().cloned())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default()
    };

    let net = widgets::checklist::run(Some(term), "Network".into(), "Select:".into(),
        vec!["intel".into(),"realtek".into(),"broadcom".into(),"atheros".into(),"virtio".into(),"intel-wifi".into(),"ath-wifi".into(),"realtek-wifi".into(),"bt".into()],
        None, None, None, None)?;
    let net_val = if net.cancelled { String::new() } else {
        net.result.and_then(|v| v.as_array().cloned())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default()
    };

    let fs = widgets::checklist::run(Some(term), "Filesystems".into(), "Select:".into(),
        vec!["ext4".into(),"btrfs".into(),"xfs".into(),"f2fs".into(),"exfat".into(),"ntfs3".into(),"overlay".into()],
        None, None, None, None)?;
    let fs_val = if fs.cancelled { String::new() } else {
        fs.result.and_then(|v| v.as_array().cloned())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default()
    };

    let snd = widgets::checklist::run(Some(term), "Sound".into(), "Select:".into(),
        vec!["intel-hda".into(),"amd-hda".into(),"usb-audio".into()],
        None, None, None, None)?;
    let snd_val = if snd.cancelled { String::new() } else {
        snd.result.and_then(|v| v.as_array().cloned())
            .map(|a| a.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" ")).unwrap_or_default()
    };

    let preempt = widgets::menu::run(Some(term), "Preemption".into(), "Model:".into(),
        Value::Array(vec!["voluntary".into(),"full".into(),"rt".into()]),
        Some(values.get("KERNEL_PREEMPT").cloned().unwrap_or("voluntary".into())), None)?;
    let preempt_val = if preempt.cancelled { "voluntary".into() }
        else { preempt.result.and_then(|v| v.as_str().map(String::from)).unwrap_or("voluntary".into()) };

    let timer = widgets::menu::run(Some(term), "Timer Frequency".into(), "Hz:".into(),
        Value::Array(vec!["100".into(),"250".into(),"300".into(),"1000".into()]),
        Some(values.get("KERNEL_TIMER").cloned().unwrap_or("1000".into())), None)?;
    let timer_val = if timer.cancelled { "1000".into() }
        else { timer.result.and_then(|v| v.as_str().map(String::from)).unwrap_or("1000".into()) };

    let gov = widgets::menu::run(Some(term), "CPU Governor".into(), "Default:".into(),
        Value::Array(vec!["schedutil".into(),"ondemand".into(),"performance".into()]),
        Some(values.get("KERNEL_GOVERNOR").cloned().unwrap_or("schedutil".into())), None)?;
    let gov_val = if gov.cancelled { "schedutil".into() }
        else { gov.result.and_then(|v| v.as_str().map(String::from)).unwrap_or("schedutil".into()) };

    Ok(format!(
        "depth={} gpu={} net={} fs={} snd={} preempt={} timer={} gov={}",
        depth_val, gpu_val, net_val, fs_val, snd_val, preempt_val, timer_val, gov_val
    ))
}