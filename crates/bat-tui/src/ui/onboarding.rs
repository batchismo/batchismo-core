use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Center the content
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(2),
            Constraint::Length(area.height.min(20)),
            Constraint::Min(2),
        ])
        .split(area);

    let inner = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Min(4),
            Constraint::Length(area.width.min(60)),
            Constraint::Min(4),
        ])
        .split(outer[1]);

    let content_area = inner[1];

    // Progress bar
    let progress = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Length(1), Constraint::Min(1)])
        .split(content_area);

    let dots: Vec<Span> = (0..5)
        .map(|i| {
            if i <= app.onboarding_step {
                Span::styled(" ● ", Style::default().fg(Color::Cyan))
            } else {
                Span::styled(" ○ ", Style::default().fg(Color::DarkGray))
            }
        })
        .collect();
    let progress_line = Paragraph::new(Line::from(dots)).alignment(Alignment::Center);
    f.render_widget(progress_line, progress[0]);

    let block = Block::default()
        .borders(Borders::ALL)
        .border_style(Style::default().fg(Color::DarkGray))
        .title_alignment(Alignment::Center);

    match app.onboarding_step {
        0 => render_welcome(f, progress[1], block),
        1 => render_api_key(f, progress[1], block, app),
        2 => render_name(f, progress[1], block, app),
        3 => render_access(f, progress[1], block, app),
        4 => render_ready(f, progress[1], block, app),
        _ => {}
    }
}

fn render_welcome(f: &mut Frame, area: Rect, block: Block) {
    let lines = vec![
        Line::from(""),
        Line::from(Span::styled("⚡ Welcome to Batchismo", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled("Your AI that actually works on your computer.", Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(Span::styled("Let's get you set up. It only takes a minute.", Style::default().fg(Color::DarkGray))),
        Line::from(""),
        Line::from(""),
        Line::from(Span::styled("Press Enter to get started", Style::default().fg(Color::Yellow))),
    ];
    let p = Paragraph::new(lines).block(block.title(" Welcome ")).alignment(Alignment::Center).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_api_key(f: &mut Frame, area: Rect, block: Block, app: &App) {
    let key_display = if app.onboarding_api_key.is_empty() {
        Span::styled("(not set)", Style::default().fg(Color::DarkGray))
    } else if app.onboarding_editing {
        Span::styled(&app.edit_buffer, Style::default().fg(Color::White))
    } else {
        let masked = format!("{}...{}", &app.onboarding_api_key[..8.min(app.onboarding_api_key.len())],
            &app.onboarding_api_key[app.onboarding_api_key.len().saturating_sub(4)..]);
        Span::styled(masked, Style::default().fg(Color::White))
    };

    let status = if app.onboarding_validated {
        Span::styled(" ✓ Valid", Style::default().fg(Color::Green))
    } else if !app.onboarding_error.is_empty() {
        Span::styled(format!(" ✗ {}", app.onboarding_error), Style::default().fg(Color::Red))
    } else {
        Span::raw("")
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("Enter your Anthropic API key", Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(vec![Span::raw("  Key: "), key_display]),
        Line::from(vec![Span::raw("  "), status]),
        Line::from(""),
    ];

    if app.onboarding_editing {
        lines.push(Line::from(Span::styled("  Type your key, press Enter when done", Style::default().fg(Color::Yellow))));
    } else {
        let mut hints = vec![
            Line::from(Span::styled("  e: edit  ", Style::default().fg(Color::Yellow))),
        ];
        if !app.onboarding_api_key.is_empty() && !app.onboarding_validated {
            hints.push(Line::from(Span::styled("  v: validate", Style::default().fg(Color::Yellow))));
        }
        if app.onboarding_validated {
            hints.push(Line::from(Span::styled("  Enter: next", Style::default().fg(Color::Yellow))));
        }
        hints.push(Line::from(Span::styled("  Esc: back", Style::default().fg(Color::DarkGray))));
        lines.extend(hints);
    }

    let p = Paragraph::new(lines).block(block.title(" API Key ")).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_name(f: &mut Frame, area: Rect, block: Block, app: &App) {
    let name_display = if app.onboarding_editing {
        Span::styled(&app.edit_buffer, Style::default().fg(Color::White))
    } else if app.onboarding_name.is_empty() {
        Span::styled("(not set)", Style::default().fg(Color::DarkGray))
    } else {
        Span::styled(&app.onboarding_name, Style::default().fg(Color::Cyan))
    };

    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("Give your agent a name", Style::default().fg(Color::White))),
        Line::from(""),
        Line::from(vec![Span::raw("  Name: "), name_display]),
        Line::from(""),
    ];

    if !app.onboarding_name.is_empty() && !app.onboarding_editing {
        lines.push(Line::from(Span::styled(
            format!("  Meet {}, your personal AI agent.", app.onboarding_name),
            Style::default().fg(Color::DarkGray),
        )));
        lines.push(Line::from(""));
    }

    if app.onboarding_editing {
        lines.push(Line::from(Span::styled("  Type a name, press Enter when done", Style::default().fg(Color::Yellow))));
    } else {
        lines.push(Line::from(Span::styled("  e: edit   Enter: next   Esc: back", Style::default().fg(Color::Yellow))));
    }

    let p = Paragraph::new(lines).block(block.title(" Name Your Agent ")).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_access(f: &mut Frame, area: Rect, block: Block, app: &App) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("What folders can your agent work with?", Style::default().fg(Color::White))),
        Line::from(Span::styled("Your agent cannot touch anything else.", Style::default().fg(Color::DarkGray))),
        Line::from(""),
    ];

    if app.onboarding_folders.is_empty() {
        lines.push(Line::from(Span::styled("  No folders added yet.", Style::default().fg(Color::DarkGray))));
    } else {
        for (path, access, _) in &app.onboarding_folders {
            let access_style = if access == "read-write" {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::Yellow)
            };
            lines.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(path, Style::default().fg(Color::White)),
                Span::raw(" ["),
                Span::styled(access, access_style),
                Span::raw("]"),
            ]));
        }
    }

    lines.push(Line::from(""));

    if app.onboarding_editing {
        lines.push(Line::from(vec![
            Span::raw("  Path: "),
            Span::styled(&app.edit_buffer, Style::default().fg(Color::White)),
        ]));
        lines.push(Line::from(Span::styled("  Type path, press Enter to add", Style::default().fg(Color::Yellow))));
    } else {
        let mut hints = String::from("  a: add folder   d: remove last");
        if !app.onboarding_folders.is_empty() {
            hints.push_str("   Enter: next");
        }
        hints.push_str("   Esc: back");
        lines.push(Line::from(Span::styled(hints, Style::default().fg(Color::Yellow))));
    }

    let p = Paragraph::new(lines).block(block.title(" Set Up Access ")).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}

fn render_ready(f: &mut Frame, area: Rect, block: Block, app: &App) {
    let mut lines = vec![
        Line::from(""),
        Line::from(Span::styled("✓ You're all set!", Style::default().fg(Color::Green).add_modifier(Modifier::BOLD))),
        Line::from(""),
        Line::from(Span::styled(
            format!("{} is ready to help.", app.onboarding_name),
            Style::default().fg(Color::White),
        )),
        Line::from(""),
    ];

    if !app.onboarding_error.is_empty() {
        lines.push(Line::from(Span::styled(&app.onboarding_error, Style::default().fg(Color::Red))));
        lines.push(Line::from(""));
    }

    lines.push(Line::from(Span::styled("  Press Enter to start chatting", Style::default().fg(Color::Yellow))));
    lines.push(Line::from(Span::styled("  Esc: back", Style::default().fg(Color::DarkGray))));

    let p = Paragraph::new(lines).block(block.title(" Ready ")).alignment(Alignment::Center).wrap(Wrap { trim: false });
    f.render_widget(p, area);
}
