use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme;
use crate::app::{App, Level, View};

const KEYS: &str = "⏎ up/down  s scan  t check  e edit  i import  d delete  Tab view  m module  r refresh  q quit";
const SCAN_KEYS: &str = "↑↓ select  ⏎ apply & save  x stop  s rescan  Esc back  q quit";

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let hints = if app.view == View::Scan { SCAN_KEYS } else { KEYS };
    let line = match &app.status {
        Some(s) => {
            let style = match s.level {
                Level::Info => theme::up(),
                Level::Error => theme::error(),
            };
            Line::from(vec![
                Span::styled(s.text.clone(), style),
                Span::raw("  │  "),
                Span::styled(hints, theme::label()),
            ])
        }
        None => Line::from(Span::styled(hints, theme::label())),
    };
    frame.render_widget(Paragraph::new(line), area);
}
