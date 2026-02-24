use ratatui::{
    prelude::*,
    widgets::{Block, Borders, Clear, Paragraph},
};

pub fn render(f: &mut Frame) {
    let area = centered_rect(60, 70, f.area());

    // Clear the area behind the popup
    f.render_widget(Clear, area);

    let text = vec![
        Line::from(""),
        Line::from(Span::styled(
            "  Keyboard Shortcuts",
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        Line::from("  ── Chat ──"),
        Line::from("  Enter          Send message"),
        Line::from("  ↑/↓            Scroll messages"),
        Line::from("  Ctrl+S         Session switcher"),
        Line::from("  Tab            Switch to Settings"),
        Line::from("  Esc            Clear input"),
        Line::from("  Ctrl+C         Quit"),
        Line::from(""),
        Line::from("  ── Settings ──"),
        Line::from("  Tab/Shift+Tab  Switch sub-pages"),
        Line::from("  ↑/↓            Navigate fields"),
        Line::from("  Enter          Edit selected field"),
        Line::from("  Space          Toggle / cycle option"),
        Line::from("  Esc            Back to Chat"),
        Line::from(""),
        Line::from("  ── Path Policies ──"),
        Line::from("  a              Add new path"),
        Line::from("  d              Delete selected path"),
        Line::from("  Space          Cycle access level"),
        Line::from(""),
        Line::from("  ── Activity ──"),
        Line::from("  ↑/↓            Select subagent"),
        Line::from("  Enter          Expand/collapse details"),
        Line::from("  r              Refresh"),
        Line::from("  Esc            Back to Chat"),
        Line::from(""),
        Line::from("  ── Usage ──"),
        Line::from("  r              Refresh stats"),
        Line::from("  Esc            Back to Chat"),
        Line::from(""),
        Line::from("  ── Memory ──"),
        Line::from("  ↑/↓            Select file"),
        Line::from("  e              Edit selected file"),
        Line::from("  Ctrl+S         Save edits"),
        Line::from("  c              Consolidate memories"),
        Line::from("  Esc            Cancel edit / Back"),
        Line::from(""),
        Line::from(Span::styled(
            "  Press any key to close",
            Style::default().fg(Color::DarkGray),
        )),
    ];

    let help = Paragraph::new(text).block(
        Block::default()
            .borders(Borders::ALL)
            .title(" Help ")
            .style(Style::default().fg(Color::Yellow)),
    );

    f.render_widget(help, area);
}

fn centered_rect(percent_x: u16, percent_y: u16, area: Rect) -> Rect {
    let popup_layout = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Percentage((100 - percent_y) / 2),
            Constraint::Percentage(percent_y),
            Constraint::Percentage((100 - percent_y) / 2),
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
