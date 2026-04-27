use ratatui::layout::Rect;
use ratatui::style::{Color, Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};
use ratatui::Frame;

use crate::api::{Impact, Incident, Summary};
use crate::app::App;
use crate::theme::AppTheme;

const MAX_INCIDENT_ROWS: usize = 3;
const MAX_MAINTENANCE_ROWS: usize = 1;

pub struct Alert {
    pub lines: Vec<Line<'static>>,
    pub border_color: Color,
    pub title: &'static str,
}

impl Alert {
    /// Total height including the block chrome.
    pub fn block_height(&self) -> u16 {
        (self.lines.len() as u16).saturating_add(2)
    }
}

pub fn build(app: &App) -> Option<Alert> {
    let s = app.summary.as_ref()?;
    let lines = lines_for(&app.theme, s);
    if lines.is_empty() {
        return None;
    }
    let active_max = active_max_impact(s);
    Some(Alert {
        lines,
        border_color: alert_color(&app.theme, active_max),
        title: alert_title(active_max.is_some()),
    })
}

pub fn render(frame: &mut Frame, area: Rect, alert: &Alert, app: &App) {
    if area.height == 0 {
        return;
    }
    let block = Block::default()
        .title(Span::styled(
            format!(" {} ", alert.title),
            Style::default()
                .fg(alert.border_color)
                .add_modifier(Modifier::BOLD),
        ))
        .borders(Borders::ALL)
        .border_style(Style::default().fg(alert.border_color))
        .style(Style::default().bg(app.theme.panel_bg()));
    let inner = block.inner(area);
    frame.render_widget(block, area);
    frame.render_widget(Paragraph::new(alert.lines.clone()), inner);
}

fn active_max_impact(s: &Summary) -> Option<Impact> {
    s.incidents
        .iter()
        .filter(|i| !i.status.is_resolved())
        .map(|i| i.impact)
        .max_by_key(|i| impact_rank(*i))
}

fn impact_rank(i: Impact) -> u8 {
    match i {
        Impact::None => 0,
        Impact::Maintenance => 1,
        Impact::Minor => 2,
        Impact::Major => 3,
        Impact::Critical => 4,
    }
}

fn alert_color(theme: &AppTheme, max: Option<Impact>) -> Color {
    match max {
        Some(Impact::Critical | Impact::Major) => theme.danger(),
        Some(Impact::Minor) => theme.warning(),
        _ => theme.info(),
    }
}

fn alert_title(has_active: bool) -> &'static str {
    if has_active {
        "Service Disruption"
    } else {
        "Scheduled Maintenance"
    }
}

fn lines_for(theme: &AppTheme, s: &Summary) -> Vec<Line<'static>> {
    let mut out = Vec::new();
    let active: Vec<&Incident> = s
        .incidents
        .iter()
        .filter(|i| !i.status.is_resolved())
        .collect();
    out.extend(rows(
        theme,
        &active,
        MAX_INCIDENT_ROWS,
        |i| theme.impact_color(i.impact),
        "active incident",
    ));

    let upcoming: Vec<&Incident> = s.scheduled_maintenances.iter().collect();
    out.extend(rows(
        theme,
        &upcoming,
        MAX_MAINTENANCE_ROWS,
        |_| theme.info(),
        "scheduled maintenance",
    ));
    out
}

fn rows(
    theme: &AppTheme,
    incidents: &[&Incident],
    cap: usize,
    color: impl Fn(&Incident) -> ratatui::style::Color,
    summary_label: &str,
) -> Vec<Line<'static>> {
    if incidents.is_empty() {
        return Vec::new();
    }
    let mut out = Vec::with_capacity(cap.min(incidents.len()) + 1);
    for inc in incidents.iter().take(cap) {
        out.push(Line::from(vec![
            Span::styled(
                "▸ ",
                Style::default().fg(color(inc)).add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                inc.name.clone(),
                Style::default()
                    .fg(theme.text())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::raw("  "),
            Span::styled(inc.impact.label(), Style::default().fg(color(inc))),
        ]));
    }
    if incidents.len() > cap {
        let extra = incidents.len() - cap;
        out.push(Line::from(vec![Span::styled(
            format!("  +{} more {}{}", extra, summary_label, plural(extra)),
            Style::default().fg(theme.dim()),
        )]));
    }
    out
}

fn plural(n: usize) -> &'static str {
    if n == 1 {
        ""
    } else {
        "s"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::{Impact, IncidentStatus};
    use chrono::Utc;
    use ratatui::backend::TestBackend;
    use ratatui::Terminal;

    const FIXTURE: &str = include_str!("../../tests/fixtures/summary.json");

    fn incident(name: &str, impact: Impact, status: IncidentStatus) -> Incident {
        let now = Utc::now();
        Incident {
            id: name.to_string(),
            name: name.to_string(),
            status,
            impact,
            created_at: now,
            updated_at: now,
            monitoring_at: None,
            resolved_at: None,
            started_at: now,
            shortlink: None,
            incident_updates: Vec::new(),
            components: Vec::new(),
        }
    }

    fn buffer_to_string(buf: &ratatui::buffer::Buffer) -> String {
        let mut out = String::new();
        for y in 0..buf.area.height {
            for x in 0..buf.area.width {
                out.push_str(buf[(x, y)].symbol());
            }
            out.push('\n');
        }
        out
    }

    #[test]
    fn empty_when_nothing_active() {
        let theme = AppTheme::default_builtin();
        let out = rows(&theme, &[], 3, |_| theme.text(), "active incident");
        assert!(out.is_empty());
    }

    #[test]
    fn caps_with_overflow_summary() {
        let theme = AppTheme::default_builtin();
        let a = incident("a", Impact::Major, IncidentStatus::Investigating);
        let b = incident("b", Impact::Minor, IncidentStatus::Identified);
        let c = incident("c", Impact::Critical, IncidentStatus::Investigating);
        let d = incident("d", Impact::Minor, IncidentStatus::Identified);
        let incs = vec![&a, &b, &c, &d];
        let out = rows(&theme, &incs, 2, |_| theme.text(), "active incident");
        assert_eq!(out.len(), 3, "2 rows + 1 overflow summary");
    }

    #[test]
    fn renders_fixture() {
        let summary: Summary = serde_json::from_str(FIXTURE).expect("parse fixture");
        let theme = AppTheme::default_builtin();
        let lines = lines_for(&theme, &summary);

        let backend = TestBackend::new(80, lines.len() as u16);
        let mut terminal = Terminal::new(backend).unwrap();
        terminal
            .draw(|f| {
                let area = f.area();
                f.render_widget(Paragraph::new(lines.clone()), area);
            })
            .unwrap();

        let dump = buffer_to_string(terminal.backend().buffer());
        println!("--- banner ---\n{dump}--- end ---");

        assert!(dump.contains("Elevated error rate on api.anthropic.com"));
        assert!(dump.contains("minor"));
        assert!(dump.contains("Database failover drill"));
        assert_eq!(
            lines.len(),
            2,
            "1 active incident + 1 scheduled maintenance"
        );
    }

    #[test]
    fn alert_picks_highest_active_impact() {
        let theme = AppTheme::default_builtin();
        let summary = Summary {
            page: crate::api::Page {
                id: "p".into(),
                name: "n".into(),
                url: "u".into(),
                updated_at: Utc::now(),
            },
            status: crate::api::Status {
                indicator: crate::api::Indicator::None,
                description: String::new(),
            },
            components: Vec::new(),
            incidents: vec![
                incident("minor", Impact::Minor, IncidentStatus::Investigating),
                incident("major", Impact::Major, IncidentStatus::Identified),
            ],
            scheduled_maintenances: Vec::new(),
        };
        let max = active_max_impact(&summary);
        assert_eq!(max, Some(Impact::Major));
        assert_eq!(alert_color(&theme, max), theme.danger());
        assert_eq!(alert_title(true), "Service Disruption");
    }

    #[test]
    fn alert_falls_back_to_info_for_maintenance_only() {
        let theme = AppTheme::default_builtin();
        let max = None;
        assert_eq!(alert_color(&theme, max), theme.info());
        assert_eq!(alert_title(false), "Scheduled Maintenance");
    }
}
