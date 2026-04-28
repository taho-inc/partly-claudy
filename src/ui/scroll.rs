use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Color, Style};
use ratatui::text::Span;

/// Paint ▲/▼ in the right column of `area` when the visible window
/// hides items above or below. Mirrors the pattern used by
/// `taho-admin/src/theme_drawer.rs` so scroll affordance is consistent
/// across our TUIs.
pub fn overlay(
    frame: &mut Frame,
    area: Rect,
    offset: usize,
    visible: usize,
    total: usize,
    color: Color,
) {
    if area.width == 0 || area.height == 0 {
        return;
    }

    let has_above = offset > 0;
    let has_below = offset + visible < total;
    let x = area.right().saturating_sub(1);

    if has_above {
        let span = Span::styled("▲", Style::default().fg(color));
        frame.buffer_mut().set_span(x, area.y, &span, 1);
    }
    if has_below {
        let span = Span::styled("▼", Style::default().fg(color));
        frame
            .buffer_mut()
            .set_span(x, area.bottom().saturating_sub(1), &span, 1);
    }
}
