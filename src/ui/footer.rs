use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::Paragraph;
use ratatui::Frame;

use crate::app::{App, Pane};

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([Constraint::Min(20), Constraint::Length(40)]).split(area);

    let hints = match app.focus {
        Pane::Services => "↑/↓ service · ←/→ scrub day · Enter detail · Tab next pane",
        Pane::Events => "↑/↓ select · Enter detail · Tab next pane",
    };
    let left = Line::from(vec![Span::styled(
        hints,
        Style::default().fg(app.theme.dim()),
    )]);
    frame.render_widget(Paragraph::new(left), cols[0]);

    let right = match &app.status_toast {
        Some(t) => Line::from(Span::styled(
            t.clone(),
            Style::default()
                .fg(app.theme.warning())
                .add_modifier(Modifier::BOLD),
        )),
        None => Line::from(Span::styled(
            "? help · t theme · q quit",
            Style::default().fg(app.theme.dim()),
        )),
    };
    frame.render_widget(Paragraph::new(right).right_aligned(), cols[1]);
}
