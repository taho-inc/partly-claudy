use ratatui::Frame;
use ratatui::buffer::Buffer;
use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Color, Modifier, Style};
use tui_skeleton::{AnimationMode, Color as SkColor, SkeletonBlock, SkeletonStreamingText};

use crate::app::App;

/// Per-row layout: name+status / bar / axis. Mirrors `services::ROW_HEIGHT`.
const SERVICE_ROW: u16 = 3;
const SERVICE_GAP: u16 = 1;

/// Skeleton for the Services pane, mirroring the loaded layout:
/// each row has a name block (left) + status pill (right), an uptime
/// braille bar, and a 3-marker axis row.
pub fn render_services(frame: &mut Frame, area: Rect, app: &App, n: u16) {
    let area = inset_one(area);
    if area.width == 0 || area.height == 0 || n == 0 {
        return;
    }

    let rows = layout_rows(area, n);
    for row in rows {
        render_service_row(frame, row, app);
    }
}

fn render_service_row(frame: &mut Frame, area: Rect, app: &App) {
    if area.height < SERVICE_ROW {
        return;
    }
    let lines = Layout::vertical([
        Constraint::Length(1),
        Constraint::Length(1),
        Constraint::Length(1),
    ])
    .split(area);

    let elapsed = app.loading_elapsed_ms();
    let base = to_sk(app.theme.panel_bg());
    let highlight = to_sk(app.theme.muted());

    let name_w = area.width.saturating_div(3).clamp(8, 22);
    let status_w = 11.min(area.width);
    let axis_w = area.width;
    let left_w = 7.min(axis_w);
    let center_w = 14.min(axis_w);
    let right_w = 5.min(axis_w);
    let center_x = area.x + axis_w.saturating_sub(center_w) / 2;
    let right_x = area.x + axis_w.saturating_sub(right_w);
    let status_x = area.x + area.width.saturating_sub(status_w);

    // (rect, braille). The `true` row is the braille uptime bar; everything
    // else is a solid-fill placeholder for a text segment.
    let blocks: [(Rect, bool); 6] = [
        (row_rect(area, lines[0].y, area.x, name_w), false),
        (row_rect(area, lines[0].y, status_x, status_w), false),
        (lines[1], true),
        (row_rect(area, lines[2].y, area.x, left_w), false),
        (row_rect(area, lines[2].y, center_x, center_w), false),
        (row_rect(area, lines[2].y, right_x, right_w), false),
    ];
    for (rect, braille) in blocks {
        block(frame, rect, elapsed, base, highlight, braille);
    }
}

fn row_rect(parent: Rect, y: u16, x: u16, width: u16) -> Rect {
    Rect {
        x,
        y,
        width,
        height: 1,
    }
    .intersection(parent)
}

/// Skeleton for the Incidents pane: a sequence of grouped headings
/// each followed by bullet rows that stream in via
/// `SkeletonStreamingText`.
pub fn render_incidents(frame: &mut Frame, area: Rect, app: &App) {
    let area = inset_one(area);
    if area.width == 0 || area.height == 0 {
        return;
    }

    // Heading width, bullet count for each visible group. Stops when
    // we run out of vertical space.
    const GROUPS: &[(u16, u16)] = &[(16, 3), (18, 1), (22, 4), (20, 2)];

    let elapsed = app.loading_elapsed_ms();
    let base = to_sk(app.theme.panel_bg());
    let highlight = to_sk(app.theme.muted());

    let mut y = area.y;
    let max_y = area.y + area.height;

    for (heading_w, bullets) in GROUPS {
        if y >= max_y {
            break;
        }
        let heading_rect = Rect {
            x: area.x,
            y,
            width: (*heading_w).min(area.width),
            height: 1,
        };
        block(frame, heading_rect, elapsed, base, highlight, false);
        y += 1;

        let bullet_indent = 3.min(area.width);
        let stream_x = area.x + bullet_indent + 2;
        if stream_x >= area.x + area.width {
            continue;
        }
        let stream_w = area.width.saturating_sub(bullet_indent + 2);
        let available = max_y.saturating_sub(y);
        let lines = (*bullets).min(available);
        if lines == 0 {
            break;
        }

        for i in 0..lines {
            paint_char(
                frame.buffer_mut(),
                area.x + bullet_indent,
                y + i,
                '●',
                Style::default()
                    .fg(app.theme.dim())
                    .add_modifier(Modifier::BOLD),
            );
        }

        let stream_rect = Rect {
            x: stream_x,
            y,
            width: stream_w,
            height: lines,
        };
        let widths: &[f32] = &[0.78, 0.62, 0.85, 0.55, 0.70];
        let widget = SkeletonStreamingText::new(elapsed)
            .mode(AnimationMode::Sweep)
            .lines(lines)
            .duration_ms(2500)
            .repeat(true)
            .line_widths(widths)
            .base(base)
            .highlight(highlight);
        frame.render_widget(widget, stream_rect);
        y += lines + 1;
    }
}

fn block(
    frame: &mut Frame,
    area: Rect,
    elapsed: u64,
    base: SkColor,
    highlight: SkColor,
    braille: bool,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let widget = SkeletonBlock::new(elapsed)
        .braille(braille)
        .mode(AnimationMode::Sweep)
        .base(base)
        .highlight(highlight);
    frame.render_widget(widget, area);
}

fn paint_char(buf: &mut Buffer, x: u16, y: u16, ch: char, style: Style) {
    if x >= buf.area.x + buf.area.width || y >= buf.area.y + buf.area.height {
        return;
    }
    let cell = &mut buf[(x, y)];
    cell.set_char(ch);
    cell.set_style(style);
}

fn inset_one(area: Rect) -> Rect {
    Rect {
        x: area.x.saturating_add(1),
        y: area.y.saturating_add(1),
        width: area.width.saturating_sub(2),
        height: area.height.saturating_sub(2),
    }
}

/// Vertical-stack `n` rows of [`SERVICE_ROW`] separated by [`SERVICE_GAP`]
/// against the top of `area`. Returns each row's `Rect`. Truncates rows
/// that would overflow.
fn layout_rows(area: Rect, n: u16) -> Vec<Rect> {
    let mut rows = Vec::with_capacity(n as usize);
    let mut y = area.y;
    let max_y = area.y + area.height;
    for i in 0..n {
        if i > 0 {
            y += SERVICE_GAP;
        }
        if y + SERVICE_ROW > max_y {
            break;
        }
        rows.push(Rect {
            x: area.x,
            y,
            width: area.width,
            height: SERVICE_ROW,
        });
        y += SERVICE_ROW;
    }
    rows
}

fn to_sk(c: Color) -> SkColor {
    match c {
        Color::Rgb(r, g, b) => SkColor::Rgb(r, g, b),
        Color::Reset => SkColor::Reset,
        Color::Black => SkColor::Black,
        Color::Red => SkColor::Red,
        Color::Green => SkColor::Green,
        Color::Yellow => SkColor::Yellow,
        Color::Blue => SkColor::Blue,
        Color::Magenta => SkColor::Magenta,
        Color::Cyan => SkColor::Cyan,
        Color::Gray => SkColor::Gray,
        Color::DarkGray => SkColor::DarkGray,
        Color::LightRed => SkColor::LightRed,
        Color::LightGreen => SkColor::LightGreen,
        Color::LightYellow => SkColor::LightYellow,
        Color::LightBlue => SkColor::LightBlue,
        Color::LightMagenta => SkColor::LightMagenta,
        Color::LightCyan => SkColor::LightCyan,
        Color::White => SkColor::White,
        Color::Indexed(i) => SkColor::Indexed(i),
    }
}
