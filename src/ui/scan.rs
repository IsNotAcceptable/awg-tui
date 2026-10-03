use ratatui::layout::{Constraint, Rect};
use ratatui::widgets::{Block, Paragraph, Row, Table, TableState};
use ratatui::Frame;

use super::theme;
use crate::app::App;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let Some(scan) = &app.scan else {
        frame.render_widget(
            Paragraph::new("Press s on a running tunnel to scan WARP endpoints.")
                .block(Block::bordered().title(" Endpoint scan ")),
            area,
        );
        return;
    };

    let sorted = scan.sorted();
    let title = format!(
        " Endpoint scan {}/{} {} ",
        scan.results.len(),
        scan.total,
        if scan.done { "· done" } else { "· running" }
    );

    let rows: Vec<Row> = sorted
        .iter()
        .map(|p| {
            let style = match p.loss {
                0 => theme::up(),
                100 => theme::down(),
                _ => theme::warn(),
            };
            Row::new([
                p.endpoint.clone(),
                format!("{}%", p.loss),
                p.rtt_ms.map_or("-".into(), |r| format!("{r:.0} ms")),
            ])
            .style(style)
        })
        .collect();

    let table = Table::new(
        rows,
        [Constraint::Length(24), Constraint::Length(6), Constraint::Min(8)],
    )
    .header(Row::new(["Endpoint", "Loss", "RTT"]).style(theme::accent()))
    .block(Block::bordered().title(title))
    .row_highlight_style(theme::selected());

    let mut state = TableState::default();
    if !sorted.is_empty() {
        state.select(Some(scan.selected.min(sorted.len() - 1)));
    }
    frame.render_stateful_widget(table, area, &mut state);
}
