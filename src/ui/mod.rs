mod details;
mod footer;
mod header;
mod theme;
mod tunnels;

use ratatui::layout::{Constraint, Layout};
use ratatui::Frame;

use crate::app::App;

pub fn draw(frame: &mut Frame, app: &App) {
    let [head, body, foot] = Layout::vertical([
        Constraint::Length(3),
        Constraint::Min(5),
        Constraint::Length(1),
    ])
    .areas(frame.area());

    let [left, right] =
        Layout::horizontal([Constraint::Percentage(30), Constraint::Percentage(70)]).areas(body);

    header::draw(frame, head, app);
    tunnels::draw(frame, left, app);
    details::draw(frame, right, app);
    footer::draw(frame, foot, app);
}
