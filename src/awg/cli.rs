use std::fs;
use std::path::Path;
use std::process::{Command, Stdio};

use anyhow::{bail, Context, Result};

use super::model::InterfaceInfo;
use super::parse::parse_dump;

/// Runs `awg`, `awg-quick`, `modprobe`… and escalates through `sudo -n`
/// when the TUI itself is not running as root.
pub struct Awg {
    awg_bin: String,
    quick_bin: String,
    sudo: bool,
}

impl Awg {
    pub fn new() -> Self {
        Self {
            awg_bin: std::env::var("AWG_BIN").unwrap_or_else(|_| "awg".into()),
            quick_bin: std::env::var("AWG_QUICK_BIN").unwrap_or_else(|_| "awg-quick".into()),
            sudo: !is_root(),
        }
    }

    pub fn is_root(&self) -> bool {
        !self.sudo
    }

    /// Names of all live amneziawg interfaces.
    pub fn interfaces(&self) -> Result<Vec<String>> {
        let out = self.run(&self.awg_bin, &["show", "interfaces"])?;
        Ok(out.split_whitespace().map(str::to_string).collect())
    }

    pub fn show(&self, iface: &str) -> Result<InterfaceInfo> {
        let out = self.run(&self.awg_bin, &["show", iface, "dump"])?;
        parse_dump(iface, &out)
    }

    pub fn quick_up(&self, conf: &Path) -> Result<()> {
        self.run(&self.quick_bin, &["up", &conf.to_string_lossy()])
            .map(drop)
    }

    pub fn quick_down(&self, conf: &Path) -> Result<()> {
        self.run(&self.quick_bin, &["down", &conf.to_string_lossy()])
            .map(drop)
    }

    pub fn modprobe(&self, module: &str) -> Result<()> {
        self.run("modprobe", &[module]).map(drop)
    }

    /// Reads a (usually root-only) file.
    pub fn read_file(&self, path: &Path) -> Result<String> {
        if self.sudo {
            self.run("cat", &[&path.to_string_lossy()])
        } else {
            fs::read_to_string(path).with_context(|| format!("reading {}", path.display()))
        }
    }

    fn run(&self, bin: &str, args: &[&str]) -> Result<String> {
        let mut cmd = if self.sudo {
            let mut c = Command::new("sudo");
            c.args(["-n", bin]);
            c
        } else {
            Command::new(bin)
        };
        let out = cmd
            .args(args)
            .stdin(Stdio::null())
            .output()
            .with_context(|| format!("failed to execute `{bin}`"))?;

        if out.status.success() {
            Ok(String::from_utf8_lossy(&out.stdout).into_owned())
        } else {
            let err = String::from_utf8_lossy(&out.stderr);
            let err = err.lines().last().unwrap_or("").trim();
            bail!("{bin} {}: {err}", args.join(" "))
        }
    }
}

fn is_root() -> bool {
    fs::read_to_string("/proc/self/status")
        .ok()
        .and_then(|s| {
            s.lines()
                .find_map(|l| l.strip_prefix("Uid:"))
                .and_then(|l| l.split_whitespace().next().map(|u| u == "0"))
        })
        .unwrap_or(false)
}
