use ratatui::prelude::*;
use ratatui::widgets::{Block, Borders, List, ListItem, Paragraph, Wrap};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(1), // title
            Constraint::Min(0),   // list + detail
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    // Title
    let title = Paragraph::new("  Activity — Subagents  [Tab] next  [r] refresh  [Enter] expand  [Esc] back")
        .style(Style::default().fg(Color::DarkGray));
    f.render_widget(title, chunks[0]);

    if app.subagents.is_empty() {
        let empty = Paragraph::new("\n  No subagents spawned yet.\n\n  Use session_spawn in chat to create background tasks.")
            .style(Style::default().fg(Color::DarkGray))
            .block(Block::default().borders(Borders::ALL).title(" Subagents "));
        f.render_widget(empty, chunks[1]);
    } else {
        // Split into list and detail
        let body = Layout::default()
            .direction(Direction::Horizontal)
            .constraints([Constraint::Percentage(45), Constraint::Percentage(55)])
            .split(chunks[1]);

        // Subagent list
        let items: Vec<ListItem> = app
            .subagents
            .iter()
            .enumerate()
            .map(|(i, agent)| {
                let icon = match agent.status {
                    bat_types::session::SubagentStatus::Running => "⏳",
                    bat_types::session::SubagentStatus::WaitingForAnswer => "❓",
                    bat_types::session::SubagentStatus::Paused => "⏸ ",
                    bat_types::session::SubagentStatus::Completed => "✅",
                    bat_types::session::SubagentStatus::Failed => "❌",
                    bat_types::session::SubagentStatus::Cancelled => "⏹ ",
                };
                let style = if i == app.activity_cursor {
                    Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::White)
                };
                let prefix = if i == app.activity_cursor { "▸ " } else { "  " };
                ListItem::new(format!("{prefix}{icon} {}", agent.label)).style(style)
            })
            .collect();

        let list = List::new(items)
            .block(Block::default().borders(Borders::ALL).title(" Subagents "));
        f.render_widget(list, body[0]);

        // Detail pane
        if let Some(agent) = app.subagents.get(app.activity_cursor) {
            let status_color = match agent.status {
                bat_types::session::SubagentStatus::Running => Color::Blue,
                bat_types::session::SubagentStatus::WaitingForAnswer => Color::Yellow,
                bat_types::session::SubagentStatus::Paused => Color::Magenta,
                bat_types::session::SubagentStatus::Completed => Color::Green,
                bat_types::session::SubagentStatus::Failed => Color::Red,
                bat_types::session::SubagentStatus::Cancelled => Color::DarkGray,
            };

            let mut lines = vec![
                Line::from(vec![
                    Span::styled("Status: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(agent.status.to_string(), Style::default().fg(status_color).add_modifier(Modifier::BOLD)),
                ]),
                Line::from(vec![
                    Span::styled("Key: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&agent.session_key, Style::default().fg(Color::Yellow)),
                ]),
                Line::from(vec![
                    Span::styled("Started: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(&agent.started_at, Style::default().fg(Color::White)),
                ]),
                Line::from(""),
                Line::from(Span::styled("Task:", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))),
            ];

            // Word-wrap the task text
            for line in agent.task.lines() {
                lines.push(Line::from(Span::styled(format!("  {line}"), Style::default().fg(Color::White))));
            }

            if let Some(ref summary) = agent.summary {
                lines.push(Line::from(""));
                lines.push(Line::from(Span::styled("Summary:", Style::default().fg(Color::DarkGray).add_modifier(Modifier::BOLD))));
                for line in summary.lines() {
                    lines.push(Line::from(Span::styled(format!("  {line}"), Style::default().fg(Color::Green))));
                }
            }

            if agent.token_input + agent.token_output > 0 {
                lines.push(Line::from(""));
                lines.push(Line::from(vec![
                    Span::styled("Tokens: ", Style::default().fg(Color::DarkGray)),
                    Span::styled(
                        format!("{} in / {} out", agent.token_input, agent.token_output),
                        Style::default().fg(Color::White),
                    ),
                ]));
            }

            let detail = Paragraph::new(lines)
                .wrap(Wrap { trim: false })
                .block(Block::default().borders(Borders::ALL).title(" Details "));
            f.render_widget(detail, body[1]);
        }
    }

    // Status bar
    let running = app.subagents.iter().filter(|s| s.status == bat_types::session::SubagentStatus::Running).count();
    let total = app.subagents.len();
    let status = format!("  {total} subagent(s)  |  {running} running");
    let bar = Paragraph::new(status)
        .style(Style::default().fg(Color::DarkGray).bg(Color::Rgb(30, 30, 30)));
    f.render_widget(bar, chunks[2]);
}
