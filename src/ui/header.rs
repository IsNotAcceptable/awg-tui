use ratatui::layout::Rect;
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Paragraph};
use ratatui::Frame;

use super::theme;
use crate::app::{App, TraceState};
use crate::kmod;

pub fn draw(frame: &mut Frame, area: Rect, app: &App) {
    let module = if app.kmod_loaded {
        Span::styled(format!("module {}: loaded", kmod::MODULE), theme::up())
    } else {
        Span::styled(
            format!("module {}: not detected (userspace?)", kmod::MODULE),
            theme::warn(),
        )
    };
    let privilege = if app.awg.is_root() {
        Span::styled("root", theme::up())
    } else {
        Span::styled("via sudo -n", theme::warn())
    };
    let exit = match &app.trace {
        TraceState::Idle => Span::styled("exit: press t to check", theme::label()),
        TraceState::Loading => Span::styled("exit: checking…", theme::warn()),
        TraceState::Failed(e) => Span::styled(format!("exit: {e}"), theme::error()),
        TraceState::Done(t) => {
            let style = if t.warp_active() { theme::up() } else { theme::warn() };
            Span::styled(format!("exit: {} {} warp={}", t.ip, t.loc, t.warp), style)
        }
    };

    let line = Line::from(vec![
        module,
        Span::raw("  │  "),
        privilege,
        Span::raw("  │  "),
        exit,
    ]);

    let block = Block::bordered().title(Span::styled(" awg-tui ", theme::accent()));
    frame.render_widget(Paragraph::new(line).block(block), area);
}
