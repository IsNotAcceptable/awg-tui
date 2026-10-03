use std::fs;
use std::path::Path;

use anyhow::Result;

use crate::awg::Awg;

pub const MODULE: &str = "amneziawg";

/// True if the module is loaded (or built into the kernel).
/// Informational only: userspace `amneziawg-go` works without it.
pub fn is_loaded() -> bool {
    Path::new("/sys/module").join(MODULE).exists()
        || fs::read_to_string("/proc/modules")
            .map(|s| s.lines().any(|l| l.split_whitespace().next() == Some(MODULE)))
            .unwrap_or(false)
}

pub fn load(awg: &Awg) -> Result<()> {
    awg.modprobe(MODULE)
}
