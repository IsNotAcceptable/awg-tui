use ratatui::layout::{Constraint, Flex, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Clear, Paragraph};
use ratatui::Frame;

use super::theme;
use crate::app::{App, Mode};

pub fn draw(frame: &mut Frame, app: &App) {
    let (title, lines) = match &app.mode {
        Mode::Normal => return,
        Mode::Import(buf) => (
            " Import config ",
            vec![
                Line::from("Path to a .conf file (file name = interface name):"),
                Line::from(vec![Span::raw(buf.clone()), Span::styled("█", theme::accent())]),
                Line::from(Span::styled("Enter import · Esc cancel", theme::label())),
            ],
        ),
        Mode::ConfirmDelete(name) => (
            " Delete ",
            vec![
                Line::from(format!("Delete config '{name}'?")),
                Line::from(Span::styled("y yes · any other key cancel", theme::label())),
            ],
        ),
    };

    let area = centered(frame.area(), 70, lines.len() as u16 + 2);
    frame.render_widget(Clear, area);
    frame.render_widget(
        Paragraph::new(lines).block(Block::bordered().title(title).border_style(theme::accent())),
        area,
    );
}

fn centered(area: Rect, width_pct: u16, height: u16) -> Rect {
    let [v] = Layout::vertical([Constraint::Length(height)])
        .flex(Flex::Center)
        .areas(area);
    let [h] = Layout::horizontal([Constraint::Percentage(width_pct)])
        .flex(Flex::Center)
        .areas(v);
    h
}
