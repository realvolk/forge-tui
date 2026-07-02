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
    let mut warnings: Vec<String> = Vec::new();

    if boot_mode == "bios" {
        warnings.push("BIOS/Legacy boot mode -- UEFI features (UKI, EFIStub, rEFInd, Limine) are disabled".into());
    }

    let bl = values.get("BOOTLOADER").map_or("", |v| v);
    let fs = values.get("FS_TYPE").map_or("", |v| v);
    let kernel = values.get("KERNEL_CHOICE").map_or("", |v| v);
    let init = values.get("INIT").map_or("", |v| v);
    let wm = values.get("WM_DE").map_or("", |v| v);
    let luks = values.get("USE_LUKS").map_or("", |v| v);
    let lvm = values.get("USE_LVM").map_or("", |v| v);
    let _uki = values.get("GENERATE_UKI").map_or("", |v| v);
    let arch = values.get("ENABLE_ARCH_REPOS").map_or("", |v| v);
    let xstack = values.get("X_STACK").map_or("", |v| v);
    let dm = values.get("DISPLAY_MANAGER").map_or("", |v| v);
    let net = values.get("NETWORK_STACK").map_or("", |v| v);
    let priv_esc = values.get("PRIV_ESCALATION").map_or("", |v| v);
    let pw = values.get("POWER_USER").map_or("", |v| v);
    let keep_bin = values.get("KEEP_BINARY_KERNEL").map_or("", |v| v);
    let pw_pkgs = values.get("POWERUSER_PACKAGES").map_or("", |v| v);
    let coreutils = values.get("COREUTILS").map_or("", |v| v);
    let offline = values.get("ALLOW_OFFLINE").map_or("", |v| v);
    let quick = values.get("QUICK_INSTALL").map_or("", |v| v);

    if bl == "efistub" && boot_mode != "bios" {
        warnings.push("EFIStub needs compatible UEFI firmware".into());
    }
    if bl == "uki" {
        warnings.push("UKI is UEFI-only -- BIOS systems not supported".into());
    }
    if kernel == "linux-libre" {
        warnings.push("linux-libre removes non-free firmware -- hardware may not work".into());
    }
    if kernel == "tkg" {
        warnings.push("TKG kernel is compiled from source during installation -- may take 20-30 minutes".into());
    }
    if bl == "grub" && fs == "xfs" {
        warnings.push("GRUB + XFS: ensure bigtime is disabled for compatibility".into());
    }
    if bl == "uki" && luks == "yes" {
        warnings.push("UKI + LUKS: ensure initramfs includes encrypt hook".into());
    }
    if init == "busybox" {
        warnings.push("BusyBox init is minimal -- manual service scripts required".into());
    }
    if init == "busybox" && wm != "none" {
        warnings.push("BusyBox init with a desktop -- you will need to start services manually".into());
    }
    if init == "busybox" && coreutils != "busybox" && coreutils != "artix" {
        warnings.push("BusyBox init with GNU coreutils -- consider BusyBox coreutils for consistency".into());
    }
    if lvm == "yes" && bl == "grub" {
        warnings.push("LVM + GRUB: ensure lvm2 hook is in initramfs".into());
    }
    if lvm == "yes" && bl == "efistub" {
        warnings.push("LVM + EFIStub: cmdline must reference /dev/mapper paths".into());
    }
    if lvm == "yes" && luks == "yes" {
        warnings.push("LVM on LUKS: correct crypt device order is critical".into());
    }
    if luks == "yes" && bl == "refind" {
        warnings.push("LUKS + rEFInd: may require manual boot config".into());
    }
    if coreutils == "busybox" {
        warnings.push("BusyBox coreutils -- some scripts may need GNU extensions".into());
    }
    if coreutils == "uutils" {
        warnings.push("uutils coreutils -- Rust-based, may have compatibility gaps".into());
    }
    if coreutils == "custom" {
        warnings.push("Custom coreutils -- ensure all essential tools are implemented".into());
    }
    if coreutils != "gnu" && coreutils != "none" && !coreutils.is_empty() {
        warnings.push("Non-GNU coreutils: some install scripts may behave unexpectedly".into());
    }
    if wm == "cosmic" {
        warnings.push("COSMIC is alpha software -- APIs may change, features may be missing".into());
    }
    if wm == "moksha" {
        warnings.push("Moksha/Enlightenment is community-maintained -- limited testing".into());
    }
    if wm == "none" {
        warnings.push("No desktop environment selected".into());
    }
    if wm == "sonicde" {
        warnings.push("SonicDE is a third-party KDE replacement -- not officially supported by Artix".into());
    }
    if wm == "sonicde" && arch == "no" {
        warnings.push("SonicDE may need Arch repositories for dependencies".into());
    }
    if (wm == "hyprland" || wm == "niri" || wm == "sway") && xstack == "xorg" {
        warnings.push("Wayland compositor selected but X.Org display stack configured".into());
    }
    if (wm == "hyprland" || wm == "niri") && arch == "no" {
        warnings.push("Hyprland/Niri may need Arch repositories for dependencies".into());
    }
    if dm == "none" && wm != "none" {
        warnings.push("No display manager -- you will start the desktop manually".into());
    }
    if net == "none" {
        warnings.push("No network stack -- you will configure networking manually".into());
    }
    if pw == "yes" && keep_bin == "no" {
        warnings.push("No fallback kernel -- system may be unbootable if custom kernel fails".into());
    }
    if pw == "yes" && pw_pkgs.contains("glibc") {
        warnings.push("glibc from source is DANGEROUS -- a miscompilation breaks everything".into());
    }
    if pw == "yes" && init == "busybox" {
        warnings.push("BusyBox init from source -- ensure the recipe compiled successfully".into());
    }
    if offline == "yes" {
        warnings.push("Offline mode -- packages may be outdated or missing".into());
    }
    if priv_esc == "none" {
        warnings.push("No privilege escalation tool -- you will need to configure su manually".into());
    }
    if priv_esc == "doas" && pw == "yes" {
        warnings.push("doas + Power User: anvil commands require root; use \"doas anvil ...\"".into());
    }
    if quick == "yes" && wm == "embedded" {
        warnings.push("Embedded profile: minimal system, no networking, no desktop -- know what you are doing".into());
    }

    if !warnings.is_empty() {
        let msg = warnings
            .iter()
            .map(|w| format!(" - {}", w))
            .collect::<Vec<_>>()
            .join("\n");
        widgets::msg::run(Some(term), "Sanity Warnings".into(), msg)?;
    }
    Ok(())
}