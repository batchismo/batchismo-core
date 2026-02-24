use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Paragraph, Row, Table},
};

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(5), // summary cards
            Constraint::Min(1),   // tables
            Constraint::Length(1), // status
        ])
        .split(f.area());

    match &app.usage_stats {
        Some(stats) => {
            render_summary(f, stats, chunks[0]);
            render_tables(f, stats, chunks[1]);
        }
        None => {
            let msg = Paragraph::new("Loading usage data...")
                .alignment(Alignment::Center)
                .block(Block::default().borders(Borders::ALL).title(" Usage "));
            f.render_widget(msg, chunks[0]);
        }
    }

    let status = Paragraph::new(" Tab: next screen │ Esc: chat │ r: refresh")
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));
    f.render_widget(status, chunks[2]);
}

fn render_summary(f: &mut Frame, stats: &bat_types::usage::UsageStats, area: Rect) {
    let cols = Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage(33),
            Constraint::Percentage(34),
            Constraint::Percentage(33),
        ])
        .split(area);

    let total = stats.total_input + stats.total_output;
    let tokens = Paragraph::new(vec![
        Line::from(Span::styled("Total Tokens", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(format_num(total), Style::default().fg(Color::White).add_modifier(Modifier::BOLD))),
        Line::from(Span::styled(
            format!("{} in / {} out", format_num(stats.total_input), format_num(stats.total_output)),
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Center);

    let cost = Paragraph::new(vec![
        Line::from(Span::styled("Est. Cost", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            format!("${:.4}", stats.estimated_cost_usd),
            Style::default().fg(Color::Green).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled("Anthropic pricing", Style::default().fg(Color::DarkGray))),
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Center);

    let sessions = Paragraph::new(vec![
        Line::from(Span::styled("Sessions", Style::default().fg(Color::DarkGray))),
        Line::from(Span::styled(
            stats.sessions.len().to_string(),
            Style::default().fg(Color::White).add_modifier(Modifier::BOLD),
        )),
        Line::from(Span::styled(
            format!("{} model(s)", stats.by_model.len()),
            Style::default().fg(Color::DarkGray),
        )),
    ])
    .block(Block::default().borders(Borders::ALL))
    .alignment(Alignment::Center);

    f.render_widget(tokens, cols[0]);
    f.render_widget(cost, cols[1]);
    f.render_widget(sessions, cols[2]);
}

fn render_tables(f: &mut Frame, stats: &bat_types::usage::UsageStats, area: Rect) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage(40),
            Constraint::Percentage(60),
        ])
        .split(area);

    // By Model
    let model_rows: Vec<Row> = stats.by_model.iter().map(|m| {
        Row::new(vec![
            m.model.clone(),
            format_num(m.token_input),
            format_num(m.token_output),
            m.session_count.to_string(),
        ])
    }).collect();

    let model_table = Table::new(
        model_rows,
        [
            Constraint::Percentage(40),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
            Constraint::Percentage(20),
        ],
    )
    .header(Row::new(vec!["Model", "Input", "Output", "Sessions"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
    .block(Block::default().borders(Borders::ALL).title(" By Model "));

    f.render_widget(model_table, chunks[0]);

    // By Session
    let session_rows: Vec<Row> = stats.sessions.iter().map(|s| {
        Row::new(vec![
            s.key.clone(),
            s.model.clone(),
            format_num(s.token_input),
            format_num(s.token_output),
            s.message_count.to_string(),
        ])
    }).collect();

    let session_table = Table::new(
        session_rows,
        [
            Constraint::Percentage(25),
            Constraint::Percentage(25),
            Constraint::Percentage(17),
            Constraint::Percentage(17),
            Constraint::Percentage(16),
        ],
    )
    .header(Row::new(vec!["Session", "Model", "Input", "Output", "Msgs"])
        .style(Style::default().fg(Color::Yellow).add_modifier(Modifier::BOLD)))
    .block(Block::default().borders(Borders::ALL).title(" By Session "));

    f.render_widget(session_table, chunks[1]);
}

fn format_num(n: i64) -> String {
    if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}
