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
        params: Vec::new(),
        peers: Vec::new(),
    };

    for line in lines {
        let f: Vec<&str> = line.split('\t').collect();
        // some awg versions glue rx/tx together in `dump`, so only trust the first 5 fields
        if f.len() < 5 {
            bail!("malformed peer line for {name}: {line}");
        }
        info.peers.push(PeerInfo {
            public_key: f[0].to_string(),
            endpoint: Some(f[2]).filter(|e| *e != "(none)").map(str::to_string),
            allowed_ips: f[3].to_string(),
            latest_handshake: f[4].parse::<u64>().ok().filter(|&t| t > 0),
            rx: 0,
            tx: 0,
            keepalive: f.last().filter(|_| f.len() > 5).and_then(|k| k.parse().ok()),
        });
    }
    Ok(info)
}

/// Fills rx/tx from `awg show <iface> transfer` (`key\trx\ttx` per line).
pub fn apply_transfer(info: &mut InterfaceInfo, text: &str) {
    for line in text.lines() {
        let f: Vec<&str> = line.split('\t').collect();
        if f.len() < 3 {
            continue;
        }
        if let Some(p) = info.peers.iter_mut().find(|p| p.public_key == f[0]) {
            p.rx = f[1].parse().unwrap_or(0);
            p.tx = f[2].parse().unwrap_or(0);
        }
    }
}

/// Extracts obfuscation parameters from `awg showconf` ([Interface] section only,
/// without keys / port / fwmark).
pub fn parse_params(showconf: &str) -> Vec<(String, String)> {
    let mut out = Vec::new();
    let mut in_iface = false;
    for line in showconf.lines() {
        let l = line.trim();
        if l.starts_with('[') {
            in_iface = l.eq_ignore_ascii_case("[interface]");
            continue;
        }
        if !in_iface {
            continue;
        }
        if let Some((k, v)) = l.split_once('=') {
            let (k, v) = (k.trim(), v.trim());
            if !matches!(
                k.to_ascii_lowercase().as_str(),
                "privatekey" | "listenport" | "fwmark"
            ) {
                out.push((k.to_string(), v.to_string()));
            }
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_interface_and_peers() {
        let dump = "PRIV\tPUB\t51820\toff\t5\t10\t20\n\
                    PEERPUB\t(none)\t1.2.3.4:51820\t0.0.0.0/0\t1700000000\t100\t200\t25\n\
                    PEER2\t(none)\t(none)\t10.0.0.2/32\t0\t0\toff\n";
        let mut i = parse_dump("awg0", dump).unwrap();
        apply_transfer(&mut i, "PEERPUB\t100\t200\nPEER2\t0\t0\n");
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
    fn extracts_params() {
        let c = "[Interface]\nPrivateKey = x\nListenPort = 1\nFwMark = 0x1\nJc = 4\nI1 = <b 0xaa>\n[Peer]\nPublicKey = p\n";
        assert_eq!(
            parse_params(c),
            vec![("Jc".into(), "4".into()), ("I1".into(), "<b 0xaa>".into())]
        );
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_dump("x", "").is_err());
        assert!(parse_dump("x", "a\tb").is_err());
    }
}
