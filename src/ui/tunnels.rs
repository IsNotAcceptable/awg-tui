use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, List, ListItem, ListState};
use ratatui::Frame;

use super::theme;
use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let items: Vec<ListItem> = app
        .tunnels
        .iter()
        .map(|t| {
            let (mark, style) = if t.is_up() {
                ("●", theme::up())
            } else {
                ("○", theme::down())
            };
            ListItem::new(Line::from(vec![
                Span::styled(format!("{mark} "), style),
                Span::raw(t.name.clone()),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::bordered().title(" Tunnels "))
        .highlight_style(theme::selected())
        .highlight_symbol("▶ ");

    let mut state = ListState::default();
    if !app.tunnels.is_empty() {
        state.select(Some(app.selected));
    }
    frame.render_stateful_widget(list, area, &mut state);
}
