//! Background jobs (endpoint scan, connectivity check). They run in their own
//! threads and report back through an mpsc channel drained by `App`.

pub mod scan;
pub mod trace;

#[derive(Debug, Clone)]
pub struct Probe {
    pub endpoint: String,
    /// Packet loss in percent (100 = unreachable).
    pub loss: u32,
    pub rtt_ms: Option<f64>,
}

pub enum Msg {
    Probe(Probe),
    ScanDone,
    Trace(Result<trace::Trace, String>),
}
