use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Paragraph};

use crate::app::{App, Pane};
use crate::bars::{DayStatus, WINDOW_DAYS};
use crate::ui::skeleton;

const NAME_WIDTH: u16 = 18;
const PCT_WIDTH: u16 = 9;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let block = Block::default()
        .title(Line::from(vec![
            Span::styled(" Uptime ", Style::default().fg(app.theme.text()).add_modifier(Modifier::BOLD)),
            Span::styled("· last 90 days ", Style::default().fg(app.theme.muted())),
        ]))
        .borders(Borders::ALL)
        .border_style(if matches!(app.focus, Pane::Bars) {
            app.theme.focused_border()
        } else {
            app.theme.unfocused_border()
        });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_loading() {
        skeleton::render_list(frame, inner, app, 4);
        return;
    }

    let layout = Layout::vertical([Constraint::Min(1), Constraint::Length(1)]).split(inner);
    let rows_area = layout[0];
    let axis_area = layout[1];

    if app.bars.is_empty() {
        frame.render_widget(
            Paragraph::new("(no top-level components)").style(Style::default().fg(app.theme.muted())),
            rows_area,
        );
        return;
    }

    let row_count = app.bars.len() as u16;
    let constraints: Vec<Constraint> = (0..row_count).map(|_| Constraint::Length(1)).collect();
    let row_areas = Layout::vertical(constraints).split(rows_area);

    for (i, row) in app.bars.iter().enumerate() {
        let row_area = row_areas[i];
        render_row(frame, row_area, app, i, row);
    }

    render_axis(frame, axis_area, app);
}

fn render_row(frame: &mut Frame, area: Rect, app: &App, idx: usize, row: &crate::bars::UptimeRow) {
    let cols = Layout::horizontal([
        Constraint::Length(NAME_WIDTH),
        Constraint::Min(10),
        Constraint::Length(PCT_WIDTH),
    ])
    .split(area);

    let focused_row = matches!(app.focus, Pane::Bars) && idx == app.bars_row;
    let name_style = if focused_row {
        Style::default().fg(app.theme.accent()).add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(app.theme.text())
    };
    let name = truncate(&row.group_name, NAME_WIDTH as usize - 1);
    frame.render_widget(Paragraph::new(Span::styled(name, name_style)), cols[0]);

    let bar_area = cols[1];
    let cells = bar_cells(row, bar_area.width as usize);
    let spans: Vec<Span> = cells
        .iter()
        .enumerate()
        .map(|(cell_i, ds)| {
            let color = day_color(app, *ds);
            let glyph = match *ds {
                DayStatus::Unknown => "·",
                _ => "▮",
            };
            let mut style = Style::default().fg(color);
            if focused_row && cell_i == focused_cell(app, cells.len()) {
                style = style.add_modifier(Modifier::REVERSED);
            }
            Span::styled(glyph, style)
        })
        .collect();
    frame.render_widget(Paragraph::new(Line::from(spans)), bar_area);

    let pct = format!(" {:>5.2}%", row.uptime_pct);
    let pct_color = if row.uptime_pct >= 99.9 {
        app.theme.success()
    } else if row.uptime_pct >= 99.0 {
        app.theme.warning()
    } else {
        app.theme.danger()
    };
    frame.render_widget(
        Paragraph::new(Span::styled(pct, Style::default().fg(pct_color))).right_aligned(),
        cols[2],
    );
}

fn render_axis(frame: &mut Frame, area: Rect, app: &App) {
    let cols = Layout::horizontal([
        Constraint::Length(NAME_WIDTH),
        Constraint::Min(10),
        Constraint::Length(PCT_WIDTH),
    ])
    .split(area);

    let bar_area = cols[1];
    let width = bar_area.width as usize;
    if width == 0 {
        return;
    }
    let mut line = String::with_capacity(width);
    for _ in 0..width {
        line.push(' ');
    }
    let left = "90 days ago";
    let right = "today";
    let mut chars: Vec<char> = line.chars().collect();
    for (i, c) in left.chars().enumerate() {
        if i < chars.len() {
            chars[i] = c;
        }
    }
    for (i, c) in right.chars().rev().enumerate() {
        let idx = chars.len().saturating_sub(1 + i);
        chars[idx] = c;
    }
    let label: String = chars.into_iter().collect();
    frame.render_widget(
        Paragraph::new(Span::styled(label, Style::default().fg(app.theme.dim()))),
        bar_area,
    );
}

fn bar_cells(row: &crate::bars::UptimeRow, width: usize) -> Vec<DayStatus> {
    let n = WINDOW_DAYS as usize;
    if width == 0 {
        return Vec::new();
    }
    if width >= n {
        return row.days.clone();
    }
    let mut out = Vec::with_capacity(width);
    for i in 0..width {
        let lo = i * n / width;
        let hi = ((i + 1) * n / width).min(n);
        let mut worst = DayStatus::Operational;
        for d in lo..hi {
            if let Some(s) = row.days.get(d) {
                if *s > worst {
                    worst = *s;
                }
                if matches!(worst, DayStatus::Unknown) && !matches!(*s, DayStatus::Unknown) {
                    worst = *s;
                }
            }
        }
        out.push(worst);
    }
    out
}

fn focused_cell(app: &App, total_cells: usize) -> usize {
    let n = WINDOW_DAYS as usize;
    if total_cells == 0 {
        return 0;
    }
    if total_cells >= n {
        return app.bars_day.min(total_cells - 1);
    }
    (app.bars_day * total_cells) / n
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

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        s.to_string()
    } else {
        let mut out: String = s.chars().take(max.saturating_sub(1)).collect();
        out.push('…');
        out
    }
}
