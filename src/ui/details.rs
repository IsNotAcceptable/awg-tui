use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph, Wrap};
use ratatui::Frame;

use super::theme;
use crate::app::{App, Tunnel, View};
use crate::fmt;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let title = match app.view {
        View::Info => " Status ",
        View::Config => " Config (secrets masked) ",
    };
    let block = Block::bordered().title(title);

    let lines = match (app.selected_tunnel(), app.view) {
        (None, _) => vec![Line::from(format!(
            "No tunnels found in {}",
            app.opts.config_dir.display()
        ))],
        (Some(_), View::Config) => app.config_text.lines().map(Line::from).collect(),
        (Some(t), View::Info) => info_lines(app, t),
    };

    frame.render_widget(
        Paragraph::new(lines).block(block).wrap(Wrap { trim: false }),
        area,
    );
}

fn kv(key: &str, value: String) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!("{key:<12}"), theme::label()),
        Span::raw(value),
    ])
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
