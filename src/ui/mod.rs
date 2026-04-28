use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::app::App;

pub mod banner;
pub mod footer;
pub mod header;
pub mod help;
pub mod modal;
pub mod scroll;
pub mod services;
pub mod skeleton;
pub mod theme_picker;
pub mod timeline;

/// Maximum content width for the stacked body. Wider terminals get
/// horizontal padding rather than letting rows stretch indefinitely.
const BODY_MAX_WIDTH: u16 = 100;

pub fn render(frame: &mut Frame, app: &mut App) {
    let full = frame.area();
    fill_bg(frame, full, app.theme.bg());
    let content = inset(full, 1);

    let alert = banner::build(app);
    let banner_h = alert.as_ref().map_or(0, |a| a.block_height());
    let chunks = Layout::vertical([
        Constraint::Length(2),        // header
        Constraint::Length(banner_h), // banner (0 when empty)
        Constraint::Min(8),           // body
        Constraint::Length(1),        // footer
    ])
    .split(content);

    header::render(frame, chunks[0], app);
    if let Some(alert) = &alert {
        banner::render(frame, centered(chunks[1], BODY_MAX_WIDTH), alert, app);
    }

    let body = centered(chunks[2], BODY_MAX_WIDTH);
    let count = if app.is_loading() {
        services::SKELETON_ROWS as usize
    } else {
        app.services().len()
    };
    let services_h = services::block_height(count).min(body.height);
    let body_split =
        Layout::vertical([Constraint::Length(services_h), Constraint::Min(4)]).split(body);
    services::render(frame, body_split[0], app);
    timeline::render(frame, body_split[1], app);

    footer::render(frame, chunks[3], app);

    modal::render(frame, full, app);

    if app.help_open {
        help::render(frame, full, app);
    }
    if app.theme_picker_open {
        theme_picker::render(frame, full, app);
    }
}

fn inset(rect: Rect, padding: u16) -> Rect {
    Rect {
        x: rect.x + padding,
        y: rect.y + padding,
        width: rect.width.saturating_sub(padding * 2),
        height: rect.height.saturating_sub(padding * 2),
    }
}

fn centered(area: Rect, max_width: u16) -> Rect {
    if area.width <= max_width {
        return area;
    }
    let pad = (area.width - max_width) / 2;
    Rect {
        x: area.x + pad,
        width: max_width,
        ..area
    }
}

fn fill_bg(frame: &mut Frame, area: Rect, bg: ratatui::style::Color) {
    use ratatui::style::Style;
    use ratatui::widgets::Block;
    let block = Block::new().style(Style::default().bg(bg));
    frame.render_widget(block, area);
}
