#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct PeerInfo {
    pub public_key: String,
    pub endpoint: Option<String>,
    pub allowed_ips: String,
    /// Unix timestamp (seconds) of the last handshake.
    pub latest_handshake: Option<u64>,
    pub rx: u64,
    pub tx: u64,
    pub keepalive: Option<u16>,
}

/// Handshake older than this means the perr is probably unreachable
/// AWG rekeys about every 2 min while traffic flows.
pub const STALE_SECS: u64 = 180;
impl PeerInfo {
    pub fn is_stale(&self, now: u64) -> bool {
        self.latest_handshake.map_or(true, |t| now.saturating_sub(t) > STALE_SECS)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterfaceInfo {
    pub name: String,
    pub public_key: String,
    pub listen_port: Option<u16>,
    /// AmneziaWG obfuscation parameters (Jc, Jmin, S1, H1, I1 …) from `awg showconf`.
    pub params: Vec<(String, String)>,
    pub peers: Vec<PeerInfo>,
}

impl InterfaceInfo {
    /// Total (rx, tx) over all peers.
    pub fn totals(&self) -> (u64, u64) {
        self.peers
            .iter()
            .fold((0, 0), |(rx, tx), p| (rx + p.rx, tx + p.tx))
    }
}
