use ratatui::style::{Color, Modifier, Style};

pub fn up() -> Style {
    Style::new().fg(Color::Green)
}

pub fn down() -> Style {
    Style::new().fg(Color::DarkGray)
}

pub fn warn() -> Style {
    Style::new().fg(Color::Yellow)
}

pub fn error() -> Style {
    Style::new().fg(Color::Red)
}

pub fn accent() -> Style {
    Style::new().fg(Color::Cyan).add_modifier(Modifier::BOLD)
}

pub fn label() -> Style {
    Style::new().fg(Color::Gray)
}

pub fn selected() -> Style {
    Style::new().add_modifier(Modifier::REVERSED | Modifier::BOLD)
}
