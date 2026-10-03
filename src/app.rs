use std::collections::{BTreeMap, HashMap};
use std::path::PathBuf;
use std::time::Instant;

use anyhow::anyhow;

use crate::args::Options;
use crate::awg::{Awg, InterfaceInfo};
use crate::{config, kmod};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum View {
    Info,
    Config,
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

pub struct App {
    pub opts: Options,
    pub awg: Awg,
    pub tunnels: Vec<Tunnel>,
    pub selected: usize,
    pub view: View,
    pub kmod_loaded: bool,
    pub status: Option<Status>,
    pub config_text: String,
    pub should_quit: bool,
    /// iface -> (rx B/s, tx B/s)
    pub rates: HashMap<String, (f64, f64)>,
    prev: HashMap<String, (Instant, u64, u64)>,
}

impl App {
    pub fn new(opts: Options) -> Self {
        Self {
            opts,
            awg: Awg::new(),
            tunnels: Vec::new(),
            selected: 0,
            view: View::Info,
            kmod_loaded: false,
            status: None,
            config_text: String::new(),
            should_quit: false,
            rates: HashMap::new(),
            prev: HashMap::new(),
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

        self.tunnels = map.into_values().collect();
        self.selected = keep
            .and_then(|n| self.tunnels.iter().position(|t| t.name == n))
            .unwrap_or(0)
            .min(self.tunnels.len().saturating_sub(1));
        self.load_config_text();
    }

    fn update_rate(&mut self, info: &InterfaceInfo) {
        let (rx, tx) = info.totals();
        let now = Instant::now();
        if let Some(&(t, prx, ptx)) = self.prev.get(&info.name) {
            let dt = now.duration_since(t).as_secs_f64();
            if dt > 0.0 {
                self.rates.insert(
                    info.name.clone(),
                    (rx.saturating_sub(prx) as f64 / dt, tx.saturating_sub(ptx) as f64 / dt),
                );
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

    pub fn next(&mut self) {
        if !self.tunnels.is_empty() {
            self.selected = (self.selected + 1) % self.tunnels.len();
            self.load_config_text();
        }
    }

    pub fn prev(&mut self) {
        if !self.tunnels.is_empty() {
            self.selected = (self.selected + self.tunnels.len() - 1) % self.tunnels.len();
            self.load_config_text();
        }
    }

    pub fn cycle_view(&mut self) {
        self.view = match self.view {
            View::Info => View::Config,
            View::Config => View::Info,
        };
    }

    /// Brings the selected tunnel up or down via `awg-quick`.
    pub fn toggle(&mut self) {
        let Some(t) = self.selected_tunnel() else { return };
        let name = t.name.clone();

        let result = match (&t.live, &t.path) {
            (Some(_), Some(p)) => self.awg.quick_down(p).map(|_| format!("{name} is down")),
            (None, Some(p)) => self.awg.quick_up(p).map(|_| format!("{name} is up")),
            (Some(_), None) => Err(anyhow!("{name}: no config file, cannot bring it down")),
            (None, None) => return,
        };

        match result {
            Ok(msg) => self.set_info(msg),
            Err(e) => self.set_error(format!("{e:#}")),
        }
        self.refresh();
    }

    pub fn load_module(&mut self) {
        match kmod::load(&self.awg) {
            Ok(()) => self.set_info(format!("module {} loaded", kmod::MODULE)),
            Err(e) => self.set_error(format!("{e:#}")),
        }
        self.refresh();
    }
}
