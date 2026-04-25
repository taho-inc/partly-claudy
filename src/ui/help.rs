use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, Paragraph};

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 60, 18);
    frame.render_widget(Clear, popup);

    let block = Block::default()
        .title(Span::styled(" Help ", Style::default().fg(app.theme.text()).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .style(Style::default().bg(app.theme.panel_bg()));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let muted = Style::default().fg(app.theme.muted());
    let key = Style::default().fg(app.theme.accent()).add_modifier(Modifier::BOLD);

    let lines = vec![
        row(" q / Ctrl-c", "quit", key, muted),
        row(" Esc        ", "close drawer · then quit", key, muted),
        row(" Tab / ⇧Tab  ", "cycle Bars → Sections → Events", key, muted),
        row(" ↑ ↓ k j    ", "move selection in focused pane", key, muted),
        row(" ← → h l    ", "scrub days in the bars pane", key, muted),
        row(" Enter      ", "pin selection into the drawer", key, muted),
        row(" d          ", "toggle the detail drawer", key, muted),
        row(" r          ", "manual refresh now", key, muted),
        row(" t          ", "open theme picker (opaline)", key, muted),
        row(" /          ", "filter incidents by name", key, muted),
        row(" ?          ", "this help screen", key, muted),
        Line::raw(""),
        Line::from(Span::styled(
            "  Source: status.claude.com (Statuspage v2)",
            Style::default().fg(app.theme.dim()),
        )),
    ];

    frame.render_widget(Paragraph::new(lines), inner);
}

fn row<'a>(k: &'a str, v: &'a str, key_style: Style, muted: Style) -> Line<'a> {
    Line::from(vec![Span::styled(k.to_string(), key_style), Span::styled(format!("  {}", v), muted)])
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let h_pad = area.width.saturating_sub(width) / 2;
    let v_pad = area.height.saturating_sub(height) / 2;
    let v = Layout::vertical([
        Constraint::Length(v_pad),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    let h = Layout::horizontal([
        Constraint::Length(h_pad),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(v[1]);
    h[1]
}
