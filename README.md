# partly-claudy

[![Built With Ratatui](https://ratatui.rs/built-with-ratatui/badge.svg)](https://ratatui.rs/)
[![Crates.io](https://img.shields.io/crates/v/partly-claudy.svg)](https://crates.io/crates/partly-claudy)
[![CI](https://github.com/taho-inc/partly-claudy/actions/workflows/ci.yml/badge.svg)](https://github.com/taho-inc/partly-claudy/actions/workflows/ci.yml)
[![License: MIT OR Apache-2.0](https://img.shields.io/badge/license-MIT%20OR%20Apache--2.0-blue.svg)](LICENSE-MIT)

A terminal app for the Claude status page. See if Claude is up without leaving your terminal.

![partly-claudy screenshot](https://raw.githubusercontent.com/taho-inc/partly-claudy/main/screenshots/partly-claudy.png)

## Features

- **At-a-glance status.** Overall indicator and uptime for each service on one screen.
- **90-day history.** Each Claude product gets a 90-day uptime bar with a percentage.
- **Incidents timeline.** Every day in the 90-day window, newest first. Open any incident for the full update history.
- **Filter by service.** Pick a service to see only the incidents that affect it.
- **Themes.** Pick from a built-in set or drop in your own.
- **Auto-refresh.** Polls once a minute by default. Press `r` to refresh right away.

## Install

```sh
cargo install partly-claudy
```

Then run it:

```sh
partly-claudy
```

### From a clone

```sh
cargo install --path .
```

Or run it without installing:

```sh
cargo run --release
```

## Usage

```sh
partly-claudy                                         # live data
partly-claudy --refresh 30                            # poll every 30s
```

## Key bindings

| Key | Action |
|-----|--------|
| `q` / `Ctrl-c` | Quit |
| `Esc` | Close the modal. Quit if no modal is open. |
| `Tab` / `Shift-Tab` | Toggle Services and Incidents |
| `↑` `↓` / `j` `k` | Move selection in the focused pane |
| `←` `→` / `h` `l` | Scrub days on the focused service |
| `Enter` | Open detail modal |
| `r` | Manual refresh |
| `t` | Theme picker |
| `?` | Help |

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) and the [Code of Conduct](CODE_OF_CONDUCT.md).

## License

Dual licensed under [MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).

Unless you state otherwise, any contribution you submit for inclusion will be dual licensed as above, with no additional terms or conditions.

---

Built by [TAHO Engineering](https://taho.is).
