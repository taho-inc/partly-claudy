# claude-status

A terminal UI for the Claude status page — check Claude's operational status
without leaving your terminal.

```
┌─ claude-status ─────────────────── theme: SilkCircuit Neon · ↻ 12s ago ──┐
│ ● All Systems Operational                                                 │
├───────────────────────────────────────────────────────────────────────────┤
│ Uptime · last 90 days                                                     │
│ Claude.ai          ▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮  99.94%   │
│ Anthropic API      ▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮▮  99.81%   │
│                    90 days ago                                    today   │
├──────────────────────────┬────────────────────────────────────────────────┤
│ Sections                 │ Events                                         │
│ ● Claude.ai              │ ▾ Today                                        │
│ ● Anthropic API          │   ● 14:02 Elevated errors · API · monitoring   │
│ ● anthropic.com          │ ▾ Yesterday                                    │
│                          │   ● Major outage – claude.ai · resolved        │
└──────────────────────────┴────────────────────────────────────────────────┘
```

## Features

- **Overall indicator** — at-a-glance "All Systems Operational" / minor /
  major / critical from the live Statuspage feed.
- **90-day uptime bars** — one row per top-level Claude product, one cell
  per day, colored by the worst severity seen that day. Mirrors the colored
  bars on https://status.claude.com.
- **Sections pane** — the same component groups Claude publishes (Claude.ai,
  Anthropic API, anthropic.com, …).
- **Events pane** — incidents bucketed by calendar day, newest first, with
  status pills.
- **Detail drawer** — slides in from the right (`tui-overlay`), pinned by
  default. Shows incident updates newest-first, component children, or the
  per-day breakdown when scrubbing the bars.
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
claude-status                                     # live data
claude-status --fixture tests/fixtures/summary.json   # offline / demo
claude-status --refresh 30                        # poll every 30s
claude-status --base https://status.example.com   # any Statuspage tenant
```

## Key bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl-c` | Quit |
| `Esc` | Close drawer; if drawer closed, quit |
| `Tab` / `Shift-Tab` | Cycle pane focus (Bars → Sections → Events) |
| `↑` `↓` / `j` `k` | Move selection in focused pane |
| `←` `→` / `h` `l` | Scrub days in the bars pane |
| `Enter` | Pin selection into the drawer |
| `d` | Toggle drawer |
| `r` | Manual refresh |
| `t` | Theme picker |
| `?` | Help |

## Data source

Hits the public Statuspage v2 endpoints under
`https://status.claude.com/api/v2/`:

- `summary.json` — page metadata, overall status, components, unresolved
  incidents, scheduled maintenances. The polling loop only ever calls this.
- `incidents.json` (via fixtures or future expansion) — past incidents,
  used to derive the 90-day uptime bars locally.

The 90-day bars are derived purely from incident windows projected onto
each component group's days; we don't depend on Statuspage's paid uptime
endpoint.

## Layout reference

A static HTML mockup of the layout lives in [`prototype/index.html`](./prototype/index.html).
Open it in any browser side-by-side with the running TUI to compare.

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
