mod chat;
mod help;
mod settings;

use ratatui::prelude::*;

use crate::app::{App, Screen};

/// Main render function — dispatches to the active screen.
pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Chat => chat::render(f, app),
        Screen::Settings => settings::render(f, app),
    }

    // Help overlay on top of everything
    if app.show_help {
        help::render(f);
    }
}
