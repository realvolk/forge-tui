use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::fs::File;

pub fn run_licenses(
    term: &mut Terminal<CrosstermBackend<File>>,
    _current: &str,
) -> Result<Option<String>> {
    let licenses = vec![
        "@FREE", "@BINARY-REDISTRIBUTABLE", "@EULA",
        "GPL-2", "GPL-3", "LGPL-2.1", "BSD", "MIT", "Apache-2.0",
    ];
    let resp = widgets::multiselect::run(
        Some(term),
        "Licenses".into(),
        "Space to toggle, Enter to confirm.".into(),
        licenses.iter().map(|s| s.to_string()).collect(),
        Some("Search licenses...".into()),
        None,
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    let selected = resp
        .result
        .and_then(|v| v.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    Ok(Some(selected))
}

pub fn run_mirrors(
    term: &mut Terminal<CrosstermBackend<File>>,
    current: &str,
) -> Result<Option<String>> {
    let mirrors = widgets::input::run(
        Some(term),
        "GENTOO_MIRRORS".into(),
        "Enter mirror URLs (space separated).".into(),
        Some(current.to_string()),
        Some("https://gentoo.osuosl.org/ https://mirror.leaseweb.com/gentoo/".into()),
        None,
    )?;
    if mirrors.cancelled { return Ok(None); }
    Ok(mirrors.result.and_then(|v| v.as_str().map(String::from)))
}

pub fn run_features(
    term: &mut Terminal<CrosstermBackend<File>>,
    _current: &str,
) -> Result<Option<String>> {
    let features = vec![
        "ccache", "buildpkg", "parallel-install", "keep-going",
        "userpriv", "quiet-build", "getbinpkg",
    ];
    let resp = widgets::multiselect::run(
        Some(term),
        "FEATURES".into(),
        "Space to toggle, Enter to confirm.".into(),
        features.iter().map(|s| s.to_string()).collect(),
        Some("Search features...".into()),
        None,
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    Ok(Some(resp.result.and_then(|v| v.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()))
}

pub fn run_accept_keywords(
    term: &mut Terminal<CrosstermBackend<File>>,
    current: &str,
) -> Result<Option<String>> {
    let resp = widgets::menu::run(
        Some(term),
        "ACCEPT_KEYWORDS".into(),
        "Stability level:".into(),
        Value::Array(vec!["amd64".into(), "~amd64".into()]),
        Some(current.to_string()),
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
}

pub fn run_overlays(
    term: &mut Terminal<CrosstermBackend<File>>,
    _current: &str,
) -> Result<Option<String>> {
    let overlays = vec!["gentoo", "guru", "pentoo", "science"];
    let resp = widgets::multiselect::run(
        Some(term),
        "Overlays".into(),
        "Space to toggle, Enter to confirm.".into(),
        overlays.iter().map(|s| s.to_string()).collect(),
        Some("Search overlays...".into()),
        None,
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    Ok(Some(resp.result.and_then(|v| v.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default()))
}

pub fn run_video_cards(
    term: &mut Terminal<CrosstermBackend<File>>,
    current: &str,
) -> Result<Option<String>> {
    let detected = detect_gpu();
    let is_vm = detected.starts_with("vm-");

    if is_vm {
        let vm_driver = match detected.as_str() {
            "vm-qemu" => "virgl",
            "vm-vmware" => "vmwgfx",
            "vm-virtualbox" => "vboxvideo",
            _ => "virgl",
        };
        widgets::msg::run(
            Some(term),
            "VM Detected".into(),
            format!("Running in {}. Selecting {} driver.", detected, vm_driver),
        )?;
        return Ok(Some(vm_driver.to_string()));
    }

    let choices = vec!["intel", "nvidia", "amdgpu radeonsi", "vesa"];
    let resp = widgets::menu::run(
        Some(term),
        "VIDEO_CARDS".into(),
        format!("Detected GPU: {}. Select driver:", detected),
        Value::Array(choices.iter().map(|s| Value::String(s.to_string())).collect()),
        if current.is_empty() { None } else { Some(current.to_string()) },
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    Ok(resp.result.and_then(|v| v.as_str().map(String::from)))
}

fn detect_gpu() -> String {
    if let Ok(cpuinfo) = std::fs::read_to_string("/proc/cpuinfo") {
        if cpuinfo.contains("hypervisor") {
            let cmd = std::process::Command::new("sh")
                .arg("-c")
                .arg("systemd-detect-virt 2>/dev/null || echo none")
                .output();
            if let Ok(out) = cmd {
                let virt = String::from_utf8_lossy(&out.stdout).trim().to_string();
                return match virt.as_str() {
                    "kvm" | "qemu" => "vm-qemu".into(),
                    "vmware" => "vm-vmware".into(),
                    "oracle" => "vm-virtualbox".into(),
                    _ => "unknown".into(),
                };
            }
        }
    }
    if let Ok(out) = std::process::Command::new("sh")
        .arg("-c")
        .arg("lspci 2>/dev/null | grep -iE 'vga|3d|display' | head -1 || echo unknown")
        .output()
    {
        let gpu_line = String::from_utf8_lossy(&out.stdout).to_lowercase();
        if gpu_line.contains("nvidia") { return "nvidia".into(); }
        if gpu_line.contains("intel") { return "intel".into(); }
        if gpu_line.contains("amd") || gpu_line.contains("ati") { return "amd".into(); }
    }
    "unknown".into()
}

pub fn run_binhost(
    term: &mut Terminal<CrosstermBackend<File>>,
    current: &str,
) -> Result<Option<String>> {
    let use_binhost = widgets::yesno::run(
        Some(term),
        "Binary Packages".into(),
        "Use Gentoo binhost for faster installation?".into(),
        Some(current == "yes"),
    )?;
    if use_binhost.cancelled || !use_binhost.result.and_then(|v| v.as_bool()).unwrap_or(false) {
        return Ok(Some("no".to_string()));
    }

    let has_v3 = std::fs::read_to_string("/proc/cpuinfo")
        .map(|c| c.contains("avx2"))
        .unwrap_or(false);

    let default_url = if has_v3 {
        "https://distfiles.gentoo.org/releases/amd64/binpackages/23.0/x86-64-v3/"
    } else {
        "https://distfiles.gentoo.org/releases/amd64/binpackages/23.0/x86-64/"
    };

    if has_v3 {
        widgets::msg::run(
            Some(term),
            "x86-64-v3".into(),
            "Your CPU supports x86-64-v3. Using optimized binhost.".into(),
        )?;
    }

    let url = widgets::input::run(
        Some(term),
        "Binhost URL".into(),
        "Enter binhost URL:".into(),
        Some(default_url.to_string()),
        None,
        None,
    )?;
    if url.cancelled { return Ok(Some("no".to_string())); }
    Ok(url.result.and_then(|v| v.as_str().map(String::from)))
}

pub fn run_desktop_use_suggestions(
    term: &mut Terminal<CrosstermBackend<File>>,
    wm_de: &str,
) -> Result<Option<String>> {
    let suggestions = match wm_de {
        "gnome" => "-kde -qt5 -qt6 gnome gtk wayland",
        "kde" => "-gnome -gtk kde qt5 qt6",
        "xfce" => "gtk -kde -qt5 -qt6 -gnome",
        "i3" => "gtk -kde -qt5 -qt6 -gnome X",
        _ => return Ok(None),
    };

    widgets::msg::run(
        Some(term),
        "USE Suggestions".into(),
        format!("Recommended USE flags for {}: {}", wm_de, suggestions),
    )?;

    let apply = widgets::yesno::run(
        Some(term),
        "Apply".into(),
        "Apply these USE flag suggestions?".into(),
        Some(true),
    )?;

    if !apply.cancelled && apply.result.and_then(|v| v.as_bool()).unwrap_or(false) {
        Ok(Some(suggestions.to_string()))
    } else {
        Ok(None)
    }
}

pub fn run_telemetry(term: &mut Terminal<CrosstermBackend<File>>) -> Result<bool> {
    let resp = widgets::yesno::run(
        Some(term),
        "Telemetry".into(),
        "Mask Gentoo telemetry package (dev-libs/telemetry)?".into(),
        Some(true),
    )?;
    Ok(!resp.cancelled && resp.result.and_then(|v| v.as_bool()).unwrap_or(false))
}

pub fn run_desktop_extras(
    term: &mut Terminal<CrosstermBackend<File>>,
    _current: &str,
) -> Result<Option<String>> {
    let extras = vec![
        "Vulkan drivers", "Printer support (cups)", "Bluetooth",
        "Power management (tlp)", "SSD TRIM (fstrim)",
        "NetworkManager applet", "Fonts (noto/dejavu)",
        "Input method (ibus/fcitx)",
    ];
    let resp = widgets::multiselect::run(
        Some(term),
        "Desktop Extras".into(),
        "Space to toggle, Enter to confirm.".into(),
        extras.iter().map(|s| s.to_string()).collect(),
        Some("Search extras...".into()),
        None,
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    let selected = resp
        .result
        .and_then(|v| v.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    Ok(Some(selected))
}

pub fn run_tool_groups(
    term: &mut Terminal<CrosstermBackend<File>>,
) -> Result<Option<String>> {
    let groups = vec![
        "Virtualization (libvirt/qemu)", "Containers (docker/podman)",
        "Development (gcc/make/gdb)", "Gaming (steam/wine)",
    ];
    let resp = widgets::multiselect::run(
        Some(term),
        "Tool Groups".into(),
        "Space to toggle, Enter to confirm.".into(),
        groups.iter().map(|s| s.to_string()).collect(),
        Some("Search groups...".into()),
        None,
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    let selected = resp
        .result
        .and_then(|v| v.as_array().cloned())
        .map(|arr| arr.iter().filter_map(|v| v.as_str()).collect::<Vec<_>>().join(" "))
        .unwrap_or_default();
    Ok(Some(selected))
}