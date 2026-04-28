use ratatui::layout::{Constraint, Layout, Rect};
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, Clear, List, ListItem, ListState};
use ratatui::Frame;

use crate::app::App;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let popup = centered(area, 50, 20);
    frame.render_widget(Clear, popup);

    let hint = Line::from(vec![
        Span::raw(" "),
        Span::styled("[enter]", Style::default().fg(app.theme.accent())),
        Span::styled(" select · ", Style::default().fg(app.theme.dim())),
        Span::styled("[esc]", Style::default().fg(app.theme.accent())),
        Span::styled(" cancel ", Style::default().fg(app.theme.dim())),
    ])
    .right_aligned();
    let block = Block::default()
        .title(Span::styled(
            " Theme picker ",
            Style::default()
                .fg(app.theme.text())
                .add_modifier(Modifier::BOLD),
        ))
        .title_bottom(hint)
        .borders(Borders::ALL)
        .border_style(Style::default().fg(app.theme.accent()))
        .style(Style::default().bg(app.theme.panel_bg()));
    let inner = block.inner(popup);
    frame.render_widget(block, popup);

    let list_area = Rect {
        width: inner.width.saturating_sub(1),
        ..inner
    };
    let items: Vec<ListItem> = app
        .theme
        .entries
        .iter()
        .map(|entry| {
            ListItem::new(Line::from(Span::styled(
                entry.display.clone(),
                Style::default().fg(app.theme.text()),
            )))
        })
        .collect();
    let total_items = items.len();
    let mut state = ListState::default();
    state.select(Some(app.theme.cursor));
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol(Line::from(" ▶ "));
    frame.render_stateful_widget(list, list_area, &mut state);

    crate::ui::scroll::overlay(
        frame,
        inner,
        state.offset(),
        list_area.height as usize,
        total_items,
        app.theme.dim(),
    );
}

fn centered(area: Rect, width: u16, height: u16) -> Rect {
    let h_pad = area.width.saturating_sub(width) / 2;
    let v_pad = area.height.saturating_sub(height) / 2;
    let v = Layout::vertical([
        Constraint::Length(v_pad),
        Constraint::Length(height),
        Constraint::Min(0),
    ])
    .split(area);
    let h = Layout::horizontal([
        Constraint::Length(h_pad),
        Constraint::Length(width),
        Constraint::Min(0),
    ])
    .split(v[1]);
    h[1]
}
