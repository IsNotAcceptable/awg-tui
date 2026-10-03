use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};

use crate::app::App;

pub fn poll(timeout: Duration) -> Result<bool> {
    Ok(event::poll(timeout)?)
}

pub fn handle(app: &mut App) -> Result<()> {
    let Event::Key(key) = event::read()? else {
        return Ok(());
    };
    if key.kind != KeyEventKind::Press {
        return Ok(());
    }

    match (key.code, key.modifiers) {
        (KeyCode::Char('c'), KeyModifiers::CONTROL) => app.should_quit = true,
        (KeyCode::Char('q') | KeyCode::Esc, _) => app.should_quit = true,
        (KeyCode::Down | KeyCode::Char('j'), _) => app.next(),
        (KeyCode::Up | KeyCode::Char('k'), _) => app.prev(),
        (KeyCode::Enter | KeyCode::Char(' '), _) => app.toggle(),
        (KeyCode::Tab, _) => app.cycle_view(),
        (KeyCode::Char('r'), _) => app.refresh(),
        (KeyCode::Char('m'), _) => app.load_module(),
        _ => {}
    }
    Ok(())
}
