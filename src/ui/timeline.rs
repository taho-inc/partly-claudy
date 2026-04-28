use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{List, ListItem, ListState};

use crate::api::{Incident, IncidentStatus};
use crate::app::{App, Pane};
use crate::ui::skeleton;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let focused = matches!(app.focus, Pane::Incidents);
    let mut block = crate::ui::services::pane_block(" Incidents ", focused, &app.theme);

    if let Some(svc) = app.selected_service() {
        let scope = Line::from(vec![
            Span::styled(" for ", Style::default().fg(app.theme.dim())),
            Span::styled(
                svc.name.clone(),
                Style::default()
                    .fg(app.theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(" ", Style::default().fg(app.theme.dim())),
        ])
        .right_aligned();
        block = block.title_top(scope);
    }
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_loading() {
        skeleton::render_incidents(frame, inner, app);
        return;
    }

    let groups = app.timeline();
    if groups.is_empty() {
        let p = ratatui::widgets::Paragraph::new("No incidents in the visible window.")
            .style(Style::default().fg(app.theme.muted()));
        frame.render_widget(p, inner);
        return;
    }

    let list_area = Rect {
        width: inner.width.saturating_sub(1),
        ..inner
    };
    let name_budget = name_budget_for(list_area.width);
    let mut items: Vec<ListItem> = Vec::new();
    let mut flat_index: Vec<Option<usize>> = Vec::new();
    let mut counter = 0usize;
    for (label, day) in &groups {
        items.push(ListItem::new(Line::from(vec![
            Span::styled("▾ ", Style::default().fg(app.theme.dim())),
            Span::styled(
                label.clone(),
                Style::default()
                    .fg(app.theme.text())
                    .add_modifier(Modifier::BOLD),
            ),
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
                items.push(ListItem::new(incident_line(app, inc, name_budget)));
                flat_index.push(Some(counter));
                counter += 1;
            }
        }
    }

    let total_items = items.len();
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
    frame.render_stateful_widget(list, list_area, &mut state);

    crate::ui::scroll::overlay(
        frame,
        inner,
        state.offset(),
        list_area.height as usize,
        total_items,
        app.theme.dim(),
    );
}

fn incident_line<'a>(app: &App, inc: &'a Incident, name_budget: usize) -> Line<'a> {
    let time = inc.started_at.format("%H:%M").to_string();
    let impact_color = app.theme.impact_color(inc.impact);
    let status_color = if inc.status.is_resolved() {
        app.theme.success()
    } else {
        impact_color
    };
    let name = truncate(&inc.name, name_budget);
    Line::from(vec![
        Span::raw("   "),
        Span::styled(
            "● ",
            Style::default()
                .fg(impact_color)
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(time, Style::default().fg(app.theme.dim())),
        Span::raw("  "),
        Span::styled(name, Style::default().fg(app.theme.text())),
        Span::raw("  "),
        Span::styled(
            status_pill(inc.status),
            Style::default()
                .fg(status_color)
                .add_modifier(Modifier::BOLD),
        ),
    ])
}

/// Width left for the incident name after fixed chrome:
/// highlight_symbol(3) + indent(3) + dot(2) + time(5) + gap(2) + gap(2) + max pill(13).
fn name_budget_for(area_width: u16) -> usize {
    const FIXED: u16 = 3 + 3 + 2 + 5 + 2 + 2 + 13;
    (area_width.saturating_sub(FIXED)) as usize
}

fn truncate(s: &str, max: usize) -> String {
    if max == 0 {
        return String::new();
    }
    if s.chars().count() <= max {
        return s.to_string();
    }
    let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
    out.push('…');
    out
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
