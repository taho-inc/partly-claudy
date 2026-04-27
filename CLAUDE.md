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
src/app.rs      App state + handle(AppEvent) reducer (no I/O), Pane enum, ModalTarget
src/theme.rs    AppTheme wraps opaline::Theme, exposes semantic colors
src/ui/mod.rs   render(frame, &mut app) — header / banner / body / footer + modal
src/ui/*.rs     header, banner, services (dot+name+bars+pct+meta per row),
                timeline, modal, footer, help, theme_picker, skeleton
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
- **tui-overlay 0.1** — `Overlay::new().anchor(Anchor::Center).slide(Slide::Top)`
  for the detail modal, with an `OverlayState` on `App`. Render order:
  main UI first, then `frame.render_stateful_widget(overlay, area, &mut state)`,
  then read `state.inner_area()` and render the modal body into it. Tick
  the state every frame from `App::handle(AppEvent::Tick)`. Modal opens
  on Enter, closes on Esc.
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
- `incidents[]` — populated by [`api::fetch_live`](src/api.rs) from
  three merged sources: `/api/v2/summary.json` (page meta +
  scheduled_maintenances), `/api/v2/incidents.json` (typed, ~50 most
  recent), and per-incident detail fetches for older codes discovered
  via `/history.json?page=N` (undocumented; codes only). Each
  incident carries `started_at`, `resolved_at`, `impact`,
  `components[]`, `incident_updates[]` (with `affected_components[]`
  recording per-component status transitions).

Uptime math (`bars::compute` + `accumulate_component_downtime`):

- Walk each incident's `incident_updates[*].affected_components[]`
  to build a per-component status timeline. Each transition opens a
  span ending at the next transition (same component) or earlier.
- Cap every span at the incident's **mitigation timestamp** —
  earliest update reaching `Monitoring | Resolved | Postmortem`.
  Statuspage stops counting downtime once an incident moves past
  identification, even if the affected_components payload still
  reads partial_outage during verification.
- Span weight: `major_outage → 1.0`, `partial_outage → 0.5`,
  everything else (degraded_performance, under_maintenance,
  operational) → 0.0.
- `Impact::None` and `Impact::Maintenance` incidents are skipped.

This matches Statuspage's published per-component 90-day uptime
within ~0.2 percentage points across all six Claude services
(verified by reverse-engineering the JSON embedded in their home
page HTML). We don't call the paid `/components/{id}/uptime` endpoint.

Fetch strategy:

- Phase 1: `summary.json` + `incidents.json` + 2 history pages in
  parallel (`tokio::try_join!` + `stream::buffer_unordered`).
- Phase 2: for codes in history but not in the recent typed set,
  parallel-fetch `/api/v2/incidents/{code}.json` with concurrency
  capped at `DETAIL_FETCH_CONCURRENCY = 8`. History months whose
  last day falls before the 90-day cutoff are filtered out before
  detail fetches issue.
- All history calls are best-effort; failures don't fail the load.

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

- `features/*.feature` — Gherkin behavior specs (not executed).
- `README.md` — user-facing install / usage / bindings.

## Current Focus

The Statuspage-mirror milestone is closed. Within ~0.2 percentage
points of Claude's published 90-day uptime numbers across all six
services. Layout matches the web hierarchy: one 3-row block per
service (name + status / full-width bar / axis with centered %).
Detail modal sized to content; severity-tinted disruption banner
when active incidents or scheduled maintenances are present.

Next-up candidates (not in flight):

- Open the selected incident's `shortlink` in a browser via `o`
  (adds the `open` crate; cross-platform spawn).
- Memoize older history months across refreshes — they don't change,
  re-fetching every 60s burns ~125 detail requests against a public
  endpoint with no rate-limit headers.
- Filter incidents by name (`/`) — UI hint was removed; add the input.
