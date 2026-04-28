# partly-claudy

A terminal UI for the Claude status page — check Claude's operational status
without leaving your terminal.

```
┌─ partly-claudy ─────────────────── theme: SilkCircuit Neon · page 6h ago ──┐
│ ● All Systems Operational                                                   │
│                                                                             │
│  ┌─ ▶ Services ───────────────────────────────────────────────────────┐    │
│  │  ▶ claude.ai                                          Operational  │    │
│  │   ▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮  │    │
│  │   90d ago ─────────────── 98.95% uptime ──────────────────  today  │    │
│  │  ...                                                                │    │
│  └─────────────────────────────────────────────────────────────────────┘    │
│  ┌─   Events ────────────────────────────────────────────────────────┐     │
│  │  ▾ Today · Apr 27                                                  │     │
│  │    (no events)                                                     │     │
│  │  ▾ Sat Apr 25                                                      │     │
│  │    ● 18:42  Elevated errors on claude.ai             resolved      │     │
│  └────────────────────────────────────────────────────────────────────┘     │
└── ↑/↓ service · ←/→ scrub day · Enter detail · Tab next pane ──────────────┘
```

## Features

- **Overall indicator** — at-a-glance "All Systems Operational" / minor /
  major / critical from the live Statuspage feed.
- **Services pane** — one 3-row block per Claude product, mirroring
  `claude.com/status`: dot + name with right-aligned status pill on
  row 1, full-width 90-day uptime bar on row 2, dim axis with centered
  uptime % on row 3. Cursored service filters the Events pane.
- **Events pane** — every day in the 90-day window, newest first, with
  "(no events)" placeholders for clean days.
- **Disruption banner** — bordered block above the body when any active
  incident or scheduled maintenance is present; border tinted by the
  highest active impact (red for major/critical, yellow for minor,
  blue for maintenance-only).
- **Detail modal** — center-anchored overlay (`tui-overlay`) opened by
  Enter, sized to content. Shows incident updates newest-first, or the
  per-day breakdown when opened from a focused service-day cell.
- **Themes** — 39 builtin themes via [`opaline`](https://crates.io/crates/opaline);
  press `t` for a live picker.
- **Skeleton loading** — placeholder rows while the first fetch is in
  flight ([`tui-skeleton`](https://crates.io/crates/tui-skeleton)).
- **Auto-refresh** — every 60s by default; stale data stays on screen
  during a refresh.
- **Offline mode** — `--fixture <path>` reads a Statuspage `summary.json`
  payload from disk so you can develop or demo without network.

## Install

```sh
cargo install --path .
```

Or run from a clone:

```sh
cargo run --release
```

## Usage

```sh
partly-claudy                                     # live data
partly-claudy --fixture tests/fixtures/summary.json   # offline / demo
partly-claudy --refresh 30                        # poll every 30s
partly-claudy --base https://status.example.com   # any Statuspage tenant
```

## Key bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl-c` | Quit |
| `Esc` | Close modal; if modal closed, quit |
| `Tab` / `Shift-Tab` | Toggle Services ↔ Events |
| `↑` `↓` / `j` `k` | Move selection in focused pane |
| `←` `→` / `h` `l` | Scrub days on the focused service |
| `Enter` | Open detail modal |
| `r` | Manual refresh |
| `t` | Theme picker |
| `?` | Help |

## Data source

Three Statuspage endpoints, fetched in two phases:

1. **Parallel:** `summary.json` (page meta + components +
   scheduled_maintenances) · `incidents.json` (~50 most recent typed
   incidents) · `history.json?page=1..2` (incident codes only, three
   calendar months per page; undocumented).
2. **Per-incident detail:** for codes present in history but not in
   the recent typed set, `incidents/{code}.json` is fetched in
   parallel (concurrency capped at 8). Months whose last day falls
   before the 90-day cutoff are filtered before any detail issues.

The uptime % is computed locally by walking each incident's
`incident_updates[*].affected_components[*]` status transitions —
weighted (`major_outage = 1.0`, `partial_outage = 0.5`, others = 0)
and capped at the incident's mitigation timestamp (first
`monitoring | resolved | postmortem` update). Matches the published
per-component 90-day numbers within ~0.2 percentage points without
calling Statuspage's paid `/uptime` endpoint.

## Behavior specs

Living BDD specs in Gherkin live under [`features/`](./features/). They
document the intended behavior of each pane and are not executed by a test
harness.

## Building

```sh
cargo check
cargo clippy --all-targets -- -D warnings
cargo run -- --fixture tests/fixtures/summary.json
```

Requires Rust 1.86+ (Ratatui 0.30 / Edition 2024).

## License

MIT OR Apache-2.0.
