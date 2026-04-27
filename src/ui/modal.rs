use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph, Wrap};
use ratatui::Frame;
use tui_overlay::{Anchor, Backdrop, Overlay, Slide};

use crate::api::Incident;
use crate::app::{App, ModalTarget};
use crate::bars::{self, DayStatus};
use crate::theme::AppTheme;

pub fn render(frame: &mut Frame, area: Rect, app: &mut App) {
    let title = modal_title(app);
    let close_hint = Line::from(vec![
        Span::raw(" "),
        Span::styled("[esc]", Style::default().fg(app.theme.accent())),
        Span::styled(" close ", Style::default().fg(app.theme.dim())),
    ])
    .right_aligned();
    let block = Block::default()
        .title(Line::from(vec![
            Span::raw(" "),
            Span::styled(
                title,
                Style::default()
                    .fg(app.theme.text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw(" "),
        ]))
        .title_bottom(close_hint)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .style(Style::default().bg(app.theme.panel_bg()));

    let (mw, mh) = desired_size(app, area);
    let overlay = Overlay::new()
        .anchor(Anchor::Center)
        .slide(Slide::Top)
        .width(Constraint::Length(mw))
        .height(Constraint::Length(mh))
        .backdrop(Backdrop::new(app.theme.bg()))
        .block(block);

    let mut state = app.overlay.clone();
    frame.render_stateful_widget(overlay, area, &mut state);
    app.overlay = state;

    let Some(inner) = app.overlay.inner_area() else {
        return;
    };

    match app.modal_target.clone() {
        ModalTarget::Empty => render_empty(frame, inner, app),
        ModalTarget::Incident(id) => {
            if let Some(inc) = lookup_incident(app, &id) {
                let inc = inc.clone();
                render_incident(frame, inner, app, &inc);
            }
        }
        ModalTarget::Day { row, day } => render_day(frame, inner, app, row, day),
    }
}

/// Pick a modal size that fits the content, clamped to the viewport.
/// Width and height are total (content + block chrome).
fn desired_size(app: &App, area: Rect) -> (u16, u16) {
    let max_w = area.width.saturating_sub(4).max(40);
    let max_h = area.height.saturating_sub(4).max(8);
    let (pref_w, content_h) = match &app.modal_target {
        ModalTarget::Empty => (50, 4),
        ModalTarget::Incident(id) => incident_size(app, id, max_w),
        ModalTarget::Day { row, day } => day_size(app, *row, *day),
    };
    let w = pref_w.min(max_w);
    let h = content_h.saturating_add(2).min(max_h); // +2 block chrome
    (w, h)
}

fn incident_size(app: &App, id: &str, width_budget: u16) -> (u16, u16) {
    let Some(inc) = lookup_incident(app, id) else {
        return (60, 8);
    };
    let body_budget = width_budget.saturating_sub(4).max(40) as usize;
    let updates_h: usize = 1 // section "Updates"
        + inc
            .incident_updates
            .iter()
            .map(|u| {
                let wrapped: usize = u
                    .body
                    .lines()
                    .map(|line| line.chars().count().div_ceil(body_budget).max(1))
                    .sum();
                1 + wrapped + 1 // timestamp+status, body, blank
            })
            .sum::<usize>();
    let total = 8 + updates_h.max(2); // 8 = header constraint
    (90, total as u16)
}

fn day_size(app: &App, row: usize, day: usize) -> (u16, u16) {
    let Some(ur) = app.bars.get(row) else {
        return (60, 8);
    };
    let date = bars::date_for_day(bars::today_utc(), day);
    let matching = app
        .summary
        .as_ref()
        .map(|s| {
            s.incidents
                .iter()
                .chain(s.scheduled_maintenances.iter())
                .filter(|inc| {
                    let started = inc.started_at.date_naive();
                    let ended = inc
                        .resolved_at
                        .unwrap_or_else(chrono::Utc::now)
                        .date_naive();
                    started <= date && date <= ended
                })
                .count()
        })
        .unwrap_or(0);
    let _ = ur;
    let h = 6 + matching.max(1) as u16; // 6 = header lines + section
    (70, h)
}

fn modal_title(app: &App) -> String {
    match &app.modal_target {
        ModalTarget::Empty => "Details".into(),
        ModalTarget::Incident(_) => "Incident detail".into(),
        ModalTarget::Day { .. } => "Day detail".into(),
    }
}

fn lookup_incident<'a>(app: &'a App, id: &str) -> Option<&'a Incident> {
    let s = app.summary.as_ref()?;
    s.incidents
        .iter()
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
            "  Tab cycles panes · Enter opens detail · Esc closes",
            Style::default().fg(app.theme.dim()),
        )),
    ]);
    frame.render_widget(p, area);
}

fn render_incident(frame: &mut Frame, area: Rect, app: &App, inc: &Incident) {
    let chunks = Layout::vertical([Constraint::Length(8), Constraint::Min(1)]).split(area);
    let resolved = inc
        .resolved_at
        .map(|r| {
            format!(
                "{} ({})",
                r.format("%Y-%m-%d %H:%M UTC"),
                duration_str(inc.started_at, r)
            )
        })
        .unwrap_or_else(|| "ongoing".to_string());
    let components = inc
        .components
        .iter()
        .map(|c| c.name.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    let affects = if components.is_empty() {
        "—".to_string()
    } else {
        components
    };
    let header = Paragraph::new(vec![
        kv(&app.theme, "Name", inc.name.clone()),
        kv_styled(
            &app.theme,
            "Status",
            inc.status.label(),
            if inc.status.is_resolved() {
                app.theme.success()
            } else {
                app.theme.impact_color(inc.impact)
            },
        ),
        kv_styled(
            &app.theme,
            "Impact",
            inc.impact.label(),
            app.theme.impact_color(inc.impact),
        ),
        kv(
            &app.theme,
            "Started",
            inc.started_at.format("%Y-%m-%d %H:%M UTC").to_string(),
        ),
        kv(&app.theme, "Resolved", resolved),
        kv(&app.theme, "Affects", affects),
    ])
    .wrap(Wrap { trim: false });
    frame.render_widget(header, chunks[0]);

    let mut updates: Vec<Line> = vec![section(&app.theme, "Updates")];
    let mut sorted = inc.incident_updates.clone();
    sorted.sort_by_key(|u| std::cmp::Reverse(u.display_at.unwrap_or(u.created_at)));
    for u in &sorted {
        let pill_color = if u.status.is_resolved() {
            app.theme.success()
        } else {
            app.theme.impact_color(inc.impact)
        };
        let shown_at = u.display_at.unwrap_or(u.created_at);
        updates.push(Line::from(vec![
            Span::styled(
                shown_at.format("%b %-d %H:%M ").to_string(),
                Style::default().fg(app.theme.dim()),
            ),
            Span::styled(
                u.status.label(),
                Style::default().fg(pill_color).add_modifier(Modifier::BOLD),
            ),
        ]));
        for line in u.body.lines() {
            updates.push(Line::from(vec![
                Span::raw("  "),
                Span::styled(line.to_string(), Style::default().fg(app.theme.text())),
            ]));
        }
        updates.push(Line::raw(""));
    }
    frame.render_widget(
        Paragraph::new(updates).wrap(Wrap { trim: false }),
        chunks[1],
    );
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
            let ended = inc
                .resolved_at
                .unwrap_or_else(chrono::Utc::now)
                .date_naive();
            started <= date && date <= ended
        };
        let mut shown = 0;
        for inc in s
            .incidents
            .iter()
            .chain(s.scheduled_maintenances.iter())
            .filter(|i| day_match(i))
        {
            lines.push(Line::from(vec![
                Span::styled(
                    "  ● ",
                    Style::default().fg(app.theme.impact_color(inc.impact)),
                ),
                Span::styled(inc.name.clone(), Style::default().fg(app.theme.text())),
                Span::raw("  "),
                Span::styled(
                    inc.impact.label(),
                    Style::default().fg(app.theme.impact_color(inc.impact)),
                ),
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

fn kv_styled(
    theme: &AppTheme,
    k: &str,
    v: impl Into<String>,
    color: ratatui::style::Color,
) -> Line<'static> {
    Line::from(vec![
        Span::styled(format!(" {:<12}", k), Style::default().fg(theme.muted())),
        Span::styled(
            v.into(),
            Style::default().fg(color).add_modifier(Modifier::BOLD),
        ),
    ])
}

fn section<'a>(theme: &AppTheme, label: &'a str) -> Line<'a> {
    Line::from(vec![Span::styled(
        format!(" — {} ", label),
        Style::default()
            .fg(theme.accent())
            .add_modifier(Modifier::BOLD),
    )])
}

fn duration_str(
    start: chrono::DateTime<chrono::Utc>,
    end: chrono::DateTime<chrono::Utc>,
) -> String {
    let mins = (end - start).num_minutes().max(0);
    if mins < 60 {
        format!("{}m", mins)
    } else {
        format!("{}h {:02}m", mins / 60, mins % 60)
    }
}
