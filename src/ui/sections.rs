use ratatui::Frame;
use ratatui::layout::Rect;
use ratatui::style::{Modifier, Style};
use ratatui::text::{Line, Span};
use ratatui::widgets::{Block, Borders, List, ListItem, ListState};

use crate::api::Component;
use crate::app::{App, Pane};
use crate::ui::skeleton;

pub fn render(frame: &mut Frame, area: Rect, app: &App) {
    let focused = matches!(app.focus, Pane::Sections);
    let block = Block::default()
        .title(Span::styled(" Sections ", Style::default().fg(app.theme.text()).add_modifier(Modifier::BOLD)))
        .borders(Borders::ALL)
        .border_style(if focused { app.theme.focused_border() } else { app.theme.unfocused_border() });
    let inner = block.inner(area);
    frame.render_widget(block, area);

    if app.is_loading() {
        skeleton::render_list(frame, inner, app, 6);
        return;
    }

    let rows = app.section_rows();
    let items: Vec<ListItem> = rows.iter().map(|c| section_item(app, c)).collect();
    let mut state = ListState::default();
    if !rows.is_empty() {
        state.select(Some(app.section_idx.min(rows.len() - 1)));
    }
    let list = List::new(items)
        .highlight_style(
            Style::default()
                .fg(app.theme.accent())
                .add_modifier(Modifier::BOLD | Modifier::REVERSED),
        )
        .highlight_symbol(Line::from(" ▶ "));
    frame.render_stateful_widget(list, inner, &mut state);
}

fn section_item<'a>(app: &App, c: &'a Component) -> ListItem<'a> {
    let dot_color = app.theme.component_color(c.status);
    let mut spans = vec![
        Span::styled("● ", Style::default().fg(dot_color).add_modifier(Modifier::BOLD)),
        Span::styled(c.name.clone(), Style::default().fg(app.theme.text())),
    ];
    if c.group {
        spans.push(Span::styled("  group", Style::default().fg(app.theme.dim())));
    }
    spans.push(Span::raw("  "));
    spans.push(Span::styled(label(c.status), Style::default().fg(dot_color)));
    ListItem::new(Line::from(spans))
}

pub fn label(s: crate::api::ComponentStatus) -> &'static str {
    use crate::api::ComponentStatus::*;
    match s {
        Operational => "operational",
        DegradedPerformance => "degraded",
        PartialOutage => "partial outage",
        MajorOutage => "major outage",
        UnderMaintenance => "maintenance",
    }
}
