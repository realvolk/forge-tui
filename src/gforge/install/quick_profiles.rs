use std::collections::HashMap;
use crate::widgets;
use anyhow::Result;
use ratatui::{backend::CrosstermBackend, Terminal};
use serde_json::Value;
use std::fs::File;

type State = HashMap<String, String>;

pub fn run(
    term: &mut Terminal<CrosstermBackend<File>>,
) -> Result<Option<State>> {
    let choices = vec!["Desktop", "Server", "Minimal", "Custom"];
    let resp = widgets::menu::run(
        Some(term),
        "Quick Profile".into(),
        "Start with a pre-configured setup:".into(),
        Value::Array(choices.iter().map(|s| Value::String(s.to_string())).collect()),
        Some("Custom".into()),
        None,
    )?;
    if resp.cancelled { return Ok(None); }
    let choice = resp.result.and_then(|v| v.as_str().map(String::from)).unwrap_or_default();

    let state = match choice.as_str() {
        "Desktop" => desktop_profile(),
        "Server" => server_profile(),
        "Minimal" => minimal_profile(),
        _ => return Ok(None),
    };

    Ok(Some(state))
}

fn desktop_profile() -> State {
    let mut s = State::new();
    s.insert("INIT".into(), "openrc".into());
    s.insert("KERNEL_CHOICE".into(), "gentoo-kernel-bin".into());
    s.insert("KERNEL_CONFIG_METHOD".into(), "binary".into());
    s.insert("STAGE3_VARIANT".into(), "desktop-openrc".into());
    s.insert("WM_DE".into(), "gnome".into());
    s.insert("DISPLAY_MANAGER".into(), "gdm".into());
    s.insert("AUDIO_STACK".into(), "pipewire".into());
    s.insert("NETWORK_STACK".into(), "networkmanager".into());
    s.insert("GLOBAL_USE".into(), "X wayland pipewire networkmanager elogind bluetooth cups gtk -kde -qt5 -qt6 -systemd".into());
    s.insert("VIDEO_CARDS".into(), "intel".into());
    s.insert("ACCEPTED_LICENSES".into(), "@FREE @BINARY-REDISTRIBUTABLE".into());
    s.insert("EXTRAS".into(), "firefox alacritty neovim mpv flatpak".into());
    s.insert("GENTOO_CFLAGS".into(), "-march=native -O2 -pipe".into());
    s.insert("GENTOO_MAKEOPTS".into(), "-j$(nproc)".into());
    s.insert("USE_BINHOST".into(), "yes".into());
    s.insert("INSTALL_EIX".into(), "yes".into());
    s
}

fn server_profile() -> State {
    let mut s = State::new();
    s.insert("INIT".into(), "openrc".into());
    s.insert("KERNEL_CHOICE".into(), "gentoo-kernel".into());
    s.insert("KERNEL_CONFIG_METHOD".into(), "binary".into());
    s.insert("STAGE3_VARIANT".into(), "openrc".into());
    s.insert("WM_DE".into(), "none".into());
    s.insert("DISPLAY_MANAGER".into(), "none".into());
    s.insert("AUDIO_STACK".into(), "none".into());
    s.insert("NETWORK_STACK".into(), "dhcpcd+iwd".into());
    s.insert("GLOBAL_USE".into(), "-X -wayland -pulseaudio -gtk -qt5 -qt6 -gnome -kde -cups -bluetooth elogind ipv6".into());
    s.insert("ACCEPTED_LICENSES".into(), "@FREE".into());
    s.insert("EXTRAS".into(), "neovim git htop tmux firewalld".into());
    s.insert("ENABLE_SSHD".into(), "yes".into());
    s.insert("GENTOO_CFLAGS".into(), "-march=native -O2 -pipe".into());
    s.insert("GENTOO_MAKEOPTS".into(), "-j$(nproc)".into());
    s.insert("USE_BINHOST".into(), "no".into());
    s
}

fn minimal_profile() -> State {
    let mut s = State::new();
    s.insert("INIT".into(), "openrc".into());
    s.insert("KERNEL_CHOICE".into(), "gentoo-kernel".into());
    s.insert("KERNEL_CONFIG_METHOD".into(), "binary".into());
    s.insert("STAGE3_VARIANT".into(), "openrc".into());
    s.insert("WM_DE".into(), "none".into());
    s.insert("DISPLAY_MANAGER".into(), "none".into());
    s.insert("AUDIO_STACK".into(), "none".into());
    s.insert("NETWORK_STACK".into(), "dhcpcd+iwd".into());
    s.insert("GLOBAL_USE".into(), "-X -wayland -pulseaudio -gtk -qt5 -qt6 -gnome -kde -cups -bluetooth elogind ipv6".into());
    s.insert("ACCEPTED_LICENSES".into(), "@FREE".into());
    s.insert("EXTRAS".into(), "".into());
    s.insert("GENTOO_CFLAGS".into(), "-march=native -O2 -pipe".into());
    s.insert("GENTOO_MAKEOPTS".into(), "-j$(nproc)".into());
    s.insert("USE_BINHOST".into(), "no".into());
    s
}