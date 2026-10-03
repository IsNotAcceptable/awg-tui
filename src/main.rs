mod app;
mod args;
mod awg;
mod config;
mod editor;
mod event;
mod fmt;
mod jobs;
mod kmod;
mod ui;

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::DefaultTerminal;

use crate::app::App;

const TICK: Duration = Duration::from_secs(1);
const POLL: Duration = Duration::from_millis(150);

fn main() -> Result<()> {
    let Some(opts) = args::parse()? else {
        return Ok(());
    };

    let mut app = App::new(opts);
    app.refresh();

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, &mut app);
    ratatui::restore();
    result
}

fn run(terminal: &mut DefaultTerminal, app: &mut App) -> Result<()> {
    let mut last_tick = Instant::now();

    while !app.should_quit {
        terminal.draw(|frame| ui::draw(frame, app))?;

        if event::poll(POLL)? {
            event::handle(app)?;
        }
        app.drain_jobs();

        if let Some(path) = app.pending_edit.take() {
            editor::edit(app, &path);
            *terminal = ratatui::init();
            app.refresh();
        }
        if last_tick.elapsed() >= TICK {
            app.refresh();
            last_tick = Instant::now();
        }
    }
    Ok(())
}
