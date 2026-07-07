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
    let current_sched = values.get("TKG_SCHEDULER").cloned().unwrap_or_else(|| "eevdf".into());
    let current_binary = values.get("TKG_BINARY").cloned().unwrap_or_else(|| "no".into());
    let current_compiler = values.get("TKG_COMPILER").cloned().unwrap_or_else(|| "gcc".into());
    let current_optlevel = values.get("TKG_OPTLEVEL").cloned().unwrap_or_else(|| "1".into());
    let current_proc_opt = values.get("TKG_PROCESSOR_OPT").cloned().unwrap_or_else(|| "native".into());
    let current_lto = values.get("TKG_LTO_MODE").cloned().unwrap_or_else(|| "no".into());
    let current_preempt_rt = values.get("TKG_PREEMPT_RT").cloned().unwrap_or_else(|| "0".into());
    let current_tickless = values.get("TKG_TICKLESS").cloned().unwrap_or_else(|| "2".into());
    let current_timer = values.get("TKG_TIMER_FREQ").cloned().unwrap_or_else(|| "1000".into());
    let current_gov = values.get("TKG_CPU_GOV").cloned().unwrap_or_else(|| "ondemand".into());
    let current_glitched = values.get("TKG_GLITCHED_BASE").cloned().unwrap_or_else(|| "false".into());
    let current_zenify = values.get("TKG_ZENIFY").cloned().unwrap_or_else(|| "false".into());
    let current_clear = values.get("TKG_CLEAR_PATCHES").cloned().unwrap_or_else(|| "false".into());
    let current_openrgb = values.get("TKG_OPENRGB").cloned().unwrap_or_else(|| "false".into());
    let current_acs = values.get("TKG_ACS_OVERRIDE").cloned().unwrap_or_else(|| "false".into());
    let current_fsync = values.get("TKG_FSYNC").cloned().unwrap_or_else(|| "false".into());
    let current_mglru = values.get("TKG_MGLRU").cloned().unwrap_or_else(|| "false".into());
    let current_ntsync = values.get("TKG_NTSYNC").cloned().unwrap_or_else(|| "false".into());
    let current_nr_cpus = values.get("TKG_NR_CPUS").cloned().unwrap_or_else(|| num_cpus().to_string());

    loop {
        let category = widgets::menu::run(
            Some(term),
            "TKG Kernel Configuration".into(),
            "Select category:".into(),
            Value::Array(vec![
                "Scheduler".into(),
                "Build Type".into(),
                "Compiler & Optimization".into(),
                "Preemption & Tickless".into(),
                "Timer & Governor".into(),
                "Patches".into(),
                "CPU Count".into(),
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
            "Scheduler" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "CPU Scheduler".into(),
                    "Select scheduler:".into(),
                    Value::Array(vec![
                        "eevdf (default)".into(),
                        "bmq".into(),
                        "bore".into(),
                        "pds".into(),
                    ]),
                    Some(match current_sched.as_str() {
                        "bmq" => "bmq".into(),
                        "bore" => "bore".into(),
                        "pds" => "pds".into(),
                        _ => "eevdf (default)".into(),
                    }),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    let code = match val.as_str() {
                        "bmq" => "bmq",
                        "bore" => "bore",
                        "pds" => "pds",
                        _ => "eevdf",
                    };
                    values.insert("TKG_SCHEDULER".into(), code.to_string());
                }
            }
            "Build Type" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Build Type".into(),
                    "Binary or compile?".into(),
                    Value::Array(vec![
                        "Binary (download, ~50MB)".into(),
                        "Compile from source (~30 min)".into(),
                    ]),
                    Some(if current_binary == "yes" { "Binary (download, ~50MB)".into() } else { "Compile from source (~30 min)".into() }),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_BINARY".into(), if val.contains("Binary") { "yes".to_string() } else { "no".to_string() });
                }
            }
            "Compiler & Optimization" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Compiler".into(),
                    "Select compiler:".into(),
                    Value::Array(vec!["gcc".into(), "clang".into()]),
                    Some(current_compiler.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_COMPILER".into(), val);
                }

                let resp = widgets::menu::run(
                    Some(term),
                    "Optimization Level".into(),
                    "Select -O level:".into(),
                    Value::Array(vec!["1".into(), "2".into(), "3".into()]),
                    Some(current_optlevel.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_OPTLEVEL".into(), val);
                }

                let resp = widgets::menu::run(
                    Some(term),
                    "Processor Optimization".into(),
                    "Target CPU:".into(),
                    Value::Array(vec!["native".into(), "generic".into(), "core2".into(), "nehalem".into(), "haswell".into(), "skylake".into(), "znver2".into(), "znver3".into()]),
                    Some(current_proc_opt.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_PROCESSOR_OPT".into(), val);
                }

                let resp = widgets::menu::run(
                    Some(term),
                    "LTO Mode".into(),
                    "Link-time optimization:".into(),
                    Value::Array(vec!["no".into(), "thin".into(), "full".into()]),
                    Some(current_lto.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_LTO_MODE".into(), val);
                }
            }
            "Preemption & Tickless" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Preempt RT".into(),
                    "Real-time preemption:".into(),
                    Value::Array(vec!["0 (off)".into(), "1 (basic)".into(), "2 (full)".into()]),
                    Some(match current_preempt_rt.as_str() {
                        "1" => "1 (basic)".into(),
                        "2" => "2 (full)".into(),
                        _ => "0 (off)".into(),
                    }),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_PREEMPT_RT".into(), val.chars().next().unwrap_or('0').to_string());
                }

                let resp = widgets::menu::run(
                    Some(term),
                    "Tickless Mode".into(),
                    "Select tickless:".into(),
                    Value::Array(vec!["0 (periodic)".into(), "1 (idle)".into(), "2 (full)".into()]),
                    Some(match current_tickless.as_str() {
                        "0" => "0 (periodic)".into(),
                        "1" => "1 (idle)".into(),
                        _ => "2 (full)".into(),
                    }),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_TICKLESS".into(), val.chars().next().unwrap_or('2').to_string());
                }
            }
            "Timer & Governor" => {
                let resp = widgets::menu::run(
                    Some(term),
                    "Timer Frequency".into(),
                    "Select Hz:".into(),
                    Value::Array(vec!["100".into(), "250".into(), "300".into(), "1000".into()]),
                    Some(current_timer.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_TIMER_FREQ".into(), val);
                }

                let resp = widgets::menu::run(
                    Some(term),
                    "CPU Governor".into(),
                    "Default governor:".into(),
                    Value::Array(vec!["ondemand".into(), "performance".into(), "schedutil".into(), "conservative".into()]),
                    Some(current_gov.clone()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_CPU_GOV".into(), val);
                }
            }
            "Patches" => {
                let patches = vec![
                    ("TKG_GLITCHED_BASE", "Glitched Base", "Community patchset"),
                    ("TKG_ZENIFY", "Zenify", "Zen kernel tweaks"),
                    ("TKG_CLEAR_PATCHES", "Clear Patches", "Intel Clear Linux patches"),
                    ("TKG_OPENRGB", "OpenRGB", "OpenRGB kernel support"),
                    ("TKG_ACS_OVERRIDE", "ACS Override", "PCIe ACS override for VFIO"),
                    ("TKG_FSYNC", "Fsync", "Fastsync for Wine/Proton"),
                    ("TKG_MGLRU", "MGLRU", "Multi-gen LRU"),
                    ("TKG_NTSYNC", "NTsync", "NT synchronization for Wine"),
                ];
                let current_map: HashMap<&str, &str> = [
                    ("TKG_GLITCHED_BASE", current_glitched.as_str()),
                    ("TKG_ZENIFY", current_zenify.as_str()),
                    ("TKG_CLEAR_PATCHES", current_clear.as_str()),
                    ("TKG_OPENRGB", current_openrgb.as_str()),
                    ("TKG_ACS_OVERRIDE", current_acs.as_str()),
                    ("TKG_FSYNC", current_fsync.as_str()),
                    ("TKG_MGLRU", current_mglru.as_str()),
                    ("TKG_NTSYNC", current_ntsync.as_str()),
                ].iter().cloned().collect();

                let choices: Vec<String> = patches.iter().map(|(_, label, _)| label.to_string()).collect();
                let defaults: Vec<String> = patches.iter()
                    .filter(|(key, _, _)| current_map.get(key) == Some(&"true"))
                    .map(|(_, label, _)| label.to_string())
                    .collect();

                let resp = widgets::checklist::run(
                    Some(term),
                    "TKG Patches".into(),
                    "Toggle patches:".into(),
                    choices,
                    None, None, None,
                    Some(defaults),
                )?;
                if !resp.cancelled {
                    let selected: Vec<String> = resp.result
                        .and_then(|v| v.as_array().cloned())
                        .map(|a| a.iter().filter_map(|v| v.as_str().map(String::from)).collect())
                        .unwrap_or_default();
                    for (key, label, _) in &patches {
                        let val = if selected.contains(&label.to_string()) { "true" } else { "false" };
                        values.insert(key.to_string(), val.to_string());
                    }
                }
            }
            "CPU Count" => {
                let resp = widgets::input::run(
                    Some(term),
                    "CPU Count".into(),
                    "Number of CPUs for compilation:".into(),
                    Some(current_nr_cpus.clone()),
                    Some("e.g. 8".into()),
                    None,
                )?;
                if !resp.cancelled {
                    let val = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
                    values.insert("TKG_NR_CPUS".into(), val);
                }
            }
            "Done" => return Ok(()),
            _ => {}
        }
    }
}

fn num_cpus() -> usize {
    std::thread::available_parallelism().map(|n| n.get()).unwrap_or(1)
}