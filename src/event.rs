use std::time::Duration;

use anyhow::Result;
use ratatui::crossterm::event::{self, Event, KeyCode, KeyEvent, KeyEventKind, KeyModifiers};

use crate::app::{App, Mode, View};

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
    if key.code == KeyCode::Char('c') && key.modifiers.contains(KeyModifiers::CONTROL) {
        app.should_quit = true;
        return Ok(());
    }

    match &app.mode {
        Mode::Normal => normal(app, key),
        Mode::Import(_) => import(app, key),
        Mode::ConfirmDelete(_) => confirm_delete(app, key),
    }
    Ok(())
}

fn normal(app: &mut App, key: KeyEvent) {
    if app.view == View::Scan && scan_keys(app, key) {
        return;
    }
    match key.code {
        KeyCode::Char('q') | KeyCode::Esc => app.should_quit = true,
        KeyCode::Down | KeyCode::Char('j') => app.next(),
        KeyCode::Up | KeyCode::Char('k') => app.prev(),
        KeyCode::Enter | KeyCode::Char(' ') => app.toggle(),
        KeyCode::Tab => app.cycle_view(),
        KeyCode::Char('r') => app.manual_refresh(),
        KeyCode::Char('m') => app.load_module(),
        KeyCode::Char('s') => app.start_scan(),
        KeyCode::Char('t') => app.check_trace(),
        KeyCode::Char('e') => app.request_edit(),
        KeyCode::Char('i') => app.start_import(),
        KeyCode::Char('d') => app.ask_delete(),
        _ => {}
    }
}

/// Returns true if the key was consumed by the scan view.
fn scan_keys(app: &mut App, key: KeyEvent) -> bool {
    match key.code {
        KeyCode::Down | KeyCode::Char('j') => app.scan_move(1),
        KeyCode::Up | KeyCode::Char('k') => app.scan_move(-1),
        KeyCode::Enter => app.apply_scan_choice(),
        KeyCode::Char('x') => app.cancel_scan(),
        KeyCode::Esc | KeyCode::Tab => app.view = View::Info,
        _ => return false,
    }
    true
}

fn import(app: &mut App, key: KeyEvent) {
    let Mode::Import(buf) = &mut app.mode else { return };
    match key.code {
        KeyCode::Esc => app.mode = Mode::Normal,
        KeyCode::Backspace => {
            buf.pop();
        }
        KeyCode::Char(c) => buf.push(c),
        KeyCode::Enter => {
            let path = std::mem::take(buf);
            app.mode = Mode::Normal;
            app.import(&path);
        }
        _ => {}
    }
}

fn confirm_delete(app: &mut App, key: KeyEvent) {
    let Mode::ConfirmDelete(name) = &app.mode else { return };
    let name = name.clone();
    app.mode = Mode::Normal;
    if matches!(key.code, KeyCode::Char('y') | KeyCode::Char('Y')) {
        app.delete_confirmed(&name);
    }
}
