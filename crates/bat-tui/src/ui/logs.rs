use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Layout: header + log list + footer
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(1),   // log entries
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 📋 Audit Log ", Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)),
        Span::raw(format!("({} entries)", app.audit_entries.len())),
    ]));
    f.render_widget(header, chunks[0]);

    // Log entries
    let visible_height = chunks[1].height as usize;
    let total = app.audit_entries.len();

    let lines: Vec<Line> = if total == 0 {
        vec![Line::from(Span::styled(
            "  No audit events yet. Send a message to generate some.",
            Style::default().fg(Color::DarkGray),
        ))]
    } else {
        // Show entries from bottom (newest) with scroll offset
        let end = total.saturating_sub(app.logs_scroll);
        let start = end.saturating_sub(visible_height);

        app.audit_entries[start..end]
            .iter()
            .map(|entry| {
                let style = if entry.contains("[ERROR]") {
                    Style::default().fg(Color::Red)
                } else if entry.contains("[WARN]") {
                    Style::default().fg(Color::Yellow)
                } else if entry.contains("[DEBUG]") {
                    Style::default().fg(Color::DarkGray)
                } else {
                    Style::default().fg(Color::Blue)
                };
                Line::from(Span::styled(format!("  {entry}"), style))
            })
            .collect()
    };

    let log_block = Paragraph::new(lines)
        .block(Block::default().borders(Borders::TOP))
        .wrap(Wrap { trim: false });
    f.render_widget(log_block, chunks[1]);

    // Footer
    let footer = Paragraph::new(Line::from(vec![
        Span::styled(" Tab", Style::default().fg(Color::Yellow)),
        Span::raw(":Chat  "),
        Span::styled("↑↓", Style::default().fg(Color::Yellow)),
        Span::raw(":Scroll  "),
        Span::styled("G", Style::default().fg(Color::Yellow)),
        Span::raw(":Bottom  "),
        Span::styled("g", Style::default().fg(Color::Yellow)),
        Span::raw(":Top  "),
        Span::styled("Esc", Style::default().fg(Color::Yellow)),
        Span::raw(":Back"),
    ]))
    .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, chunks[2]);
}
