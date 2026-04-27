use chrono::{DateTime, Utc};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::horizontal([Constraint::Min(20), Constraint::Length(48)]).split(area);

    let (indicator_color, label) = match &app.summary {
        Some(s) => (
            app.theme.indicator_color(s.status.indicator),
            s.status.description.clone(),
        ),
        None => (app.theme.muted(), "Loading…".to_string()),
    };

    let title_line = Line::from(vec![
        Span::styled(
            "claude-status ",
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            "· terminal status for claude.com",
            Style::default().fg(app.theme.muted()),
        ),
    ]);
    let status_line = Line::from(vec![
        Span::styled(
            "● ",
            Style::default()
                .fg(indicator_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(
            label,
            Style::default()
                .fg(app.theme.text())
                .add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(vec![title_line, status_line]), columns[0]);

    let right = right_status(app);
    frame.render_widget(Paragraph::new(right).right_aligned(), columns[1]);
}

fn right_status(app: &App) -> Vec<Line<'static>> {
    let theme_line = Line::from(vec![
        Span::styled("theme: ", Style::default().fg(app.theme.muted())),
        Span::styled(
            app.theme.name.clone(),
            Style::default().fg(app.theme.accent()),
        ),
        Span::raw("  "),
        Span::styled("? help", Style::default().fg(app.theme.muted())),
    ]);
    let updated = match app.summary.as_ref().map(|s| s.page.updated_at) {
        Some(t) => format!("page updated {}", relative(t)),
        None => "page updated never".to_string(),
    };
    let updated_line = Line::from(vec![Span::styled(
        updated,
        Style::default().fg(app.theme.dim()),
    )]);
    vec![theme_line, updated_line]
}

fn relative(t: DateTime<Utc>) -> String {
    let secs = (Utc::now() - t).num_seconds();
    if secs < 5 {
        "just now".into()
    } else if secs < 60 {
        format!("{}s ago", secs)
    } else if secs < 3600 {
        format!("{}m ago", secs / 60)
    } else {
        format!("{}h ago", secs / 3600)
    }
}
