//! Cloudflare WARP endpoint scanner.
//!
//! For each candidate `ip:port` the peer endpoint of the *live* interface is
//! switched (`awg set … endpoint`) and a few pings are sent through the tunnel.
//! The loss / RTT of those pings is the score. The original endpoint is
//! restored when the scan ends.

use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::Sender;
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};

use super::{Msg, Probe};
use crate::awg::Awg;

const PREFIXES: &[&str] = &[
    "162.159.192",
    "162.159.193",
    "162.159.195",
    "188.114.96",
    "188.114.97",
    "188.114.98",
    "188.114.99",
];

const PORTS: &[u16] = &[
    500, 854, 859, 864, 878, 880, 890, 891, 894, 903, 908, 928, 934, 939, 942, 943, 945, 946, 955,
    968, 987, 988, 1002, 1010, 1014, 1018, 1070, 1074, 1180, 1387, 1701, 1843, 2371, 2408, 2506,
    3138, 3476, 3581, 3854, 4177, 4198, 4233, 4500, 5279, 5956, 7103, 7152, 7156, 7281, 7559,
    8319, 8742, 8854, 8886,
];

const PING_TARGET: &str = "1.1.1.1";

/// `count` random endpoints; `first` (the current endpoint) is always included as a baseline.
pub fn candidates(count: usize, first: Option<&str>) -> Vec<String> {
    let mut seed = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos() as u64)
        .unwrap_or(0x9E37_79B9_7F4A_7C15)
        | 1;
    let mut next = move || {
        seed ^= seed << 13;
        seed ^= seed >> 7;
        seed ^= seed << 17;
        seed
    };

    let mut out: Vec<String> = first.map(str::to_string).into_iter().collect();
    let target = out.len() + count;
    let mut guard = 0;
    while out.len() < target && guard < count * 20 {
        guard += 1;
        let prefix = PREFIXES[(next() % PREFIXES.len() as u64) as usize];
        let host = 1 + next() % 254;
        let port = PORTS[(next() % PORTS.len() as u64) as usize];
        let c = format!("{prefix}.{host}:{port}");
        if !out.contains(&c) {
            out.push(c);
        }
    }
    out
}

pub fn run(
    awg: Awg,
    iface: String,
    peer: String,
    original: Option<String>,
    list: Vec<String>,
    cancel: Arc<AtomicBool>,
    tx: Sender<Msg>,
) {
    for ep in list {
        if cancel.load(Ordering::Relaxed) {
            break;
        }
        let probe = match awg.set_endpoint(&iface, &peer, &ep) {
            Ok(()) => measure(&awg, &iface, &ep),
            Err(_) => Probe { endpoint: ep, loss: 100, rtt_ms: None },
        };
        if tx.send(Msg::Probe(probe)).is_err() {
            return;
        }
    }
    if let Some(o) = original {
        let _ = awg.set_endpoint(&iface, &peer, &o);
    }
    let _ = tx.send(Msg::ScanDone);
}

fn measure(awg: &Awg, iface: &str, ep: &str) -> Probe {
    let out = awg.raw_plain(
        "ping",
        &["-c", "3", "-i", "0.4", "-W", "2", "-I", iface, PING_TARGET],
    );
    match out {
        Ok(o) => {
            let (loss, rtt_ms) = parse_ping(&String::from_utf8_lossy(&o.stdout));
            Probe { endpoint: ep.to_string(), loss, rtt_ms }
        }
        Err(_) => Probe { endpoint: ep.to_string(), loss: 100, rtt_ms: None },
    }
}

/// Returns (loss %, avg rtt ms) from iputils-style ping output.
pub fn parse_ping(out: &str) -> (u32, Option<f64>) {
    let loss = out
        .lines()
        .find(|l| l.contains("packet loss"))
        .and_then(|l| {
            let pct = l.split('%').next()?;
            let num = pct.rsplit(|c: char| !(c.is_ascii_digit() || c == '.')).next()?;
            num.parse::<f64>().ok()
        })
        .map_or(100, |v| v.round() as u32);

    let rtt = out
        .lines()
        .find(|l| l.contains("min/avg/max"))
        .and_then(|l| l.split('=').nth(1))
        .and_then(|v| v.trim().split('/').nth(1))
        .and_then(|v| v.parse::<f64>().ok());

    (loss, rtt)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_ping() {
        let ok = "3 packets transmitted, 3 received, 0% packet loss, time 800ms\n\
                  rtt min/avg/max/mdev = 11.1/14.5/20.0/3.2 ms\n";
        assert_eq!(parse_ping(ok), (0, Some(14.5)));
        let bad = "3 packets transmitted, 0 received, 100% packet loss, time 2040ms\n";
        assert_eq!(parse_ping(bad), (100, None));
        let part = "3 packets transmitted, 2 received, 33.3333% packet loss, time 1ms\n";
        assert_eq!(parse_ping(part).0, 33);
    }

    #[test]
    fn generates_candidates() {
        let c = candidates(10, Some("1.2.3.4:5"));
        assert_eq!(c.len(), 11);
        assert_eq!(c[0], "1.2.3.4:5");
        assert!(c[1..].iter().all(|e| e.contains(':')));
    }
}
