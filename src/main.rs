mod app;
mod args;
mod awg;
mod config;
mod event;
mod fmt;
mod kmod;
mod ui;

use std::time::{Duration, Instant};

use anyhow::Result;
use ratatui::DefaultTerminal;

use crate::app::App;

const TICK: Duration = Duration::from_secs(1);

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

        let timeout = TICK.saturating_sub(last_tick.elapsed());
        if event::poll(timeout)? {
            event::handle(app)?;
        }
        if last_tick.elapsed() >= TICK {
            app.refresh();
            last_tick = Instant::now();
        }
    }
    Ok(())
}
