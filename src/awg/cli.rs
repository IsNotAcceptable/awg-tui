use std::fs;
use std::io::Write;
use std::path::Path;
use std::process::{Command, Output, Stdio};

use anyhow::{bail, Context, Result};

use super::model::InterfaceInfo;
use super::parse::{apply_transfer, parse_dump, parse_params};

/// Runs `awg`, `awg-quick`, `modprobe`… and escalates through `sudo -n`
/// when the TUI itself is not running as root.
#[derive(Clone)]
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
        let mut info = parse_dump(iface, &out)?;
        if let Ok(t) = self.run(&self.awg_bin, &["show", iface, "transfer"]) {
            apply_transfer(&mut info, &t);
        }
        if let Ok(c) = self.run(&self.awg_bin, &["showconf", iface]) {
            info.params = parse_params(&c);
        }
        Ok(info)
    }

    pub fn set_endpoint(&self, iface: &str, peer: &str, endpoint: &str) -> Result<()> {
        self.run(
            &self.awg_bin,
            &["set", iface, "peer", peer, "endpoint", endpoint],
        )
        .map(drop)
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

    pub fn write_file(&self, path: &Path, content: &str) -> Result<()> {
        if self.sudo {
            self.exec("tee", &[&path.to_string_lossy()], Some(content), true)
                .and_then(check)
                .map(drop)
        } else {
            fs::write(path, content).with_context(|| format!("writing {}", path.display()))
        }
    }

    /// Copies `src` to `dest` with mode 600 (creates parent dirs).
    pub fn install_file(&self, src: &Path, dest: &Path) -> Result<()> {
        self.run(
            "install",
            &["-D", "-m", "600", &src.to_string_lossy(), &dest.to_string_lossy()],
        )
        .map(drop)
    }

    pub fn remove_file(&self, path: &Path) -> Result<()> {
        self.run("rm", &["-f", &path.to_string_lossy()]).map(drop)
    }

    /// Recent kernel log lines mentioning amneziawg.
    pub fn kernel_log(&self) -> Result<String> {
        self.run(
            "journalctl",
            &["-k", "-g", "amnezia", "-n", "100", "--no-pager", "-o", "short"],
        )
    }

    /// Runs a command without privilege escalation, returning raw output
    /// (non-zero exit is *not* an error — e.g. `ping` with 100% loss).
    pub fn raw_plain(&self, bin: &str, args: &[&str]) -> Result<Output> {
        self.exec(bin, args, None, false)
    }

    fn run(&self, bin: &str, args: &[&str]) -> Result<String> {
        let out = check(self.exec(bin, args, None, true)?)?;
        Ok(String::from_utf8_lossy(&out.stdout).into_owned())
    }

    fn exec(&self, bin: &str, args: &[&str], stdin: Option<&str>, escalate: bool) -> Result<Output> {
        let mut cmd = if escalate && self.sudo {
            let mut c = Command::new("sudo");
            c.args(["-n", bin]);
            c
        } else {
            Command::new(bin)
        };
        cmd.args(args)
            .stdout(Stdio::piped())
            .stderr(Stdio::piped())
            .stdin(if stdin.is_some() { Stdio::piped() } else { Stdio::null() });

        let mut child = cmd
            .spawn()
            .with_context(|| format!("failed to execute `{bin}`"))?;
        if let Some(data) = stdin {
            if let Some(mut si) = child.stdin.take() {
                si.write_all(data.as_bytes())?;
            }
        }
        let out = child.wait_with_output()?;
        // keep command name in the error message
        if !out.status.success() {
            let err = String::from_utf8_lossy(&out.stderr);
            let err = err.lines().last().unwrap_or("").trim().to_string();
            if escalate {
                return Err(anyhow::anyhow!("{bin} {}: {err}", args.join(" ")));
            }
        }
        Ok(out)
    }
}

fn check(out: Output) -> Result<Output> {
    if out.status.success() {
        Ok(out)
    } else {
        let err = String::from_utf8_lossy(&out.stderr);
        bail!("{}", err.lines().last().unwrap_or("command failed").trim())
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
