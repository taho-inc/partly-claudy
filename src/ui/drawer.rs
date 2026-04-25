use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use tui_overlay::{Anchor, Backdrop, Easing, Overlay, Slide};

use crate::api::{Component, Incident};
use crate::app::{App, DrawerTarget};
use crate::bars::{self, DayStatus};
use crate::theme::AppTheme;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = drawer_title(app);
    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(title, Style::default().fg(app.theme.text()).add_modifier(Modifier::BOLD)),
            Span::raw(" "),
        ]))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .style(Style::default().bg(app.theme.panel_bg()));

    let width = drawer_width(area.width);
    let overlay = Overlay::new()
        .anchor(Anchor::Right)
        .slide(Slide::Right)
        .width(Constraint::Length(width))
        .height(Constraint::Percentage(100))
        .backdrop(Backdrop::new(app.theme.bg()))
        .block(block);

    let mut state = app.overlay.clone();
    let _ = Easing::EaseOut;
    frame.render_stateful_widget(overlay, area, &mut state);
    app.overlay = state;

    let Some(inner) = app.overlay.inner_area() else { return };

    match app.drawer_target.clone() {
        DrawerTarget::Empty => render_empty(frame, inner, app),
        DrawerTarget::Component(id) => {
            if let Some(c) = lookup_component(app, &id) {
                let c = c.clone();
                render_component(frame, inner, app, &c);
            }
        }
        DrawerTarget::Incident(id) => {
            if let Some(inc) = lookup_incident(app, &id) {
                let inc = inc.clone();
                render_incident(frame, inner, app, &inc);
            }
        }
        DrawerTarget::Day { row, day } => render_day(frame, inner, app, row, day),
    }
}

fn drawer_width(w: u16) -> u16 {
    let pct = (w as u32 * 40 / 100) as u16;
    pct.clamp(40, 64).min(w.saturating_sub(2))
}

fn drawer_title(app: &App) -> String {
    match &app.drawer_target {
        DrawerTarget::Empty => "Details".into(),
        DrawerTarget::Component(_) => "Section detail".into(),
        DrawerTarget::Incident(_) => "Incident detail".into(),
        DrawerTarget::Day { .. } => "Day detail".into(),
    }
}

fn lookup_component<'a>(app: &'a App, id: &str) -> Option<&'a Component> {
    app.summary.as_ref()?.components.iter().find(|c| c.id == id)
}

fn lookup_incident<'a>(app: &'a App, id: &str) -> Option<&'a Incident> {
    let s = app.summary.as_ref()?;
    s.incidents
        .iter()
        .chain(s.past_incidents.iter())
        .chain(s.scheduled_maintenances.iter())
        .find(|i| i.id == id)
}

fn render_empty(frame: &mut Frame, area: Rect, app: &App) {
    let p = Paragraph::new(vec![
        Line::raw(""),
        Line::from(Span::styled(
            "  Select a component, day or incident",
            Style::default().fg(app.theme.muted()),
        )),
        Line::raw(""),
        Line::from(Span::styled(
            "  Tab cycles panes · Enter pins selection · d toggles drawer",
            Style::default().fg(app.theme.dim()),
        )),
    ]);
    frame.render_widget(p, area);
}

fn render_component(frame: &mut Frame, area: Rect, app: &App, c: &Component) {
    let kids = app.children_of(&c.id);
    let mut lines = vec![
        kv(&app.theme, "Name", c.name.clone()),
        kv(&app.theme, "Status", crate::ui::sections::label(c.status)),
        kv(&app.theme, "Updated", c.updated_at.format("%Y-%m-%d %H:%M UTC").to_string()),
    ];
    if let Some(desc) = &c.description {
        if !desc.is_empty() {
            lines.push(kv(&app.theme, "Description", desc.clone()));
        }
    }
    lines.push(Line::raw(""));
    if !kids.is_empty() {
        lines.push(section(&app.theme, "Children"));
        for k in kids {
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(app.theme.component_color(k.status))),
                Span::styled(k.name.clone(), Style::default().fg(app.theme.text())),
                Span::raw("  "),
                Span::styled(
                    crate::ui::sections::label(k.status),
                    Style::default().fg(app.theme.component_color(k.status)),
                ),
            ]));
        }
    }
    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn render_incident(frame: &mut Frame, area: Rect, app: &App, inc: &Incident) {
    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Min(1)]).split(area);
    let resolved = inc
        .resolved_at
        .map(|r| format!("{} ({})", r.format("%Y-%m-%d %H:%M UTC"), duration_str(inc.started_at, r)))
        .unwrap_or_else(|| "ongoing".to_string());
    let components = inc
        .components
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let affects = if components.is_empty() { "—".to_string() } else { components };
    let header = Paragraph::new(vec![
        kv(&app.theme, "Name", inc.name.clone()),
        kv_styled(
            &app.theme,
            "Status",
            inc.status.label(),
            if inc.status.is_resolved() { app.theme.success() } else { app.theme.impact_color(inc.impact) },
        ),
        kv_styled(&app.theme, "Impact", inc.impact.label(), app.theme.impact_color(inc.impact)),
        kv(&app.theme, "Started", inc.started_at.format("%Y-%m-%d %H:%M UTC").to_string()),
        kv(&app.theme, "Resolved", resolved),
        kv(&app.theme, "Affects", affects),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    let mut updates: Vec<Line> = vec![section(&app.theme, "Updates")];
    let mut sorted = inc.incident_updates.clone();
    sorted.sort_by_key(|u| std::cmp::Reverse(u.display_at));
    for u in &sorted {
        let pill_color = if u.status.is_resolved() {
            app.theme.success()
        } else {
            app.theme.impact_color(inc.impact)
        };
        updates.push(Line::from(vec![
            Span::styled(
                u.display_at.format("%b %-d %H:%M ").to_string(),
                Style::default().fg(app.theme.dim()),
            ),
            Span::styled(u.status.label(), Style::default().fg(pill_color).add_modifier(Modifier::BOLD)),
        ]));
        for line in u.body.lines() {
            updates.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line.to_string(), Style::default().fg(app.theme.text())),
            ]));
        }
        updates.push(Line::raw(""));
    }
    frame.render_widget(Paragraph::new(updates).wrap(Wrap { trim: false }), chunks[1]);
}

fn render_day(frame: &mut Frame, area: Rect, app: &App, row: usize, day: usize) {
    let Some(ur) = app.bars.get(row) else { return };
    let date = bars::date_for_day(bars::today_utc(), day);
    let status = ur.days.get(day).copied().unwrap_or(DayStatus::Unknown);
    let status_label = match status {
        DayStatus::Unknown => "no data",
        DayStatus::Operational => "operational",
        DayStatus::Maintenance => "maintenance",
        DayStatus::Degraded => "degraded",
        DayStatus::Outage => "outage",
    };
    let status_color = match status {
        DayStatus::Unknown => app.theme.dim(),
        DayStatus::Operational => app.theme.success(),
        DayStatus::Maintenance => app.theme.info(),
        DayStatus::Degraded => app.theme.warning(),
        DayStatus::Outage => app.theme.danger(),
    };

    let mut lines = vec![
        kv(&app.theme, "Component", ur.group_name.clone()),
        kv(&app.theme, "Date", date.format("%a %b %-d, %Y").to_string()),
        kv_styled(&app.theme, "Status", status_label, status_color),
        kv(&app.theme, "90d uptime", format!("{:.2}%", ur.uptime_pct)),
        Line::raw(""),
        section(&app.theme, "Incidents on this day"),
    ];

    if let Some(s) = app.summary.as_ref() {
        let day_match = |inc: &Incident| {
            let started = inc.started_at.date_naive();
            let ended = inc.resolved_at.unwrap_or_else(chrono::Utc::now).date_naive();
            started <= date && date <= ended
        };
        let mut shown = 0;
        for inc in s
            .incidents
            .iter()
            .chain(s.past_incidents.iter())
            .chain(s.scheduled_maintenances.iter())
            .filter(|i| day_match(i))
        {
            lines.push(Line::from(vec![
                Span::styled("  ● ", Style::default().fg(app.theme.impact_color(inc.impact))),
                Span::styled(inc.name.clone(), Style::default().fg(app.theme.text())),
                Span::raw("  "),
                Span::styled(inc.impact.label(), Style::default().fg(app.theme.impact_color(inc.impact))),
            ]));
            shown += 1;
        }
        if shown == 0 {
            lines.push(Line::from(Span::styled(
                "  (none)",
                Style::default().fg(app.theme.dim()),
            )));
        }
    }

    frame.render_widget(Paragraph::new(lines).wrap(Wrap { trim: false }), area);
}

fn kv(theme: &AppTheme, k: &str, v: impl Into<String>) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {:<12}", k), Style::default().fg(theme.muted())),
        Span::styled(v.into(), Style::default().fg(theme.text())),
    ])
}

fn kv_styled(theme: &AppTheme, k: &str, v: impl Into<String>, color: ratatui::style::Color) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {:<12}", k), Style::default().fg(theme.muted())),
        Span::styled(v.into(), Style::default().fg(color).add_modifier(Modifier::BOLD)),
    ])
}

fn section<'a>(theme: &AppTheme, label: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!(" — {} ", label), Style::default().fg(theme.accent()).add_modifier(Modifier::BOLD)),
    ])
}

fn duration_str(start: chrono::DateTime<chrono::Utc>, end: chrono::DateTime<chrono::Utc>) -> String {
    let mins = (end - start).num_minutes().max(0);
    if mins < 60 {
        format!("{}m", mins)
    } else {
        format!("{}h {:02}m", mins / 60, mins % 60)
    }
}
