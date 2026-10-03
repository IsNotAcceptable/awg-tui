use std::path::Path;
use std::process::Command;

use crate::app::App;

/// Suspends the TUI, opens the config in `$VISUAL`/`$EDITOR` (via `sudoedit`
/// when not root) and returns; the caller re-initialises the terminal.
pub fn edit(app: &mut App, path: &Path) {
    ratatui::restore();

    let status = if app.awg.is_root() {
        let editor = ["VISUAL", "EDITOR"]
            .iter()
            .find_map(|k| std::env::var(k).ok())
            .unwrap_or_else(|| "vi".into());
        let mut parts = editor.split_whitespace();
        let program = parts.next().unwrap_or("vi").to_string();
        Command::new(program).args(parts).arg(path).status()
    } else {
        Command::new("sudoedit").arg(path).status()
    };

    match status {
        Ok(s) if s.success() => {
            app.set_info("config saved — toggle the tunnel (Enter twice) to apply changes")
        }
        Ok(_) => app.set_error("editor exited with an error"),
        Err(e) => app.set_error(format!("cannot start editor: {e}")),
    }
}
