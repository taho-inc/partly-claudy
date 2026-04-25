# claude-status — agent guide

Terminal UI for the Claude status page (Statuspage v2 API). Single binary,
async tokio + ratatui 0.30. Read this before making non-trivial changes.

## Architecture

```
src/main.rs     CLI parsing (clap) → ratatui::init() → run() → ratatui::restore()
src/api.rs      Statuspage v2 DTOs + Source { Live, Fixture } async fetch_summary()
src/bars.rs     compute(components, incidents, today) → Vec<UptimeRow> for 90-day bars
src/events.rs   EventLoop spawns 3 tokio tasks (input / render tick / refresh)
                merges into mpsc<AppEvent> { Key, Tick, Resize, Loaded, LoadFailed }
src/app.rs      App state + handle(AppEvent) reducer (no I/O), Pane enum, DrawerTarget
src/theme.rs    AppTheme wraps opaline::Theme, exposes semantic colors
src/ui/mod.rs   render(frame, &mut app) — header / bars / body / footer + drawer overlay
src/ui/*.rs     header, uptime_bars, sections, timeline, drawer, footer, help,
                theme_picker, skeleton
```

The reducer in `app.rs` is pure: no I/O, no async. All network/disk work is
in `events.rs`, which posts results back through the channel as
`AppEvent::Loaded(Box<Summary>)` or `AppEvent::LoadFailed(String)`. UI
state lives entirely on `App`.

## Key crates and how we use them

- **ratatui 0.30** — `ratatui::init()` / `ratatui::restore()` for terminal
  lifecycle. `Layout::vertical(...)` / `Layout::horizontal(...)` for
  splits. We do **not** use the renamed `HorizontalAlignment` directly;
  `.right_aligned()` / `.left_aligned()` on `Paragraph` is enough.
- **tui-overlay 0.1** — `Overlay::new().anchor(Anchor::Right).slide(Slide::Right)`
  with an `OverlayState` on `App`. Render order: main UI first, then
  `frame.render_stateful_widget(overlay, area, &mut state)`, then read
  `state.inner_area()` and render the drawer body into it. Tick the state
  every frame from `App::handle(AppEvent::Tick)`.
- **tui-skeleton 0.3** — stateless `SkeletonList::new(elapsed_ms)`, pass
  `app.elapsed_ms()`. Wrapped in `src/ui/skeleton.rs` with a
  `ratatui::Color → tui_skeleton::Color` adapter (the crate re-exports its
  own `Color` enum, not ratatui's).
- **opaline 0.4** — `opaline::builtins::silkcircuit_neon()` is the default.
  Themes resolve semantic tokens: `success`, `warning`, `error` (we treat
  as `danger`), `info`. Background tokens: `bg.base`, `bg.panel`. Text
  tokens: `text.primary`, `text.muted`, `text.dim`. Accent:
  `accent.primary`. **Do not** hardcode `Color::Red`/`Color::Green`
  anywhere — go through `AppTheme`.

## Adding a new pane / widget

1. Add a file under `src/ui/`. Pattern: `pub fn render(frame, area, app)`.
2. If the widget needs focus, add a variant to `Pane` in `app.rs` and
   include it in `cycle_focus`'s order array.
3. Borrow colors from `AppTheme` — never hardcode RGB.
4. If the widget shows live data, render `skeleton::render_list(...)` when
   `app.is_loading()` (i.e. `app.summary.is_none()`).

## Adding a new event source

All async work happens in `events.rs`. To add a new background source
(e.g. Statuspage incidents history), spawn another tokio task in
`EventLoop::new`, add a variant to `AppEvent`, and handle it in
`App::handle`. Keep the reducer pure.

## Statuspage API notes

We hit `https://status.claude.com/api/v2/summary.json`. The response is
documented at `https://status.claude.com/api#javascript-library` — note
that the same shape is used by every Atlassian Statuspage tenant. Real
fields we rely on:

- `status.indicator` — `"none" | "minor" | "major" | "critical"` (we also
  accept `"maintenance"` as a fallback).
- `components[]` — `id`, `name`, `status`, `group_id`, `group: bool`,
  `position`, `created_at`. `group: true` means a top-level grouping.
- `incidents[]` and `past_incidents[]` (the latter is fixture-only; the
  live API exposes resolved incidents through `/api/v2/incidents.json`).
  Each has `started_at`, `resolved_at`, `impact`, `incident_updates[]`.

`bars::compute` derives 90-day uptime locally — we do not call Statuspage's
paid `/components/{id}/uptime` endpoint.

## Testing

- `cargo check` — fast, run after every change.
- `cargo clippy --all-targets -- -D warnings` — must be clean.
- `cargo run -- --fixture tests/fixtures/summary.json` — end-to-end
  manual test without network. Adjust the fixture to exercise edge cases
  (missing fields, multi-day outages, scheduled maintenances).

The project does not have automated tests yet. If you add behavior
worthy of a regression test, prefer tests against `bars::compute` and the
DTO deserializers — they're pure functions over data.

## Conventions

- Edition 2024, MSRV 1.86 (set by tui-overlay).
- No emojis in code or commits unless explicitly requested.
- Default to writing no comments — explain WHY only when it's
  non-obvious.
- DTO modules can use `#![allow(dead_code)]` for fields we deserialize but
  don't currently render. Other modules should not.
- Keep the reducer free of I/O; if you need data, dispatch an event.

## Living docs

- `prototype/index.html` — visual layout reference.
- `features/*.feature` — Gherkin behavior specs (not executed).
- `README.md` — user-facing install / usage / bindings.
