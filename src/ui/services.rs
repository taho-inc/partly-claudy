use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, BorderType, Borders, Paragraph};
use ratatui::Frame;

use crate::api::{Component, ComponentStatus};
use crate::app::{App, Pane};
use crate::bars::{DayStatus, UptimeRow, WINDOW_DAYS};
use crate::theme::AppTheme;
use crate::ui::skeleton;

const ROW_HEIGHT: u16 = 3;
const INTER_ROW_GAP: u16 = 1;
const FOCUS_PREFIX: &str = "▶ ";
const NO_PREFIX: &str = "  ";

/// Service rows reserved when the pane is rendering its loading
/// skeleton. Matches the typical Statuspage tenant ('claude.ai',
/// 'Claude Console', 'Claude API', 'Claude Code', 'Claude Cowork',
/// 'Claude for Government') so the layout doesn't reflow on first
/// data arrival.
pub const SKELETON_ROWS: u16 = 6;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let focused_pane = matches!(app.focus, Pane::Services);
    let block = pane_block(" Services ", focused_pane, &app.theme);
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_loading() {
        let n = app.services().len() as u16;
        let n = if n == 0 { SKELETON_ROWS } else { n };
        skeleton::render_services(frame, inner, app, n);
        return;
    }

    let services = app.services();
    if services.is_empty() || app.bars.is_empty() {
        let p = Paragraph::new("(no services)").style(Style::default().fg(app.theme.muted()));
        frame.render_widget(p, inner);
        return;
    }

    let chunks = space_evenly(inner, services.len() as u16);
    for (i, ((comp, row), area)) in services
        .iter()
        .zip(app.bars.iter())
        .zip(chunks.iter())
        .enumerate()
    {
        render_service(frame, *area, app, focused_pane, i, comp, row);
    }
}

/// Vertical layout with a uniform fixed gap between every row, and flex
/// outer margins. Rows centered vertically up to a 1-row remainder.
fn space_evenly(area: Rect, row_count: u16) -> Vec<Rect> {
    let mut constraints = Vec::with_capacity(2 * row_count as usize + 1);
    constraints.push(Constraint::Min(0));
    for i in 0..row_count {
        if i > 0 {
            constraints.push(Constraint::Length(INTER_ROW_GAP));
        }
        constraints.push(Constraint::Length(ROW_HEIGHT));
    }
    constraints.push(Constraint::Min(0));
    Layout::vertical(constraints)
        .split(area)
        .iter()
        .enumerate()
        .filter(|(i, _)| *i % 2 == 1)
        .map(|(_, r)| *r)
        .collect()
}

fn render_service(
    frame: &mut Frame,
    area: Rect,
    app: &App,
    pane_focused: bool,
    idx: usize,
    comp: &Component,
    row: &UptimeRow,
) {
    // The "selected" service stays marked even when focus moves to the
    // Incidents pane, since the incidents shown there are filtered by
    // this selection.
    let selected = idx == app.service_idx;
    let lines = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);
    render_name_row(frame, lines[0], app, selected, comp);
    render_bar_row(frame, lines[1], app, pane_focused && selected, row);
    render_axis_row(frame, lines[2], app, row);
}

fn render_name_row(frame: &mut Frame, area: Rect, app: &App, focused: bool, comp: &Component) {
    let cols = Layout::horizontal([
        Constraint::Min(0),
        Constraint::Length(label(comp.status).chars().count() as u16),
    ])
    .split(area);

    let prefix = if focused { FOCUS_PREFIX } else { NO_PREFIX };
    let name_style = if focused {
        Style::default()
            .fg(app.theme.accent())
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
            .fg(app.theme.text())
            .add_modifier(Modifier::BOLD)
    };
    let name = Line::from(vec![
        Span::styled(
            prefix,
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD),
        ),
        Span::styled(comp.name.clone(), name_style),
    ]);
    frame.render_widget(Paragraph::new(name), cols[0]);

    let status = Line::from(Span::styled(
        label(comp.status),
        Style::default()
            .fg(app.theme.component_color(comp.status))
            .add_modifier(Modifier::BOLD),
    ));
    frame.render_widget(Paragraph::new(status), cols[1]);
}

fn render_bar_row(frame: &mut Frame, area: Rect, app: &App, focused: bool, row: &UptimeRow) {
    let cells = cells_for_width(row, area.width as usize);
    let spans: Vec<Span> = cells
        .iter()
        .map(|(day, ds)| {
            let mut style = Style::default().fg(day_color(app, *ds));
            if focused && *day == app.bars_day {
                style = style.add_modifier(Modifier::REVERSED);
            }
            Span::styled("▮", style)
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), area);
}

fn render_axis_row(frame: &mut Frame, area: Rect, app: &App, row: &UptimeRow) {
    let width = area.width as usize;
    if width == 0 {
        return;
    }
    let left = "90d ago";
    let right = "today";
    let center = format!("{:.2}% uptime", row.uptime_pct);
    let pct_color = if row.uptime_pct >= 99.9 {
        app.theme.success()
    } else if row.uptime_pct >= 99.0 {
        app.theme.warning()
    } else {
        app.theme.danger()
    };

    let l_len = left.chars().count();
    let c_len = center.chars().count();
    let r_len = right.chars().count();
    let needed = l_len + c_len + r_len + 4; // 4 spaces around the dashes

    let dim = Style::default().fg(app.theme.dim());
    if width < needed {
        let line = Line::from(vec![
            Span::styled(format!("{} ", left), dim),
            Span::styled(center, Style::default().fg(pct_color)),
            Span::styled(format!(" {}", right), dim),
        ]);
        frame.render_widget(Paragraph::new(line), area);
        return;
    }

    let total_dashes = width - needed;
    let left_dashes = total_dashes / 2;
    let right_dashes = total_dashes - left_dashes;
    let line = Line::from(vec![
        Span::styled(left, dim),
        Span::styled(format!(" {} ", "─".repeat(left_dashes)), dim),
        Span::styled(
            center,
            Style::default().fg(pct_color).add_modifier(Modifier::BOLD),
        ),
        Span::styled(format!(" {} ", "─".repeat(right_dashes)), dim),
        Span::styled(right, dim),
    ]);
    frame.render_widget(Paragraph::new(line), area);
}

fn cells_for_width(row: &UptimeRow, width: usize) -> Vec<(usize, DayStatus)> {
    let n = WINDOW_DAYS as usize;
    if width == 0 {
        return Vec::new();
    }
    if width >= n {
        // Stretch: each day occupies width/n cells, distributed by (c * n) / width.
        return (0..width)
            .map(|c| {
                let day = (c * n) / width;
                (day, row.days[day])
            })
            .collect();
    }
    (0..width)
        .map(|c| {
            let lo = c * n / width;
            let hi = ((c + 1) * n / width).min(n);
            let mut worst = DayStatus::Unknown;
            let mut day_for_focus = lo;
            for d in lo..hi {
                let s = row.days[d];
                if matches!(s, DayStatus::Unknown) {
                    continue;
                }
                if matches!(worst, DayStatus::Unknown) || s > worst {
                    worst = s;
                    day_for_focus = d;
                }
            }
            (day_for_focus, worst)
        })
        .collect()
}

fn day_color(app: &App, ds: DayStatus) -> ratatui::style::Color {
    match ds {
        DayStatus::Unknown => app.theme.dim(),
        DayStatus::Operational => app.theme.success(),
        DayStatus::Maintenance => app.theme.info(),
        DayStatus::Degraded => app.theme.warning(),
        DayStatus::Outage => app.theme.danger(),
    }
}

/// Natural height of the Services block: chrome + N rows + N-1 gaps + a
/// little headroom so the space-evenly margins have something to absorb.
pub fn block_height(n_services: usize) -> u16 {
    let n = n_services as u16;
    let rows = n.saturating_mul(ROW_HEIGHT);
    let gaps = n.saturating_sub(1).saturating_mul(INTER_ROW_GAP);
    rows.saturating_add(gaps)
        .saturating_add(2)
        .saturating_add(2)
}

/// Standard pane chrome used by `services` and `timeline`: rounded
/// border, focus-dependent border color, and a `pane_title` leader.
pub fn pane_block(label: &str, focused: bool, theme: &AppTheme) -> Block<'static> {
    Block::default()
        .title(pane_title(label, focused, theme))
        .borders(Borders::ALL)
        .border_type(BorderType::Rounded)
        .border_style(if focused {
            theme.focused_border()
        } else {
            theme.unfocused_border()
        })
}

pub fn pane_title(label: &str, focused: bool, theme: &AppTheme) -> Line<'static> {
    let label_style = Style::default()
        .fg(theme.text())
        .add_modifier(Modifier::BOLD);
    if focused {
        Line::from(vec![
            Span::styled(
                " ▶ ",
                Style::default()
                    .fg(theme.accent())
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(label.trim_start().to_string(), label_style),
        ])
    } else {
        Line::from(Span::styled(label.to_string(), label_style))
    }
}

fn label(s: ComponentStatus) -> &'static str {
    match s {
        ComponentStatus::Operational => "Operational",
        ComponentStatus::DegradedPerformance => "Degraded",
        ComponentStatus::PartialOutage => "Partial Outage",
        ComponentStatus::MajorOutage => "Major Outage",
        ComponentStatus::UnderMaintenance => "Maintenance",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn row_with(days: Vec<DayStatus>) -> UptimeRow {
        UptimeRow {
            group_id: "g".into(),
            group_name: "g".into(),
            days,
            uptime_pct: 100.0,
        }
    }

    #[test]
    fn stretches_to_fill_when_width_exceeds_window() {
        let row = row_with(vec![DayStatus::Operational; WINDOW_DAYS as usize]);
        let cells = cells_for_width(&row, 90);
        assert_eq!(cells.len(), 90, "every char in bar_area gets a cell");
        let day_set: std::collections::BTreeSet<usize> = cells.iter().map(|(d, _)| *d).collect();
        assert_eq!(day_set.len(), WINDOW_DAYS as usize, "every day represented");
    }

    #[test]
    fn coalesces_when_narrower_than_window() {
        let mut days = vec![DayStatus::Operational; WINDOW_DAYS as usize];
        days[15] = DayStatus::Outage;
        let cells = cells_for_width(&row_with(days), 10);
        assert_eq!(cells.len(), 10);
        assert!(cells.iter().any(|(_, ds)| matches!(ds, DayStatus::Outage)));
    }

    #[test]
    fn coalesce_preserves_all_unknown_runs() {
        let mut days = vec![DayStatus::Operational; WINDOW_DAYS as usize];
        for d in days.iter_mut().take(10) {
            *d = DayStatus::Unknown;
        }
        let cells = cells_for_width(&row_with(days), 10);
        assert!(matches!(cells[0].1, DayStatus::Unknown));
    }
}
