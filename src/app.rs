use std::collections::{BTreeMap, HashMap, VecDeque};
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::Arc;
use std::time::Duration;
use std::thread;

use anyhow::{anyhow, Result};

use crate::args::Options;
use crate::awg::{Awg, InterfaceInfo};
use crate::jobs::{self, trace::Trace, Msg, Probe};
use crate::{config, kmod};

const SCAN_COUNT: usize = 24;
const HISTORY: usize = 90;
const AUTO_COOLDOWN: Duration = Duration::from_secs(600);

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Info,
    Config,
    Log,
    Scan,
}

pub enum Mode {
    Normal,
    /// Typing a path to a .conf file to import.
    Import(String),
    /// Waiting for y/n to delete the named tunnel.
    ConfirmDelete(String),
}

pub enum TraceState {
    Idle,
    Loading,
    Done(Trace),
    Failed(String),
}

pub struct Tunnel {
    pub name: String,
    /// `None` for interfaces that are up but have no config file in the config dir.
    pub path: Option<PathBuf>,
    pub live: Option<InterfaceInfo>,
}

impl Tunnel {
    pub fn is_up(&self) -> bool {
        self.live.is_some()
    }
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Level {
    Info,
    Error,
}

pub struct Status {
    pub text: String,
    pub level: Level,
}

pub struct ScanState {
    pub results: Vec<Probe>,
    pub total: usize,
    pub done: bool,
    pub selected: usize,
    pub iface: String,
    pub peer: String,
    pub auto: bool,
    cancel: Arc<AtomicBool>,
}

impl ScanState {
    /// Best first: lowest loss, then lowest RTT.
    pub fn sorted(&self) -> Vec<&Probe> {
        let mut v: Vec<&Probe> = self.results.iter().collect();
        v.sort_by(|a, b| {
            a.loss.cmp(&b.loss).then(
                a.rtt_ms
                    .partial_cmp(&b.rtt_ms)
                    .unwrap_or(std::cmp::Ordering::Equal),
            )
        });
        v
    }
}

#[derive(Default)]
pub struct Speed {
    pub rx: VecDeque<u64>,
    pub tx: VecDeque<u64>,
}

pub struct App {
    pub opts: Options,
    pub awg: Awg,
    pub tunnels: Vec<Tunnel>,
    pub selected: usize,
    pub view: View,
    pub mode: Mode,
    pub kmod_loaded: bool,
    pub status: Option<Status>,
    pub config_text: String,
    config_for: Option<PathBuf>,
    pub log_text: String,
    pub trace: TraceState,
    pub scan: Option<ScanState>,
    pub should_quit: bool,
    /// Set by the key handler; the main loop suspends the TUI and opens the editor.
    pub pending_edit: Option<PathBuf>,
    /// iface -> (rx B/s, tx B/s)
    pub rates: HashMap<String, (f64, f64)>,
    pub history: HashMap<String, Speed>,
    last_auto: Option<std::time::Instant>,
    prev: HashMap<String, (std::time::Instant, u64, u64)>,
    tx: Sender<Msg>,
    rx: Receiver<Msg>,
}

impl App {
    pub fn new(opts: Options) -> Self {
        let (tx, rx) = mpsc::channel();
        Self {
            opts,
            awg: Awg::new(),
            tunnels: Vec::new(),
            selected: 0,
            view: View::Info,
            mode: Mode::Normal,
            kmod_loaded: false,
            status: None,
            config_text: String::new(),
            config_for: None,
            log_text: String::new(),
            trace: TraceState::Idle,
            scan: None,
            should_quit: false,
            pending_edit: None,
            rates: HashMap::new(),
            history: HashMap::new(),
            last_auto: None,
            prev: HashMap::new(),
            tx,
            rx,
        }
    }

    pub fn selected_tunnel(&self) -> Option<&Tunnel> {
        self.tunnels.get(self.selected)
    }

    pub fn set_info(&mut self, text: impl Into<String>) {
        self.status = Some(Status { text: text.into(), level: Level::Info });
    }

    pub fn set_error(&mut self, text: impl Into<String>) {
        self.status = Some(Status { text: text.into(), level: Level::Error });
    }

    // ---------------------------------------------------------------- refresh

    /// Re-reads config files and live interface state.
    pub fn refresh(&mut self) {
        self.kmod_loaded = kmod::is_loaded();
        let keep = self.selected_tunnel().map(|t| t.name.clone());

        let mut map: BTreeMap<String, Tunnel> = BTreeMap::new();
        match config::scan(&self.opts.config_dir) {
            Ok(list) => {
                for c in list {
                    map.insert(
                        c.name.clone(),
                        Tunnel { name: c.name, path: Some(c.path), live: None },
                    );
                }
            }
            Err(e) => self.set_error(format!("{e:#}")),
        }

        match self.awg.interfaces() {
            Ok(names) => {
                for name in names {
                    match self.awg.show(&name) {
                        Ok(info) => {
                            self.update_rate(&info);
                            map.entry(name.clone())
                                .or_insert_with(|| Tunnel {
                                    name: name.clone(),
                                    path: None,
                                    live: None,
                                })
                                .live = Some(info);
                        }
                        Err(e) => self.set_error(format!("{e:#}")),
                    }
                }
            }
            Err(e) => self.set_error(format!("{e:#}")),
        }

        // drop rate history of interfaces that disappeared
        let live: Vec<&String> = map.values().filter(|t| t.is_up()).map(|t| &t.name).collect();
        self.prev.retain(|k, _| live.contains(&k));
        self.rates.retain(|k, _| live.contains(&k));
        self.history.retain(|k, _| live.contains(&k));

        self.tunnels = map.into_values().collect();
        self.selected = keep
            .and_then(|n| self.tunnels.iter().position(|t| t.name == n))
            .unwrap_or(0)
            .min(self.tunnels.len().saturating_sub(1));
        self.sync_config();
        self.check_watchdog();
    }

    /// Warns about tunnels whose peers stoped handshaking
    fn check_watchdog(&mut self) {
        if self.scan.as_ref().is_some_and(|s| !s.done) {
            return;
        }
        let now = crate::fmt::now_secs();
        let stale = self.tunnels.iter().position(|t| {
            t.live.as_ref().is_some_and(|i| {
                i.peers
                    .iter()
                    // without keepalive, a quiet peer legitimately has no fresh handshake
                    .any(|p| p.keepalive.is_some() && p.is_stale(now))
            })
        });
        let Some(idx) = stale else { return };
        let name = self.tunnels[idx].name.clone();
        if !self.opts.auto_rescan {
            self.set_error(format!("{name}: no recent handshake - press s to rescan endpoints"));
            return;
        }
        if self.last_auto.is_some_and(|t| t.elapsed() < AUTO_COOLDOWN) {
            return;
        }
        self.last_auto = Some(std::time::Instant::now());
        self.selected = idx;
        self.set_error(format!("{name}: no recent handshake — rescanning endpoints"));
        self.begin_scan(true);
    }

    pub fn invalidate_config(&mut self) {
        self.config_for = None;
    }

    fn sync_config(&mut self) {
        let path = self.selected_tunnel().and_then(|t| t.path.clone());
        if path == self.config_for {
            return;
        }
        self.config_for = path;
        self.load_config_text();
    }

    /// `r`: refresh plus reload the kernel log.
    pub fn manual_refresh(&mut self) {
        self.invalidate_config();
        self.refresh();
        if self.view == View::Log {
            self.load_log();
        }
    }

    fn update_rate(&mut self, info: &InterfaceInfo) {
        let (rx, tx) = info.totals();
        let now = std::time::Instant::now();
        if let Some(&(t, prx, ptx)) = self.prev.get(&info.name) {
            let dt = now.duration_since(t).as_secs_f64();
            if dt > 0.0 {
                let r = rx.saturating_sub(prx) as f64 / dt;
                let w = tx.saturating_sub(ptx) as f64 / dt;
                self.rates.insert(info.name.clone(), (r, w));
                let h = self.history.entry(info.name.clone()).or_default();
                push_capped(&mut h.rx, r as u64);
                push_capped(&mut h.tx, w as u64);
            }
        }
        self.prev.insert(info.name.clone(), (now, rx, tx));
    }

    fn load_config_text(&mut self) {
        self.config_text = match self.selected_tunnel().and_then(|t| t.path.clone()) {
            Some(p) => match self.awg.read_file(&p) {
                Ok(t) => config::mask_secrets(&t),
                Err(e) => format!("cannot read config: {e:#}"),
            },
            None => String::from("(no config file for this interface)"),
        };
    }

    fn load_log(&mut self) {
        self.log_text = match self.awg.kernel_log() {
            Ok(t) if t.trim().is_empty() => "(no amneziawg messages in the kernel log)".into(),
            Ok(t) => t,
            Err(e) => format!("cannot read kernel log: {e:#}"),
        };
    }

    // ------------------------------------------------------------- navigation

    pub fn next(&mut self) {
        if !self.tunnels.is_empty() {
            self.selected = (self.selected + 1) % self.tunnels.len();
            self.sync_config();
        }
    }

    pub fn prev(&mut self) {
        if !self.tunnels.is_empty() {
            self.selected = (self.selected + self.tunnels.len() - 1) % self.tunnels.len();
            self.sync_config();
        }
    }

    /// Tab: Info → Config → Log → Info.
    pub fn cycle_view(&mut self) {
        self.view = match self.view {
            View::Info | View::Scan => View::Config,
            View::Config => View::Log,
            View::Log => View::Info,
        };
        if self.view == View::Log {
            self.load_log();
        }
    }

    // ----------------------------------------------------------------- tunnels

    /// Brings the selected tunnel up or down via `awg-quick`.
    pub fn toggle(&mut self) {
        let Some(t) = self.selected_tunnel() else { return };
        let (name, live, path) = (t.name.clone(), t.is_up(), t.path.clone());

        let result: Result<String> = match (live, path) {
            (true, Some(p)) => self.awg.quick_down(&p).map(|_| format!("{name} is down")),
            (false, Some(p)) => self.bring_up(&name, &p),
            (true, None) => Err(anyhow!("{name}: no config file, cannot bring it down")),
            (false, None) => return,
        };

        match result {
            Ok(msg) => self.set_info(msg),
            Err(e) => self.set_error(format!("{e:#}")),
        }
        self.trace = TraceState::Idle;
        self.refresh();
    }

    /// Starts `name`; unless `--multi`, other running tunnels are stopped first
    /// (two full-tunnel configs share the same fwmark / routing rules).
    fn bring_up(&self, name: &str, path: &Path) -> Result<String> {
        let mut stopped = Vec::new();
        if !self.opts.multi {
            for other in self.tunnels.iter().filter(|t| t.is_up() && t.name != name) {
                if let Some(p) = &other.path {
                    self.awg.quick_down(p)?;
                    stopped.push(other.name.clone());
                }
            }
        }
        self.awg.quick_up(path)?;
        Ok(if stopped.is_empty() {
            format!("{name} is up")
        } else {
            format!("{name} is up ({} brought down)", stopped.join(", "))
        })
    }

    pub fn load_module(&mut self) {
        match kmod::load(&self.awg) {
            Ok(()) => self.set_info(format!("module {} loaded", kmod::MODULE)),
            Err(e) => self.set_error(format!("{e:#}")),
        }
        self.refresh();
    }

    pub fn request_edit(&mut self) {
        match self.selected_tunnel().and_then(|t| t.path.clone()) {
            Some(p) => self.pending_edit = Some(p),
            None => self.set_error("no config file for this interface"),
        }
    }

    pub fn start_import(&mut self) {
        self.mode = Mode::Import(String::new());
    }

    pub fn import(&mut self, raw: &str) {
        let src = PathBuf::from(raw.trim());
        let Some(stem) = src.file_stem().map(|s| s.to_string_lossy().into_owned()) else {
            return self.set_error("empty path");
        };
        if src.extension().map_or(true, |e| e != "conf") || !src.is_file() {
            return self.set_error(format!("{}: not a .conf file", src.display()));
        }
        if !config::valid_iface_name(&stem) {
            return self.set_error(format!(
                "'{stem}' is not a valid interface name (max 15 chars: A-Z a-z 0-9 _ = + . -)"
            ));
        }
        let dest = self.opts.config_dir.join(format!("{stem}.conf"));
        match self.awg.install_file(&src, &dest) {
            Ok(()) => self.set_info(format!("imported {stem}")),
            Err(e) => self.set_error(format!("{e:#}")),
        }
        self.invalidate_config();
        self.refresh();
    }

    pub fn ask_delete(&mut self) {
        match self.selected_tunnel() {
            Some(t) if t.is_up() => self.set_error("bring the tunnel down first"),
            Some(t) if t.path.is_none() => self.set_error("no config file to delete"),
            Some(t) => self.mode = Mode::ConfirmDelete(t.name.clone()),
            None => {}
        }
    }

    pub fn delete_confirmed(&mut self, name: &str) {
        let path = self
            .tunnels
            .iter()
            .find(|t| t.name == name)
            .and_then(|t| t.path.clone());
        if let Some(p) = path {
            match self.awg.remove_file(&p) {
                Ok(()) => self.set_info(format!("deleted {name}")),
                Err(e) => self.set_error(format!("{e:#}")),
            }
        }
        self.invalidate_config();
        self.refresh();
    }

    // ------------------------------------------------------------------- jobs

    pub fn drain_jobs(&mut self) {
        while let Ok(msg) = self.rx.try_recv() {
            match msg {
                Msg::Probe(p) => {
                    if let Some(s) = &mut self.scan {
                        s.results.push(p);
                    }
                }
                Msg::ScanDone => {
                    if let Some(s) = &mut self.scan {
                        s.done = true;
                    }
                    if self.scan.as_ref().is_some_and(|s| s.auto) {
                        self.apply_best_if_reachable();
                    } else {
                        self.set_info("scan finished — Enter applies the selected endpoint");
                    }
                }
                Msg::Trace(r) => {
                    self.trace = match r {
                        Ok(t) => TraceState::Done(t),
                        Err(e) => TraceState::Failed(e),
                    }
                }
            }
        }
    }

    pub fn check_trace(&mut self) {
        if matches!(self.trace, TraceState::Loading) {
            return;
        }
        self.trace = TraceState::Loading;
        let (awg, tx) = (self.awg.clone(), self.tx.clone());
        thread::spawn(move || {
            let r = jobs::trace::fetch(&awg).map_err(|e| format!("{e:#}"));
            let _ = tx.send(Msg::Trace(r));
        });
    }

    pub fn start_scan(&mut self) {
        self.begin_scan(false);
    }

    fn begin_scan(&mut self, auto: bool) {
        if self.scan.as_ref().is_some_and(|s| !s.done) {
            self.view = View::Scan;
            return;
        }
        let info = self.selected_tunnel().and_then(|t| t.live.clone());
        let Some(info) = info else {
            return self.set_error("bring the tunnel up first (Enter)");
        };
        let Some(peer) = info.peers.first() else {
            return self.set_error("the tunnel has no peers");
        };
        let (peer_key, original) = (peer.public_key.clone(), peer.endpoint.clone());
        let list = jobs::scan::candidates(SCAN_COUNT, original.as_deref());

        let cancel = Arc::new(AtomicBool::new(false));
        self.scan = Some(ScanState {
            results: Vec::new(),
            total: list.len(),
            done: false,
            selected: 0,
            iface: info.name.clone(),
            peer: peer_key.clone(),
            auto,
            cancel: cancel.clone(),
        });
        self.view = View::Scan;
        if !auto {
            self.set_info("scanning... (x to stop)");
        }

        let (awg, tx, iface) = (self.awg.clone(), self.tx.clone(), info.name);
        thread::spawn(move || jobs::scan::run(awg, iface, peer_key, original, list, cancel, tx));
    }

    pub fn cancel_scan(&mut self) {
        if let Some(s) = &self.scan {
            if !s.done {
                s.cancel.store(true, Ordering::Relaxed);
                self.set_info("stopping scan…");
            }
        }
    }

    pub fn scan_move(&mut self, delta: isize) {
        if let Some(s) = &mut self.scan {
            let n = s.results.len();
            if n > 0 {
                s.selected = (s.selected as isize + delta).rem_euclid(n as isize) as usize;
            }
        }
    }

    /// Applies the highlighted endpoint to the live peer and saves it into the config file.
    pub fn apply_scan_choice(&mut self) {
        let picked = self.scan.as_ref().map(|s| {
            let choice = s
                .sorted()
                .get(s.selected)
                .map(|p| (p.endpoint.clone(), s.iface.clone(), s.peer.clone()));
            (s.done, choice)
        });
        let Some((done, choice)) = picked else { return };
        if !done {
            return self.set_error("scan is still running (x to stop it first)");
        }
        let Some((endpoint, iface, peer)) = choice else { return };

        if let Err(e) = self.awg.set_endpoint(&iface, &peer, &endpoint) {
            return self.set_error(format!("{e:#}"));
        }

        let path = self
            .tunnels
            .iter()
            .find(|t| t.name == iface)
            .and_then(|t| t.path.clone());
        let saved = path.map(|p| {
            let text = self.awg.read_file(&p)?;
            let new = config::replace_endpoint(&text, &endpoint)
                .ok_or_else(|| anyhow!("no Endpoint line in {}", p.display()))?;
            self.awg.write_file(&p, &new)
        });
        match saved {
            Some(Ok(())) => self.set_info(format!("endpoint {endpoint} applied and saved to config")),
            Some(Err(e)) => self.set_error(format!("applied {endpoint}, but saving failed: {e:#}")),
            None => self.set_info(format!("endpoint {endpoint} applied (no config file to save)")),
        }
        self.invalidate_config();
        self.refresh();
    }

    /// Auto mode: apply the top-ranked endpoint if anything answered.
    fn apply_best_if_reachable(&mut self) {
        let reachable = self.scan.as_mut().is_some_and(|s| {
            s.selected = 0;
            s.sorted().first().is_some_and(|p| p.loss < 100)
        });
        if reachable {
            self.apply_scan_choice();
        } else {
            self.set_error("auto rescan: no reachable endpoint found");
        }
    }
}

fn push_capped(q: &mut VecDeque<u64>, v: u64) {
    if q.len() >= HISTORY {
        q.pop_front();
    }
    q.push_back(v);
}
