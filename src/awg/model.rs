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

#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct InterfaceInfo {
    pub name: String,
    pub public_key: String,
    pub listen_port: Option<u16>,
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
