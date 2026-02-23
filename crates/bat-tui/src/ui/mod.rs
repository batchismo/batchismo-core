mod activity;
mod chat;
mod help;
mod logs;
mod memory;
mod onboarding;
mod settings;

use ratatui::prelude::*;

use crate::app::{App, Screen};

/// Main render function — dispatches to the active screen.
pub fn render(f: &mut Frame, app: &App) {
    match app.screen {
        Screen::Onboarding => onboarding::render(f, app),
        Screen::Chat => chat::render(f, app),
        Screen::Settings => settings::render(f, app),
        Screen::Logs => logs::render(f, app),
        Screen::Memory => memory::render(f, app),
        Screen::Activity => activity::render(f, app),
    }

    // Help overlay on top of everything
    if app.show_help {
        help::render(f);
    }
}
