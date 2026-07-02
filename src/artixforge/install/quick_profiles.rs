use crate::contract::Response;
use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::collections::HashMap;
use std::fs::File;

type State = HashMap<String, String>;

fn defaults() -> State {
    let mut s = State::new();
    s.insert("FS_TYPE".into(), "ext4".into());
    s.insert("BOOTLOADER".into(), "grub".into());
    s.insert("KERNEL_CHOICE".into(), "linux".into());
    s.insert("INIT".into(), "openrc".into());
    s.insert("PRIV_ESCALATION".into(), "sudo".into());
    s.insert("USE_LUKS".into(), "no".into());
    s.insert("USE_LVM".into(), "no".into());
    s.insert("GENERATE_UKI".into(), "no".into());
    s.insert("ALLOW_OFFLINE".into(), "no".into());
    s.insert("ENABLE_ARCH_REPOS".into(), "no".into());
    s.insert("MICROCODE_OVERRIDE".into(), "auto".into());
    s.insert("KEEP_BINARY_KERNEL".into(), "yes".into());
    s.insert("COREUTILS".into(), "gnu".into());
    s.insert("KERNEL_CONFIG_DEPTH".into(), "auto".into());
    s.insert("POWER_USER".into(), "no".into());
    s.insert("USER_SHELL".into(), "bash".into());
    s
}

fn desktop(wm: &str, dm: &str, xstack: &str, arch: &str, extras: &str) -> State {
    let mut s = defaults();
    s.insert("WM_DE".into(), wm.into());
    s.insert("DISPLAY_MANAGER".into(), dm.into());
    s.insert("NETWORK_STACK".into(), "networkmanager".into());
    s.insert("AUDIO_STACK".into(), "pipewire".into());
    s.insert("X_STACK".into(), xstack.into());
    s.insert("BTRFS_LAYOUT".into(), "standard".into());
    s.insert("ENABLE_ARCH_REPOS".into(), arch.into());
    s.insert("EXTRAS".into(), extras.into());
    s
}

fn wayland(wm: &str, arch: &str, extras: &str) -> State {
    desktop(wm, "none", "none", arch, extras)
}

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
) -> Result<Option<State>> {
    let profile = widgets::menu::run(
        Some(term),
        "Quick Profile".into(),
        "Select desktop environment:".into(),
        Value::Array(
            vec![
                "KDE Plasma", "XFCE", "MangoWM", "Hyprland", "Sway", "Niri", "i3wm", "dwm",
                "LXQt", "LXDE", "Cinnamon", "Budgie", "Moksha", "COSMIC",
                "Server (no desktop)", "Embedded (BusyBox)", "Volk's Personal",
                "TestingQP", "Load custom profile...",
            ]
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect(),
        ),
        None,
        None,
    )?;

    if profile.cancelled {
        return Ok(None);
    }

    let choice = profile
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let state = match choice.as_str() {
        "KDE Plasma" => kde(term)?,
        "XFCE" => xfce(term)?,
        "MangoWM" => Some(wayland(
            "mango", "yes",
            "git firefox alacritty waybar wofi swaybg swaylock fzf zoxide starship eza btop tmux",
        )),
        "Hyprland" => Some(wayland(
            "hyprland", "yes",
            "git firefox alacritty waybar wofi hyprpaper hyprlock fzf zoxide starship eza btop tmux",
        )),
        "Sway" => Some(wayland(
            "sway", "no",
            "git firefox alacritty waybar wofi swaybg swaylock fzf zoxide starship eza btop tmux",
        )),
        "Niri" => Some(wayland(
            "niri", "no",
            "git firefox alacritty waybar fuzzel swaybg swaylock fzf zoxide starship eza btop tmux",
        )),
        "i3wm" => Some(desktop(
            "i3wm", "lightdm", "xlibre", "yes",
            "git firefox alacritty fzf zoxide starship eza btop tmux",
        )),
        "dwm" => Some(desktop(
            "dwm", "lightdm", "xlibre", "yes",
            "git firefox st fzf zoxide starship eza tmux",
        )),
        "LXQt" => Some(desktop(
            "lxqt", "sddm", "xlibre", "no",
            "git firefox alacritty fzf zoxide starship eza btop tmux",
        )),
        "LXDE" => Some(desktop(
            "lxde", "lightdm", "xlibre", "no",
            "git firefox alacritty fzf zoxide starship eza btop tmux",
        )),
        "Cinnamon" => Some(desktop(
            "cinnamon", "lightdm", "xlibre", "no",
            "git firefox alacritty fzf zoxide starship eza btop tmux",
        )),
        "Budgie" => Some(desktop(
            "budgie", "lightdm", "xlibre", "no",
            "git firefox alacritty fzf zoxide starship eza btop tmux",
        )),
        "Moksha" => Some(desktop(
            "moksha", "lightdm", "xlibre", "no",
            "git firefox terminology fzf zoxide starship eza tmux",
        )),
        "COSMIC" => Some(desktop(
            "cosmic", "lightdm", "none", "no",
            "git firefox alacritty fzf zoxide starship eza btop tmux",
        )),
        "Server (no desktop)" => server(term)?,
        "Embedded (BusyBox)" => Some(embedded()),
        "Volk's Personal" => Some(volk()),
        "TestingQP" => Some(testing()),
        "Load custom profile..." => load_profile(term)?,
        _ => None,
    };

    if let Some(ref s) = state {
        if !confirm(term, s)? {
            return Ok(None);
        }
    }

    Ok(state)
}

fn kde(term: &mut Terminal<CrosstermBackend<File>>) -> Result<Option<State>> {
    let v = widgets::menu::run(
        Some(term),
        "KDE Plasma".into(),
        "Variant:".into(),
        Value::Array(
            vec![
                "Full - plasma + kde-applications",
                "Desktop - plasma",
                "Minimal - plasma-desktop only",
            ]
            .iter()
            .map(|s| Value::String(s.to_string()))
            .collect(),
        ),
        None,
        None,
    )?;
    if v.cancelled {
        return Ok(None);
    }
    let variant = v
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let mut s = desktop(
        "kde", "sddm", "xlibre", "yes",
        "git flatpak fastfetch firewalld bluez zram-tools firefox neovim alacritty fzf zoxide starship eza btop htop tmux mpv",
    );
    if variant.contains("Full") {
        s.insert("KDE_PROFILE".into(), "full".into());
    } else if variant.contains("Desktop") {
        s.insert("KDE_PROFILE".into(), "desktop".into());
    } else {
        s.insert("KDE_PROFILE".into(), "minimal".into());
    }
    Ok(Some(s))
}

fn xfce(term: &mut Terminal<CrosstermBackend<File>>) -> Result<Option<State>> {
    let v = widgets::menu::run(
        Some(term),
        "XFCE".into(),
        "Variant:".into(),
        Value::Array(
            vec!["Full - xfce4 + goodies", "Minimal - xfce4 only"]
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
        None,
        None,
    )?;
    if v.cancelled {
        return Ok(None);
    }
    let variant = v
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let extras = if variant.contains("Full") {
        "git firefox neovim alacritty fzf zoxide starship eza btop tmux mpv"
    } else {
        "git neovim tmux"
    };
    Ok(Some(desktop("xfce4", "lightdm", "xlibre", "no", extras)))
}

fn server(term: &mut Terminal<CrosstermBackend<File>>) -> Result<Option<State>> {
    let v = widgets::menu::run(
        Some(term),
        "Server".into(),
        "Variant:".into(),
        Value::Array(
            vec!["Full - firewalld, zram, SSH", "Minimal - SSH only"]
                .iter()
                .map(|s| Value::String(s.to_string()))
                .collect(),
        ),
        None,
        None,
    )?;
    if v.cancelled {
        return Ok(None);
    }
    let variant = v
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();
    let mut s = defaults();
    s.insert("WM_DE".into(), "none".into());
    s.insert("DISPLAY_MANAGER".into(), "none".into());
    s.insert("NETWORK_STACK".into(), "dhcpcd+iwd".into());
    s.insert("AUDIO_STACK".into(), "none".into());
    s.insert("X_STACK".into(), "none".into());
    s.insert("PRIV_ESCALATION".into(), "doas".into());
    if variant.contains("Full") {
        s.insert("EXTRAS".into(), "git firewalld zram-tools tmux".into());
    } else {
        s.insert("EXTRAS".into(), "git tmux".into());
    }
    Ok(Some(s))
}

fn embedded() -> State {
    let mut s = defaults();
    s.insert("WM_DE".into(), "none".into());
    s.insert("DISPLAY_MANAGER".into(), "none".into());
    s.insert("NETWORK_STACK".into(), "none".into());
    s.insert("AUDIO_STACK".into(), "none".into());
    s.insert("X_STACK".into(), "none".into());
    s.insert("PRIV_ESCALATION".into(), "none".into());
    s.insert("INIT".into(), "busybox".into());
    s.insert("KERNEL_CHOICE".into(), "linux-lts".into());
    s.insert("COREUTILS".into(), "busybox".into());
    s.insert("POWER_USER".into(), "yes".into());
    s.insert("KEEP_BINARY_KERNEL".into(), "no".into());
    s.insert("EXTRAS".into(), "".into());
    s
}

fn volk() -> State {
    let mut s = desktop(
        "kde", "lightdm", "xlibre", "yes",
        "git fastfetch tmux htop kitty firewalld flatpak",
    );
    s.insert("KDE_PROFILE".into(), "minimal".into());
    s.insert("INIT".into(), "dinit".into());
    s.insert("PRIV_ESCALATION".into(), "doas".into());
    s.insert("POWER_USER".into(), "yes".into());
    s.insert("KEEP_BINARY_KERNEL".into(), "no".into());
    s.insert("NETWORK_STACK".into(), "dhcpcd+iwd".into());
    s
}

fn testing() -> State {
    let mut s = defaults();
    s.insert("FS_TYPE".into(), "xfs".into());
    s.insert("BOOTLOADER".into(), "limine".into());
    s.insert("KERNEL_CHOICE".into(), "linux-cachyos-bore".into());
    s.insert("INIT".into(), "s6".into());
    s.insert("PRIV_ESCALATION".into(), "doas".into());
    s.insert("USE_LUKS".into(), "yes".into());
    s.insert("USE_LVM".into(), "yes".into());
    s.insert("GENERATE_UKI".into(), "yes".into());
    s.insert("ENABLE_ARCH_REPOS".into(), "yes".into());
    s.insert("MICROCODE_OVERRIDE".into(), "none".into());
    s.insert("COREUTILS".into(), "busybox".into());
    s.insert("WM_DE".into(), "mango".into());
    s.insert("DISPLAY_MANAGER".into(), "lightdm".into());
    s.insert("NETWORK_STACK".into(), "dhcpcd+iwd".into());
    s.insert("AUDIO_STACK".into(), "pipewire".into());
    s.insert("X_STACK".into(), "none".into());
    s.insert("USER_SHELL".into(), "fish".into());
    s.insert(
        "EXTRAS".into(),
        "git fastfetch tmux htop kitty firewalld flatpak".into(),
    );
    s
}

fn load_profile(
    term: &mut Terminal<CrosstermBackend<File>>,
) -> Result<Option<State>> {
    let resp = widgets::file_picker::run(
        Some(term),
        "Load Profile".into(),
        Some("/mnt/etc".into()),
        Some("conf".into()),
    )?;
    if resp.cancelled {
        return Ok(None);
    }
    let path = resp
        .result
        .and_then(|v| v.as_str().map(String::from))
        .unwrap_or_default();

    let mut state = State::new();
    if let Ok(content) = std::fs::read_to_string(&path) {
        for line in content.lines() {
            if let Some((key, value)) = line.split_once('=') {
                let key = key.trim().to_string();
                let value = value.trim().trim_matches('"').to_string();
                state.insert(key, value);
            }
        }
    }
    if !state.is_empty() {
        Ok(Some(state))
    } else {
        Ok(None)
    }
}

fn confirm(term: &mut Terminal<CrosstermBackend<File>>, state: &State) -> Result<bool> {
    let summary = format!(
        "Filesystem: {}\nBootloader: {}\nKernel: {}\nInit: {}\nDesktop: {}\nDisplay Manager: {}\nNetwork: {}\nAudio: {}\nX Stack: {}\nPrivilege: {}\nLUKS: {}\nLVM: {}\nUKI: {}\nExtras: {}",
        state.get("FS_TYPE").map_or("", |v| v),
        state.get("BOOTLOADER").map_or("", |v| v),
        state.get("KERNEL_CHOICE").map_or("", |v| v),
        state.get("INIT").map_or("", |v| v),
        state.get("WM_DE").map_or("", |v| v),
        state.get("DISPLAY_MANAGER").map_or("", |v| v),
        state.get("NETWORK_STACK").map_or("", |v| v),
        state.get("AUDIO_STACK").map_or("", |v| v),
        state.get("X_STACK").map_or("", |v| v),
        state.get("PRIV_ESCALATION").map_or("", |v| v),
        state.get("USE_LUKS").map_or("", |v| v),
        state.get("USE_LVM").map_or("", |v| v),
        state.get("GENERATE_UKI").map_or("", |v| v),
        state.get("EXTRAS").map_or("", |v| v),
    );

    let resp = widgets::yesno::run(
        Some(term),
        "Confirm Profile".into(),
        summary,
        Some(true),
    )?;
    Ok(!resp.cancelled && resp.result.and_then(|v| v.as_bool()).unwrap_or(false))
}