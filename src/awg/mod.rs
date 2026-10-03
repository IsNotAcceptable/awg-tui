//! Thin layer over the `awg` / `awg-quick` userspace tools that talk to the
//! `amneziawg` kernel module.

mod cli;
mod model;
mod parse;

pub use cli::Awg;
pub use model::InterfaceInfo;
