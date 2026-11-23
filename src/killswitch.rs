//! Kill Switch: nftables table that drops all output except loopback
//! tunnel and handshake packets to the peer endpoint

use crate::awg::Awg;
use crate::config::valid_iface_name;
use anyhow::{anyhow, bail, Result};
use std::net::Ipv4Addr;

pub const TABLE: &str = "awg_tui_ks";

/// split ip:port (only ipv4 endpoints)
fn split_endpoint(ep: &str) -> Result<(Ipv4Addr, u16)> {
    let (ip, port) = ep
        .rsplit_once(':')
        .ok_or_else(|| anyhow!("bad endpoint {ep}"))?;
    let ip: Ipv4Addr = ip
        .parse()
        .map_err(|_| anyhow!("kill switch supports IPv4 endpoints only: {ep}"))?;
    let port: u16 = port.parse().map_err(|_| anyhow!("bad port in {ep}"))?;
    Ok((ip, port))
}

/// nft script that re/creates the table. `add` + `delete` first makes it idempotent.
pub fn ruleset(iface: &str, endpoint: &str) -> Result<String> {
    if !valid_iface_name(iface) {
        bail!("refusing interface name {iface:?}");
    }
    let (ip, port) = split_endpoint(endpoint)?;
    Ok(format!(
        "add table inet {TABLE}\n\
         delete table inet {TABLE}\n\
         table inet {TABLE} {{\n\
         \tchain output {{\n\
         \t\ttype filter hook output priority 0; policy accept;\n\
         \t\toifname \"lo\" accept\n\
         \t\toifname \"{iface}\" accept\n\
         \t\tip daddr {ip} udp dport {port} accept\n\
         \t\tdrop\n\
         \t}}\n\
         }}\n"
    ))
}

pub fn enable(awg: &Awg, iface: &str, endpoint: &str) -> Result<()> {
    awg.nft_apply(&ruleset(iface, endpoint)?)
}

pub fn disable(awg: &Awg) -> Result<()> {
    awg.nft_apply(&format!(
        "add table inet {TABLE}\ndelete table inet {TABLE}\n"
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allows_only_tunnel_and_endpoint() {
        let r = ruleset("WARPv3", "162.159.192.1:2408").unwrap();
        assert!(r.contains("oifname \"WARPv3\" accept"));
        assert!(r.contains("ip daddr 162.159.192.1 udp dport 2408 accept"));
        assert!(r.trim_end().ends_with("drop\n\t}\n}"));
    }

    #[test]
    fn rejects_bad_input() {
        assert!(ruleset("x", "[2606:4700::1]:2408").is_err());
        assert!(ruleset("x", "1.2.3.4:notaport").is_err());
        assert!(ruleset("bad\"name", "1.2.3.4:1").is_err());
    }
}
