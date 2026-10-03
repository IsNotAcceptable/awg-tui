use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use super::theme;
use crate::app::App;
use crate::kmod;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let module = if app.kmod_loaded {
        Span::styled(format!("module {}: loaded", kmod::MODULE), theme::up())
    } else {
        Span::styled(
            format!("module {}: NOT detected (userspace?)", kmod::MODULE),
            theme::warn(),
        )
    };
    let priv_ = if app.awg.is_root() {
        Span::styled("root", theme::up())
    } else {
        Span::styled("via sudo -n", theme::warn())
    };

    let line = Line::from(vec![
        module,
        Span::raw("  │  "),
        priv_,
        Span::raw("  │  "),
        Span::styled(app.opts.config_dir.display().to_string(), theme::label()),
    ]);

    let block = Block::bordered().title(Span::styled(" awg-tui ", theme::accent()));
    frame.render_widget(Paragraph::new(line).block(block), area);
}
