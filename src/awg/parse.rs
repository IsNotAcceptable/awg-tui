use anyhow::{anyhow, bail, Result};

use super::model::{InterfaceInfo, PeerInfo};

/// Parses the output of `awg show <iface> dump`.
///
/// First line: `private-key public-key listen-port fwmark [amnezia params…]`
/// Next lines (one per peer, tab separated):
/// `public-key preshared-key endpoint allowed-ips latest-handshake rx tx keepalive`
pub fn parse_dump(name: &str, dump: &str) -> Result<InterfaceInfo> {
    let mut lines = dump.lines().filter(|l| !l.trim().is_empty());

    let head = lines.next().ok_or_else(|| anyhow!("empty dump for {name}"))?;
    let f: Vec<&str> = head.split('\t').collect();
    if f.len() < 4 {
        bail!("malformed interface line for {name}: {head}");
    }

    let mut info = InterfaceInfo {
        name: name.to_string(),
        public_key: f[1].to_string(),
        listen_port: f[2].parse().ok(),
        peers: Vec::new(),
    };

    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 8 {
            bail!("malformed peer line for {name}: {line}");
        }
        info.peers.push(PeerInfo {
            public_key: f[0].to_string(),
            endpoint: Some(f[2]).filter(|e| *e != "(none)").map(str::to_string),
            allowed_ips: f[3].to_string(),
            latest_handshake: f[4].parse::<u64>().ok().filter(|&t| t > 0),
            rx: f[5].parse().unwrap_or(0),
            tx: f[6].parse().unwrap_or(0),
            keepalive: f[7].parse().ok(),
        });
    }
    Ok(info)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_interface_and_peers() {
        let dump = "PRIV\tPUB\t51820\toff\t5\t10\t20\n\
                    PEERPUB\t(none)\t1.2.3.4:51820\t0.0.0.0/0\t1700000000\t100\t200\t25\n\
                    PEER2\t(none)\t(none)\t10.0.0.2/32\t0\t0\t0\toff\n";
        let i = parse_dump("awg0", dump).unwrap();
        assert_eq!(i.public_key, "PUB");
        assert_eq!(i.listen_port, Some(51820));
        assert_eq!(i.peers.len(), 2);
        assert_eq!(i.peers[0].endpoint.as_deref(), Some("1.2.3.4:51820"));
        assert_eq!(i.peers[0].keepalive, Some(25));
        assert_eq!(i.peers[1].endpoint, None);
        assert_eq!(i.peers[1].latest_handshake, None);
        assert_eq!(i.totals(), (100, 200));
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_dump("x", "").is_err());
        assert!(parse_dump("x", "a\tb").is_err());
    }
}
