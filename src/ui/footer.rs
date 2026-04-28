use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Pane};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([Constraint::Min(20), Constraint::Length(48)]).split(area);

    let hints: &[(&str, &str)] = match app.focus {
        Pane::Services => &[
            ("↑/↓", "service"),
            ("←/→", "scrub day"),
            ("Enter", "detail"),
            ("Tab", "next pane"),
        ],
        Pane::Incidents => &[("↑/↓", "select"), ("Enter", "detail"), ("Tab", "next pane")],
    };
    frame.render_widget(Paragraph::new(hint_line(app, hints)), cols[0]);

    let right = match &app.status_toast {
        Some(t) => Line::from(Span::styled(
            t.clone(),
            Style::default()
                .fg(app.theme.warning())
                .add_modifier(Modifier::BOLD),
        )),
        None => hint_line(app, &[("?", "help"), ("t", "theme"), ("q", "quit")]),
    };
    frame.render_widget(Paragraph::new(right).right_aligned(), cols[1]);
}

fn hint_line(app: &App, pairs: &[(&str, &str)]) -> Line<'static> {
    let dim = Style::default().fg(app.theme.dim());
    let bracket = Style::default().fg(app.theme.muted());
    let key = Style::default().fg(app.theme.accent());

    let mut spans: Vec<Span<'static>> = Vec::with_capacity(pairs.len() * 6);
    for (i, (k, v)) in pairs.iter().enumerate() {
        if i > 0 {
            spans.push(Span::styled(" · ", dim));
        }
        spans.push(Span::styled("[", bracket));
        spans.push(Span::styled((*k).to_string(), key));
        spans.push(Span::styled("]", bracket));
        spans.push(Span::styled(format!(" {}", v), dim));
    }
    Line::from(spans)
}
