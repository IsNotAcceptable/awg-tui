//! First run installer for the AWG kernel module and tools
//! Detects distro and runs matching steps

use std::fs;
use anyhow::Result;
use crate::awg::Awg;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Distro {
    Nixos,
    Arch,
    Debian,
    Fedora,
    Other,
}

pub fn detect() -> Distro {
    let text = fs::read_to_string("/etc/os-release").unwrap_or_default();
    let id: Vec<String> = text
        .lines()
        .filter_map(|l| l.strip_prefix("ID=").or_else(|| l.strip_prefix("ID_LIKE=")))
        .flat_map(|v| v.trim_matches('"').split_whitespace().map(str::to_string).collect::<Vec<_>>())
        .collect();
    let has = |s: &str| id.iter().any(|i| i==s);
    if has("arch") {
        Distro::Arch
    } else if has("debian") || has("ubuntu") {
        Distro::Debian
    } else if has("fedora") {
        Distro::Fedora
    } else if has("nixos") {
        Distro::Nixos
    } else {
        Distro::Other
    }
}

/// One privileged command with a desc shown to user
pub struct Step {
    pub desc: &'static str,
    pub bin: &'static str,
    pub args: &'static [&'static str],
}

/// For distro, `none` -> manually install
pub fn plan(distro: Distro) -> Option<Vec<Step>> {
    match distro {

        Distro::Debian => Some(vec![Step {
            desc: "install build depends",
            bin: "apt-get",
            // TODO: verify exact package list against upstream README
            args: &["install", "-y", "git", "make", "gcc", "dkms", "linux-headers-generic"],
        }]),

        Distro::Fedora => Some(vec![Step {
            desc: "install build depends",
            bin: "dnf",
            args: &["install", "-y", "git", "make", "gcc", "dkms", "kernel-devel"],
        }]),

        Distro::Nixos | Distro::Arch | Distro::Other => None,
    }
}

/// run steps one by one
pub fn run(awg: &Awg, steps: &[Step], log: &mut Vec<String>) -> Result<()> {
    for s in steps {
        log.push(format!("» {}", s.desc));
        let out = awg.sudo_run(s.bin, s.args)?;
        let lines: Vec<&str> = out.lines().collect();
        let tail = lines.len().saturating_sub(3);
        log.extend(lines[tail..].iter().map(|l| l.to_string()));
    }
    Ok(())
}

/// shown text when automatic install is not possible.
pub fn manual_hint(distro: Distro) -> &'static str {
    match distro {
        Distro::Nixos => "NixOS: enable programs.awg-tui.kernelModule in your configuration",
        Distro::Arch => "Arch: yay -S amneziawg-dkms amneziawg-tools (AUR)",
        _ => "install amneziawg kernel module and tools manually, see the project README",
    }
}
