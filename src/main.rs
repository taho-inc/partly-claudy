use std::path::PathBuf;
use std::time::Duration;

use clap::Parser;
use color_eyre::eyre::Result;

mod api;
mod app;
mod bars;
mod events;
mod theme;
mod ui;

#[derive(Parser, Debug)]
#[command(version, about = "Terminal UI for the Claude status page")]
struct Cli {
    /// Read summary JSON from a local fixture instead of the live API.
    #[arg(long, value_name = "PATH")]
    fixture: Option<PathBuf>,

    /// Override the Statuspage base URL (defaults to https://status.claude.com).
    #[arg(long, value_name = "URL")]
    base: Option<String>,

    /// Auto-refresh interval in seconds.
    #[arg(long, default_value_t = 60)]
    refresh: u64,
}

#[tokio::main]
async fn main() -> Result<()> {
    color_eyre::install()?;
    let cli = Cli::parse();

    let source = match cli.fixture {
        Some(p) => api::Source::fixture(p),
        None => api::Source::live(cli.base)?,
    };

    let mut terminal = ratatui::init();
    let result = run(&mut terminal, source, Duration::from_secs(cli.refresh)).await;
    ratatui::restore();
    result
}

async fn run(
    terminal: &mut ratatui::DefaultTerminal,
    source: api::Source,
    refresh: Duration,
) -> Result<()> {
    let theme = theme::AppTheme::load();
    let mut app = app::App::new(theme);
    let mut events = events::EventLoop::new(source, refresh, Duration::from_millis(80));

    loop {
        terminal.draw(|frame| ui::render(frame, &mut app))?;

        let Some(ev) = events.rx.recv().await else { break };
        if let events::AppEvent::Key(k) = &ev {
            if matches!(k.code, crossterm::event::KeyCode::Char('r')) && !app.modal_open() {
                events.refresh_now();
            }
        }
        app.handle(ev);
        if app.quit {
            break;
        }
    }
    Ok(())
}
