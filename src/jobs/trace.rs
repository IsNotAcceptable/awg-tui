//! Connectivity check through Cloudflare's `/cdn-cgi/trace`: shows the exit IP,
//! country and whether traffic really goes through WARP (`warp=on|plus`).

use anyhow::{bail, Result};

use crate::awg::Awg;

const URL: &str = "https://www.cloudflare.com/cdn-cgi/trace";

#[derive(Debug, Clone, Default)]
pub struct Trace {
    pub ip: String,
    pub loc: String,
    pub colo: String,
    pub warp: String,
}

impl Trace {
    pub fn warp_active(&self) -> bool {
        self.warp == "on" || self.warp == "plus"
    }
}

pub fn fetch(awg: &Awg) -> Result<Trace> {
    let out = awg.raw_plain("curl", &["-s", "--max-time", "6", URL])?;
    if !out.status.success() {
        bail!("no response (curl exit {:?})", out.status.code());
    }
    let t = parse(&String::from_utf8_lossy(&out.stdout));
    if t.ip.is_empty() {
        bail!("unexpected response");
    }
    Ok(t)
}

pub fn parse(text: &str) -> Trace {
    let mut t = Trace::default();
    for line in text.lines() {
        if let Some((k, v)) = line.split_once('=') {
            match k {
                "ip" => t.ip = v.into(),
                "loc" => t.loc = v.into(),
                "colo" => t.colo = v.into(),
                "warp" => t.warp = v.into(),
                _ => {}
            }
        }
    }
    t
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_trace() {
        let t = parse("fl=1\nip=1.2.3.4\ncolo=FRA\nloc=DE\nwarp=on\n");
        assert_eq!((t.ip.as_str(), t.loc.as_str(), t.colo.as_str()), ("1.2.3.4", "DE", "FRA"));
        assert!(t.warp_active());
    }
}
