use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, List, ListItem, Paragraph, Scrollbar, ScrollbarOrientation, ScrollbarState, Wrap},
};

use bat_types::message::Role;

use crate::app::App;

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Min(1),    // messages
            Constraint::Length(3), // input
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    render_messages(f, app, chunks[0]);
    render_input(f, app, chunks[1]);
    render_status_bar(f, app, chunks[2]);

    // Session switcher overlay
    if app.show_session_switcher {
        render_session_switcher(f, app);
    }
}

fn render_messages(f: &mut Frame, app: &App, area: Rect) {
    let mut lines: Vec<Line> = Vec::new();

    for msg in &app.messages {
        let (prefix, style) = match msg.role {
            Role::User => ("You", Style::default().fg(Color::Cyan)),
            Role::Assistant => (
                &*app.gateway.get_config().agent.name,
                Style::default().fg(Color::Green),
            ),
            Role::System => ("System", Style::default().fg(Color::Yellow)),
        };

        // Role header
        lines.push(Line::from(Span::styled(
            format!("─── {prefix} ───"),
            style.add_modifier(Modifier::BOLD),
        )));

        // Message content
        for text_line in msg.content.lines() {
            lines.push(Line::from(text_line.to_string()));
        }

        // Tool calls
        for tc in &msg.tool_calls {
            lines.push(Line::from(Span::styled(
                format!("  🔧 {}: {}", tc.name, tc.id),
                Style::default().fg(Color::DarkGray),
            )));
        }

        lines.push(Line::from("")); // spacer
    }

    // Streaming text (if active)
    if app.is_streaming && !app.streaming_text.is_empty() {
        let name = &app.gateway.get_config().agent.name;
        lines.push(Line::from(Span::styled(
            format!("─── {name} ───"),
            Style::default()
                .fg(Color::Green)
                .add_modifier(Modifier::BOLD),
        )));
        for text_line in app.streaming_text.lines() {
            lines.push(Line::from(text_line.to_string()));
        }
        lines.push(Line::from(Span::styled(
            "▊",
            Style::default().fg(Color::Green),
        )));
    } else if app.is_streaming {
        lines.push(Line::from(Span::styled(
            "⏳ Thinking…",
            Style::default().fg(Color::DarkGray).add_modifier(Modifier::ITALIC),
        )));
    }

    let total_lines = lines.len();
    let visible = area.height as usize;

    // Calculate scroll: offset 0 = bottom (most recent)
    let max_scroll = total_lines.saturating_sub(visible);
    let scroll_pos = max_scroll.saturating_sub(app.scroll_offset.min(max_scroll));

    let messages_widget = Paragraph::new(lines)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Batchismo ")
                .title_alignment(Alignment::Center),
        )
        .wrap(Wrap { trim: false })
        .scroll((scroll_pos as u16, 0));

    f.render_widget(messages_widget, area);

    // Scrollbar
    if total_lines > visible {
        let mut scrollbar_state =
            ScrollbarState::new(total_lines).position(scroll_pos);
        f.render_stateful_widget(
            Scrollbar::new(ScrollbarOrientation::VerticalRight),
            area,
            &mut scrollbar_state,
        );
    }
}

fn render_input(f: &mut Frame, app: &App, area: Rect) {
    let input_widget = Paragraph::new(app.input.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Message (Enter to send, Tab for settings) "),
        )
        .style(Style::default().fg(Color::White));

    f.render_widget(input_widget, area);

    // Show cursor in input
    f.set_cursor_position(Position::new(
        area.x + app.input.len() as u16 + 1,
        area.y + 1,
    ));
}

const CONTEXT_LIMIT: i64 = 200_000;

fn token_color(total: i64) -> Color {
    let ratio = total as f64 / CONTEXT_LIMIT as f64;
    if ratio < 0.5 {
        Color::Green
    } else if ratio < 0.75 {
        Color::Yellow
    } else if ratio < 0.9 {
        Color::Rgb(255, 165, 0) // orange
    } else {
        Color::Red
    }
}

fn render_status_bar(f: &mut Frame, app: &App, area: Rect) {
    let cfg = app.gateway.get_config();
    let streaming = if app.is_streaming { " 🔄 streaming" } else { "" };

    // Sum up token usage from messages
    let (total_in, total_out) = app.messages.iter().fold((0i64, 0i64), |(i, o), msg| {
        (i + msg.token_input.unwrap_or(0), o + msg.token_output.unwrap_or(0))
    });
    let total = total_in + total_out;
    let color = token_color(total);

    let token_str = format!(
        "in: {} · out: {} · total: {} / {}",
        total_in, total_out, total, CONTEXT_LIMIT
    );

    let active_key = app.gateway.active_session_key();
    let session_indicator = if active_key != "main" {
        format!(" │ 📂 {active_key}")
    } else {
        String::new()
    };

    let spans = vec![
        Span::styled(
            format!(" 🤖 {} │ {} │ ", cfg.agent.name, cfg.agent.model),
            Style::default().fg(Color::White),
        ),
        Span::styled(token_str, Style::default().fg(color)),
        Span::styled(
            format!("{streaming}{session_indicator} │ Ctrl+S sessions │ ? help"),
            Style::default().fg(Color::White),
        ),
    ];

    let bar = Paragraph::new(Line::from(spans))
        .style(Style::default().bg(Color::DarkGray));

    f.render_widget(bar, area);
}

fn render_session_switcher(f: &mut Frame, app: &App) {
    let area = f.area();
    // Center a popup
    let width = 50u16.min(area.width.saturating_sub(4));
    let height = (app.session_list.len() as u16 + 6).min(area.height.saturating_sub(4)).max(8);
    let x = (area.width.saturating_sub(width)) / 2;
    let y = (area.height.saturating_sub(height)) / 2;
    let popup_area = Rect::new(x, y, width, height);

    f.render_widget(Clear, popup_area);

    let active_key = app.gateway.active_session_key();

    if app.session_creating {
        // New session input
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" New Session ")
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let prompt = Paragraph::new(vec![
            Line::from(""),
            Line::from(Span::styled("Session name:", Style::default().fg(Color::DarkGray))),
            Line::from(format!("{}_", app.session_new_name)),
            Line::from(""),
            Line::from(Span::styled("Enter: create │ Esc: cancel", Style::default().fg(Color::DarkGray))),
        ]);
        f.render_widget(prompt, inner);
    } else {
        let block = Block::default()
            .borders(Borders::ALL)
            .title(" Sessions (Ctrl+S to close) ")
            .border_style(Style::default().fg(Color::Cyan));
        let inner = block.inner(popup_area);
        f.render_widget(block, popup_area);

        let chunks = Layout::default()
            .direction(Direction::Vertical)
            .constraints([Constraint::Min(1), Constraint::Length(2)])
            .split(inner);

        let items: Vec<ListItem> = app.session_list.iter().enumerate().map(|(i, s)| {
            let active = if s.key == active_key { " ● " } else { "   " };
            let tokens = s.token_input + s.token_output;
            let token_str = if tokens > 0 { format!(" ({tokens}t)") } else { String::new() };
            let style = if i == app.session_cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else if s.key == active_key {
                Style::default().fg(Color::Green)
            } else {
                Style::default().fg(Color::White)
            };
            ListItem::new(format!("{active}{}{token_str}", s.key)).style(style)
        }).collect();

        let list = List::new(items);
        f.render_widget(list, chunks[0]);

        let help = Paragraph::new(Line::from(Span::styled(
            " Enter: switch │ n: new │ d: delete │ Esc: close",
            Style::default().fg(Color::DarkGray),
        )));
        f.render_widget(help, chunks[1]);
    }
}
