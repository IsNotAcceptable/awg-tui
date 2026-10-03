use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Sparkline, Wrap};
use ratatui::Frame;

use super::{scan, theme};
use crate::app::{App, Tunnel, View};
use crate::fmt;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    match app.view {
        View::Scan => scan::draw(frame, area, app),
        View::Config => text(frame, area, " Config (secrets masked) ", &app.config_text, false),
        View::Log => text(frame, area, " Kernel log ", &app.log_text, true),
        View::Info => info(frame, area, app),
    }
}

/// Plain text pane; `tail` keeps the last lines visible (logs).
fn text(frame: &mut Frame, area: Rect, title: &str, body: &str, tail: bool) {
    let total = body.lines().count() as u16;
    let visible = area.height.saturating_sub(2);
    let skip = if tail { total.saturating_sub(visible) } else { 0 };
    let lines: Vec<Line> = body.lines().map(Line::from).collect();
    frame.render_widget(
        Paragraph::new(lines)
            .block(Block::bordered().title(title.to_string()))
            .scroll((skip, 0))
            .wrap(Wrap { trim: false }),
        area,
    );
}

fn info(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::bordered().title(" Status ");
    let Some(t) = app.selected_tunnel() else {
        let msg = format!("No tunnels found in {}  (press i to import)", app.opts.config_dir.display());
        frame.render_widget(Paragraph::new(msg).block(block), area);
        return;
    };

    // live tunnels get two throughput sparklines at the bottom
    let (top, graphs) = if t.is_up() {
        let [a, b] = Layout::vertical([Constraint::Min(6), Constraint::Length(6)]).areas(area);
        (a, Some(b))
    } else {
        (area, None)
    };

    frame.render_widget(
        Paragraph::new(info_lines(app, t)).block(block).wrap(Wrap { trim: false }),
        top,
    );

    if let (Some(graphs), Some(h)) = (graphs, app.history.get(&t.name)) {
        let [l, r] = Layout::horizontal([Constraint::Percentage(50), Constraint::Percentage(50)])
            .areas(graphs);
        let spark = |title: &str, data: &std::collections::VecDeque<u64>, style| {
            Sparkline::default()
                .block(Block::bordered().title(title.to_string()))
                .style(style)
                .data(data.iter().copied())
        };
        frame.render_widget(spark(" ↓ rx ", &h.rx, theme::up()), l);
        frame.render_widget(spark(" ↑ tx ", &h.tx, theme::accent()), r);
    }
}

fn kv(key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<12}"), theme::label()),
        Span::raw(value),
    ])
}

fn clip(s: &str, max: usize) -> String {
    if s.chars().count() > max {
        let head: String = s.chars().take(max).collect();
        format!("{head}…")
    } else {
        s.to_string()
    }
}

fn info_lines(app: &App, t: &Tunnel) -> Vec<Line<'static>> {
    let Some(live) = &t.live else {
        return vec![
            Line::from(Span::styled(format!("{} is down", t.name), theme::down())),
            Line::default(),
            Line::from("Press Enter to bring it up."),
        ];
    };

    let mut out = vec![
        Line::from(Span::styled(format!("{} is up", t.name), theme::up())),
        Line::default(),
        kv("public key", live.public_key.clone()),
        kv(
            "listen port",
            live.listen_port.map_or("-".into(), |p| p.to_string()),
        ),
    ];

    let (rx, tx) = live.totals();
    out.push(kv("total", format!("↓ {}  ↑ {}", fmt::bytes(rx), fmt::bytes(tx))));
    if let Some(&(rrx, rtx)) = app.rates.get(&t.name) {
        out.push(kv("speed", format!("↓ {}  ↑ {}", fmt::rate(rrx), fmt::rate(rtx))));
    }

    if !live.params.is_empty() {
        out.push(Line::default());
        out.push(Line::from(Span::styled("Obfuscation", theme::accent())));
        for (k, v) in &live.params {
            out.push(kv(k, clip(v, 48)));
        }
    }

    for (i, p) in live.peers.iter().enumerate() {
        out.push(Line::default());
        out.push(Line::from(Span::styled(
            format!("Peer #{} {}", i + 1, fmt::short_key(&p.public_key)),
            theme::accent(),
        )));
        out.push(kv("endpoint", p.endpoint.clone().unwrap_or_else(|| "-".into())));
        out.push(kv("allowed ips", p.allowed_ips.clone()));
        out.push(kv(
            "handshake",
            p.latest_handshake.map_or("never".into(), fmt::ago),
        ));
        out.push(kv(
            "transfer",
            format!("↓ {}  ↑ {}", fmt::bytes(p.rx), fmt::bytes(p.tx)),
        ));
        if let Some(k) = p.keepalive {
            out.push(kv("keepalive", format!("{k}s")));
        }
    }
    out
}
