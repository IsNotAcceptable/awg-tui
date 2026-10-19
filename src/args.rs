use std::path::PathBuf;

use anyhow::{bail, Result};

pub const DEFAULT_CONFIG_DIR: &str = "/etc/amnezia/amneziawg";

#[derive(Debug, Clone)]
pub struct Options {
    pub config_dir: PathBuf,
    /// Allow several tunnels up at once (default: bringing one up drops the others).
    pub multi: bool,
    /// Rescan, apply the best endpint auto when tunnel goes stale.
    pub auto_rescan: bool,
}

const HELP: &str = "\
awg-tui — terminal UI for AmneziaWG (kernel module)

USAGE:
    awg-tui [--config-dir <DIR>]

OPTIONS:
    -c, --config-dir <DIR>   Directory with *.conf tunnels
                             (env: AWG_TUI_CONFIG_DIR, default: /etc/amnezia/amneziawg)
    -m, --multi              Do not bring other tunnels down when starting one
    -a  --auto-rescan        Rescan WARP endpoint automatically when a handshake goes stale
    -h, --help               Print help
    -V, --version            Print version

ENV:
    AWG_BIN        path to `awg`        (default: awg)
    AWG_QUICK_BIN  path to `awg-quick`  (default: awg-quick)
";

/// Returns `None` if the program should exit right away (help/version printed).
pub fn parse() -> Result<Option<Options>> {
    let mut config_dir = std::env::var_os("AWG_TUI_CONFIG_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(DEFAULT_CONFIG_DIR));

    let mut multi = false;
    let mut auto_rescan = false;
    let mut it = std::env::args().skip(1);
    while let Some(arg) = it.next() {
        match arg.as_str() {
            "-h" | "--help" => {
                print!("{HELP}");
                return Ok(None);
            }
            "-V" | "--version" => {
                println!("awg-tui {}", env!("CARGO_PKG_VERSION"));
                return Ok(None);
            }
            "-m" | "--multi" => multi = true,
            "-a" | "--auto-rescan" => auto_rescan = true,
            "-c" | "--config-dir" => match it.next() {
                Some(v) => config_dir = PathBuf::from(v),
                None => bail!("{arg} requires a value"),
            },
            other => bail!("unknown argument: {other}\n\n{HELP}"),
        }
    }
    Ok(Some(Options { config_dir, multi, auto_rescan }))
}
