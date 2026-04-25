use ratatui::Frame;
use ratatui::layout::Rect;
use tui_skeleton::{AnimationMode, Color as SkColor, SkeletonList};

use crate::app::App;

pub fn render_list(frame: &mut Frame, area: Rect, app: &App, items: u16) {
    if area.width == 0 || area.height == 0 {
        return;
    }
    let base = to_sk(app.theme.panel_bg());
    let highlight = to_sk(app.theme.muted());
    let widget = SkeletonList::new(app.elapsed_ms())
        .mode(AnimationMode::Sweep)
        .items(items)
        .base(base)
        .highlight(highlight);
    frame.render_widget(widget, area);
}

fn to_sk(c: ratatui::style::Color) -> SkColor {
    match c {
        ratatui::style::Color::Rgb(r, g, b) => SkColor::Rgb(r, g, b),
        ratatui::style::Color::Reset => SkColor::Reset,
        ratatui::style::Color::Black => SkColor::Black,
        ratatui::style::Color::Red => SkColor::Red,
        ratatui::style::Color::Green => SkColor::Green,
        ratatui::style::Color::Yellow => SkColor::Yellow,
        ratatui::style::Color::Blue => SkColor::Blue,
        ratatui::style::Color::Magenta => SkColor::Magenta,
        ratatui::style::Color::Cyan => SkColor::Cyan,
        ratatui::style::Color::Gray => SkColor::Gray,
        ratatui::style::Color::DarkGray => SkColor::DarkGray,
        ratatui::style::Color::LightRed => SkColor::LightRed,
        ratatui::style::Color::LightGreen => SkColor::LightGreen,
        ratatui::style::Color::LightYellow => SkColor::LightYellow,
        ratatui::style::Color::LightBlue => SkColor::LightBlue,
        ratatui::style::Color::LightMagenta => SkColor::LightMagenta,
        ratatui::style::Color::LightCyan => SkColor::LightCyan,
        ratatui::style::Color::White => SkColor::White,
        ratatui::style::Color::Indexed(i) => SkColor::Indexed(i),
    }
}
