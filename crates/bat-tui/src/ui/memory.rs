use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let area = f.area();

    // Layout: header + main + footer
    let outer = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // header
            Constraint::Min(1),   // main area
            Constraint::Length(1), // footer
        ])
        .split(area);

    // Header
    let header = Paragraph::new(Line::from(vec![
        Span::styled(" 🧠 Memory ", Style::default().fg(Color::Magenta).add_modifier(Modifier::BOLD)),
        Span::raw(format!("({} files)", app.memory_files.len())),
    ]));
    f.render_widget(header, outer[0]);

    // Main: left panel (files + stats) | right panel (content)
    let main = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(35),
            Constraint::Percentage(65),
        ])
        .split(outer[1]);

    render_file_list(f, app, main[0]);
    render_content(f, app, main[1]);

    // Footer
    let footer_text = if app.memory_editing {
        vec![
            Span::styled(" Esc", Style::default().fg(Color::Yellow)),
            Span::raw(":Cancel  "),
            Span::styled("Ctrl+S", Style::default().fg(Color::Yellow)),
            Span::raw(":Save"),
        ]
    } else {
        vec![
            Span::styled(" Tab", Style::default().fg(Color::Yellow)),
            Span::raw(":Chat  "),
            Span::styled("↑↓", Style::default().fg(Color::Yellow)),
            Span::raw(":Select  "),
            Span::styled("e", Style::default().fg(Color::Yellow)),
            Span::raw(":Edit  "),
            Span::styled("c", Style::default().fg(Color::Yellow)),
            Span::raw(":Consolidate  "),
            Span::styled("Esc", Style::default().fg(Color::Yellow)),
            Span::raw(":Back"),
        ]
    };
    let footer = Paragraph::new(Line::from(footer_text))
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(footer, outer[2]);
}

fn render_file_list(f: &mut Frame, app: &App, area: Rect) {
    // Split: file list on top, stats on bottom
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(4),    // file list
            Constraint::Length(10), // observation stats
        ])
        .split(area);

    // File list
    let items: Vec<ListItem> = app
        .memory_files
        .iter()
        .enumerate()
        .map(|(i, file)| {
            let selected = i == app.memory_cursor;
            let size = if file.size_bytes < 1024 {
                format!("{} B", file.size_bytes)
            } else {
                format!("{:.1} KB", file.size_bytes as f64 / 1024.0)
            };
            let style = if selected {
                Style::default().fg(Color::White).bg(Color::DarkGray).add_modifier(Modifier::BOLD)
            } else {
                Style::default().fg(Color::Gray)
            };
            ListItem::new(Line::from(vec![
                Span::styled(&file.name, style),
                Span::styled(format!("  {size}"), Style::default().fg(Color::DarkGray)),
            ]))
        })
        .collect();

    let list = List::new(items)
        .block(Block::default().borders(Borders::RIGHT).title(" Files "));
    f.render_widget(list, chunks[0]);

    // Observation stats
    let mut stats_lines: Vec<Line> = vec![
        Line::from(Span::styled(
            " Observations",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD),
        )),
    ];

    if let Some(ref summary) = app.memory_summary {
        stats_lines.push(Line::from(vec![
            Span::raw("  Total: "),
            Span::styled(
                summary.total_observations.to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]));
        stats_lines.push(Line::from(vec![
            Span::raw("  Sessions: "),
            Span::styled(
                summary.total_sessions.to_string(),
                Style::default().fg(Color::Cyan),
            ),
        ]));

        if !summary.top_tools.is_empty() {
            stats_lines.push(Line::from(Span::styled(
                "  Top Tools",
                Style::default().fg(Color::DarkGray),
            )));
            for (tool, count) in summary.top_tools.iter().take(3) {
                stats_lines.push(Line::from(vec![
                    Span::styled(format!("   {tool}"), Style::default().fg(Color::Green)),
                    Span::styled(format!(" {count}×"), Style::default().fg(Color::DarkGray)),
                ]));
            }
        }
    } else {
        stats_lines.push(Line::from(Span::styled(
            "  No data yet",
            Style::default().fg(Color::DarkGray),
        )));
    }

    // Consolidation status
    if app.memory_consolidating {
        stats_lines.push(Line::from(Span::styled(
            "  ⏳ Consolidating...",
            Style::default().fg(Color::Yellow),
        )));
    } else if !app.memory_consolidation_result.is_empty() {
        let color = if app.memory_consolidation_result.starts_with("Error") {
            Color::Red
        } else {
            Color::Green
        };
        stats_lines.push(Line::from(Span::styled(
            format!("  {}", truncate(&app.memory_consolidation_result, 30)),
            Style::default().fg(color),
        )));
    }

    let stats = Paragraph::new(stats_lines)
        .block(Block::default().borders(Borders::TOP | Borders::RIGHT));
    f.render_widget(stats, chunks[1]);
}

fn render_content(f: &mut Frame, app: &App, area: Rect) {
    let title = if let Some(file) = app.memory_files.get(app.memory_cursor) {
        if app.memory_editing {
            format!(" {} [EDITING] ", file.name)
        } else {
            format!(" {} ", file.name)
        }
    } else {
        " No file selected ".to_string()
    };

    let content = if app.memory_editing {
        &app.memory_edit_content
    } else {
        &app.memory_content
    };

    let style = if app.memory_editing {
        Style::default().fg(Color::White)
    } else {
        Style::default().fg(Color::Gray)
    };

    let paragraph = Paragraph::new(content.as_str())
        .style(style)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(title)
                .border_style(if app.memory_editing {
                    Style::default().fg(Color::Magenta)
                } else {
                    Style::default().fg(Color::DarkGray)
                }),
        )
        .wrap(Wrap { trim: false });
    f.render_widget(paragraph, area);
}

fn truncate(s: &str, max: usize) -> &str {
    if s.len() <= max { s } else { &s[..max] }
}
