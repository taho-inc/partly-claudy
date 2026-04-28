use chrono::{DateTime, Utc};
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let columns = Layout::horizontal([Constraint::Min(20), Constraint::Length(48)]).split(area);

    let title_line = Line::from(vec![Span::styled(
        "partly-claudy",
        Style::default()
            .fg(app.theme.accent())
            .add_modifier(Modifier::BOLD),
    )]);
    let subtitle_line = Line::from(vec![Span::styled(
        "live uptime and incidents from status.claude.com",
        Style::default().fg(app.theme.muted()),
    )]);
    frame.render_widget(Paragraph::new(vec![title_line, subtitle_line]), columns[0]);

    frame.render_widget(
        Paragraph::new(right_status(app)).right_aligned(),
        columns[1],
    );
}

fn right_status(app: &App) -> Vec<Line<'static>> {
    let (indicator_color, label) = match &app.summary {
        Some(s) => (
            app.theme.indicator_color(s.status.indicator),
            s.status.description.clone(),
        ),
        None => (app.theme.muted(), "Loading…".to_string()),
    };
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

    let updated = match app.summary.as_ref().map(|s| s.page.updated_at) {
        Some(t) => format!("page updated {}", relative(t)),
        None => "page updated never".to_string(),
    };
    let updated_line = Line::from(vec![Span::styled(
        updated,
        Style::default().fg(app.theme.dim()),
    )]);

    vec![status_line, updated_line]
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
