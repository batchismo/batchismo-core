use ratatui::{
    prelude::*,
    widgets::{Block, Borders, List, ListItem, Paragraph, Tabs},
};

use crate::app::{App, InputMode, SettingsTab};

pub fn render(f: &mut Frame, app: &App) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(3), // tab bar
            Constraint::Min(1),   // content
            Constraint::Length(1), // status bar
        ])
        .split(f.area());

    render_tabs(f, app, chunks[0]);

    match app.settings_tab {
        SettingsTab::AgentConfig => render_agent_config(f, app, chunks[1]),
        SettingsTab::PathPolicies => render_path_policies(f, app, chunks[1]),
        SettingsTab::Tools => render_tools(f, app, chunks[1]),
        SettingsTab::About => render_about(f, chunks[1]),
    }

    render_settings_status(f, app, chunks[2]);
}

fn render_tabs(f: &mut Frame, app: &App, area: Rect) {
    let titles: Vec<&str> = SettingsTab::ALL.iter().map(|t| t.label()).collect();
    let selected = SettingsTab::ALL
        .iter()
        .position(|t| *t == app.settings_tab)
        .unwrap_or(0);

    let tabs = Tabs::new(titles)
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" Settings (Tab/Shift+Tab to switch, Esc to go back) "),
        )
        .select(selected)
        .style(Style::default().fg(Color::DarkGray))
        .highlight_style(Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD));

    f.render_widget(tabs, area);
}

fn render_agent_config(f: &mut Frame, app: &App, area: Rect) {
    let cfg = app.gateway.get_config();

    let api_key_display = if app.show_api_key {
        cfg.agent.api_key.clone().unwrap_or_else(|| "(not set)".to_string())
    } else {
        cfg.agent
            .api_key
            .as_ref()
            .map(|k| {
                if k.len() > 8 {
                    format!("{}…{}", &k[..4], &k[k.len() - 4..])
                } else {
                    "••••••••".to_string()
                }
            })
            .unwrap_or_else(|| "(not set)".to_string())
    };

    let fields = vec![
        format!("Agent Name:     {}", cfg.agent.name),
        format!("Model:          {}", cfg.agent.model),
        format!("Thinking Level: {}", cfg.agent.thinking_level),
        format!(
            "API Key:        {}  [Space to {}]",
            api_key_display,
            if app.show_api_key { "hide" } else { "show" },
        ),
    ];

    let items: Vec<ListItem> = fields
        .iter()
        .enumerate()
        .map(|(i, field)| {
            let style = if i == app.settings_cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == app.settings_cursor { "▸ " } else { "  " };
            ListItem::new(format!("{prefix}{field}")).style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Agent Configuration (Enter to edit, ↑↓ to navigate) "),
    );

    f.render_widget(list, area);

    // Show edit buffer overlay if editing
    if app.input_mode == InputMode::Editing {
        let edit_area = centered_rect(60, 3, area);
        let field_name = match app.settings_cursor {
            0 => "Agent Name",
            1 => "Model",
            2 => "Thinking Level",
            3 => "API Key",
            _ => "Field",
        };
        let editor = Paragraph::new(app.edit_buffer.as_str())
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(format!(" Edit: {field_name} (Enter to save, Esc to cancel) "))
                    .style(Style::default().fg(Color::Yellow)),
            )
            .style(Style::default().fg(Color::White));
        f.render_widget(editor, edit_area);
        f.set_cursor_position(Position::new(
            edit_area.x + app.edit_buffer.len() as u16 + 1,
            edit_area.y + 1,
        ));
    }
}

fn render_path_policies(f: &mut Frame, app: &App, area: Rect) {
    if app.path_policies.is_empty() {
        let msg = Paragraph::new("  No path policies configured.\n\n  Press 'a' to add one.")
            .block(
                Block::default()
                    .borders(Borders::ALL)
                    .title(" Path Policies "),
            );
        f.render_widget(msg, area);

        if app.input_mode == InputMode::Editing {
            render_path_editor(f, app, area);
        }
        return;
    }

    let items: Vec<ListItem> = app
        .path_policies
        .iter()
        .enumerate()
        .map(|(i, policy)| {
            let access = format!("{:?}", policy.access).to_lowercase().replace("only", "-only");
            let recursive = if policy.recursive { " (recursive)" } else { "" };
            let style = if i == app.path_cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == app.path_cursor { "▸ " } else { "  " };
            ListItem::new(format!(
                "{prefix}{} [{}]{recursive}",
                policy.path.display(),
                access,
            ))
            .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Path Policies (Space=cycle access, a=add, d=delete) "),
    );

    f.render_widget(list, area);

    if app.input_mode == InputMode::Editing {
        render_path_editor(f, app, area);
    }
}

fn render_path_editor(f: &mut Frame, app: &App, area: Rect) {
    let edit_area = centered_rect(70, 3, area);
    let editor = Paragraph::new(app.edit_buffer.as_str())
        .block(
            Block::default()
                .borders(Borders::ALL)
                .title(" New Path (Enter to add, Esc to cancel) ")
                .style(Style::default().fg(Color::Yellow)),
        )
        .style(Style::default().fg(Color::White));
    f.render_widget(editor, edit_area);
    f.set_cursor_position(Position::new(
        edit_area.x + app.edit_buffer.len() as u16 + 1,
        edit_area.y + 1,
    ));
}

fn render_tools(f: &mut Frame, app: &App, area: Rect) {
    let tools = app.gateway.get_tools_info();

    let items: Vec<ListItem> = tools
        .iter()
        .enumerate()
        .map(|(i, tool)| {
            let status = if tool.enabled { "✅" } else { "❌" };
            let style = if i == app.tools_cursor {
                Style::default().fg(Color::Cyan).add_modifier(Modifier::BOLD)
            } else {
                Style::default()
            };
            let prefix = if i == app.tools_cursor { "▸ " } else { "  " };
            ListItem::new(format!("{prefix}{status} {}  — {}", tool.name, tool.description))
                .style(style)
        })
        .collect();

    let list = List::new(items).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Tools (Space to toggle, ↑↓ to navigate) "),
    );

    f.render_widget(list, area);
}

fn render_about(f: &mut Frame, area: Rect) {
    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Batchismo",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  Version: 0.1.0"),
        Line::from("  Interface: Terminal (bat-tui)"),
        Line::from(""),
        Line::from("  Your hardware. Your data. Your rules."),
        Line::from(""),
        Line::from(Span::styled(
            "  A locally-installed, OS-native AI agent platform.",
            Style::default().fg(Color::DarkGray),
        )),
        Line::from(Span::styled(
            "  No cloud. No Docker. No config files to hand-edit.",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let about = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" About "),
    );

    f.render_widget(about, area);
}

fn render_settings_status(f: &mut Frame, app: &App, area: Rect) {
    let hint = match app.settings_tab {
        SettingsTab::AgentConfig => "↑↓ navigate │ Enter edit │ Tab next page │ Esc back to chat",
        SettingsTab::PathPolicies => "↑↓ navigate │ Space cycle access │ a add │ d delete │ Esc back",
        SettingsTab::Tools => "↑↓ navigate │ Space toggle │ Esc back to chat",
        SettingsTab::About => "Tab next page │ Esc back to chat",
    };

    let bar = Paragraph::new(format!(" {hint}"))
        .style(Style::default().bg(Color::DarkGray).fg(Color::White));

    f.render_widget(bar, area);
}

/// Helper to create a centered rect within a given area.
fn centered_rect(percent_x: u16, height: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - height.min(100)) / 2),
            Constraint::Length(height),
            Constraint::Percentage((100 - height.min(100)) / 2),
        ])
        .split(area);

    Layout::default()
        .direction(Direction::Horizontal)
        .constraints([
            Constraint::Percentage((100 - percent_x) / 2),
            Constraint::Percentage(percent_x),
            Constraint::Percentage((100 - percent_x) / 2),
        ])
        .split(popup_layout[1])[1]
}
