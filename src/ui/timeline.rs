use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::api::{Incident, IncidentStatus};
use crate::app::{App, Pane};
use crate::ui::skeleton;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let focused = matches!(app.focus, Pane::Events);
    let block = Block::default()
        .title(Span::styled(" Events ", Style::default().fg(app.theme.text()).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(if focused { app.theme.focused_border() } else { app.theme.unfocused_border() });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_loading() {
        skeleton::render_list(frame, inner, app, 8);
        return;
    }

    let groups = app.timeline();
    if groups.is_empty() {
        let p = ratatui::widgets::Paragraph::new("No incidents in the visible window.")
            .style(Style::default().fg(app.theme.muted()));
        frame.render_widget(p, inner);
        return;
    }

    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_index: Vec<Option<usize>> = Vec::new();
    let mut counter = 0usize;
    for (label, day) in &groups {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("▾ ", Style::default().fg(app.theme.dim())),
            Span::styled(label.clone(), Style::default().fg(app.theme.text()).add_modifier(Modifier::BOLD)),
        ])));
        flat_index.push(None);

        if day.is_empty() {
            items.push(ListItem::new(Line::from(vec![
                Span::raw("   "),
                Span::styled("(no incidents)", Style::default().fg(app.theme.dim())),
            ])));
            flat_index.push(None);
        } else {
            for inc in day {
                items.push(ListItem::new(incident_line(app, inc)));
                flat_index.push(Some(counter));
                counter += 1;
            }
        }
    }

    let mut state = ListState::default();
    let target = flat_index
        .iter()
        .position(|x| matches!(x, Some(i) if *i == app.event_idx));
    if let Some(t) = target {
        state.select(Some(t));
    }
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol(Line::from(" ▶ "));
    frame.render_stateful_widget(list, inner, &mut state);
}

fn incident_line<'a>(app: &App, inc: &'a Incident) -> Line<'a> {
    let time = inc.started_at.format("%H:%M").to_string();
    let impact_color = app.theme.impact_color(inc.impact);
    let status_color = if inc.status.is_resolved() {
        app.theme.success()
    } else {
        impact_color
    };
    Line::from(vec![
        Span::raw("   "),
        Span::styled("● ", Style::default().fg(impact_color).add_modifier(Modifier::BOLD)),
        Span::styled(time, Style::default().fg(app.theme.dim())),
        Span::raw("  "),
        Span::styled(inc.name.clone(), Style::default().fg(app.theme.text())),
        Span::raw("  "),
        Span::styled(
            status_pill(inc.status),
            Style::default().fg(status_color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn status_pill(s: IncidentStatus) -> &'static str {
    match s {
        IncidentStatus::Investigating => "investigating",
        IncidentStatus::Identified => "identified",
        IncidentStatus::Monitoring => "monitoring",
        IncidentStatus::Resolved => "resolved",
        IncidentStatus::Postmortem => "postmortem",
        IncidentStatus::Scheduled => "scheduled",
        IncidentStatus::InProgress => "in progress",
        IncidentStatus::Verifying => "verifying",
        IncidentStatus::Completed => "completed",
    }
}
