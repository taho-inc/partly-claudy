use ratatui::Frame;
use ratatui::layout::{Constraint, Layout, Rect};

use crate::app::App;

pub mod drawer;
pub mod footer;
pub mod header;
pub mod help;
pub mod sections;
pub mod skeleton;
pub mod theme_picker;
pub mod timeline;
pub mod uptime_bars;

pub fn render(frame: &mut Frame, app: &mut App) {
    let area = frame.area();
    let bg = app.theme.bg();
    fill_bg(frame, area, bg);

    let bars_height = bars_height(app);
    let chunks = Layout::vertical([
        Constraint::Length(2),                  // header
        Constraint::Length(bars_height),        // uptime bars
        Constraint::Min(8),                     // body
        Constraint::Length(1),                  // footer
    ])
    .split(area);

    header::render(frame, chunks[0], app);
    uptime_bars::render(frame, chunks[1], app);

    let body = Layout::horizontal([Constraint::Percentage(40), Constraint::Min(0)]).split(chunks[2]);
    sections::render(frame, body[0], app);
    timeline::render(frame, body[1], app);

    footer::render(frame, chunks[3], app);

    drawer::render(frame, area, app);

    if app.help_open {
        help::render(frame, area, app);
    }
    if app.theme_picker_open {
        theme_picker::render(frame, area, app);
    }
}

fn bars_height(app: &App) -> u16 {
    let rows = app.bars.len().max(1) as u16;
    rows + 3
}

fn fill_bg(frame: &mut Frame, area: Rect, bg: ratatui::style::Color) {
    use ratatui::style::Style;
    use ratatui::widgets::Block;
    let block = Block::new().style(Style::default().bg(bg));
    frame.render_widget(block, area);
}
