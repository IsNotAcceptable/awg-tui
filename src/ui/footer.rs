use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use super::theme;
use crate::app::{App, Level};

const KEYS: &str = "↑↓/jk select  ⏎/space up/down  Tab info/config  m load module  r refresh  q quit";

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let line = match &app.status {
        Some(s) => {
            let style = match s.level {
                Level::Info => theme::up(),
                Level::Error => theme::error(),
            };
            Line::from(vec![
                Span::styled(s.text.clone(), style),
                Span::raw("  │  "),
                Span::styled("q quit", theme::label()),
            ])
        }
        None => Line::from(Span::styled(KEYS, theme::label())),
    };
    frame.render_widget(Paragraph::new(line), area);
}
