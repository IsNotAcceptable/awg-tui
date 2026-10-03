use std::path::Path;

use anyhow::Result;

use crate::awg::Awg;

pub const MODULE: &str = "amneziawg";

/// True if the module is loaded (or built into the kernel).
pub fn is_loaded() -> bool {
    Path::new("/sys/module").join(MODULE).exists()
}

pub fn load(awg: &Awg) -> Result<()> {
    awg.modprobe(MODULE)
}
