use std::path::PathBuf;
use std::time::{Duration, Instant};

use opaline::Theme;
use ratatui::style::{Color, Modifier, Style};

use crate::api::{ComponentStatus, Impact, Indicator};

/// Quiet window before the picker cursor commits as the active theme.
/// Mitigates seizure-inducing flashes when scrolling rapidly through
/// the theme list.
const PREVIEW_DEBOUNCE: Duration = Duration::from_millis(120);

/// Theme used on first launch when no preference has been persisted.
/// Matched against `ThemeEntry::display` (the meta.name from the TOML).
const DEFAULT_THEME_NAME: &str = "Claude Code Dark";

/// Themes embedded into the binary, in picker order. Pinned ahead of
/// opaline builtins and any user themes discovered under `~/.taho/themes/`.
const EMBEDDED_TOMLS: &[&str] = &[
    include_str!("../themes/taho_dark.toml"),
    include_str!("../themes/taho_light.toml"),
    include_str!("../themes/claude_code_dark.toml"),
    include_str!("../themes/claude_code_light.toml"),
];

pub struct AppTheme {
    pub entries: Vec<ThemeEntry>,
    /// Currently active theme — drives all rendering and persistence.
    pub applied: usize,
    /// Picker highlight position. Diverges from `applied` while the
    /// debounce window is open and converges on tick or commit.
    pub cursor: usize,
    pending_since: Option<Instant>,
}

pub struct ThemeEntry {
    pub display: String,
    pub theme: Theme,
}

impl From<Theme> for ThemeEntry {
    fn from(theme: Theme) -> Self {
        Self {
            display: theme.meta.name.clone(),
            theme,
        }
    }
}

impl AppTheme {
    pub fn load() -> Self {
        let mut entries: Vec<ThemeEntry> = Vec::new();

        for src in EMBEDDED_TOMLS {
            if let Ok(theme) = opaline::load_from_str(src, None) {
                entries.push(theme.into());
            }
        }

        let mut tail: Vec<ThemeEntry> = Vec::new();

        for (id, _) in opaline::builtins::builtin_names() {
            if let Some(theme) = opaline::builtins::load_by_name(id) {
                tail.push(theme.into());
            }
        }

        if let Ok(read) = std::fs::read_dir(taho_themes_dir()) {
            let mut paths: Vec<PathBuf> = read
                .filter_map(Result::ok)
                .map(|e| e.path())
                .filter(|p| p.extension().is_some_and(|ext| ext == "toml"))
                .collect();
            paths.sort();
            for path in paths {
                if let Ok(theme) = opaline::load_from_file(&path) {
                    tail.push(theme.into());
                }
            }
        }

        tail.sort_by_key(|t| t.display.to_lowercase());
        tail.dedup_by(|a, b| a.display == b.display);
        entries.extend(tail);

        let find = |name: &str| entries.iter().position(|e| e.display == name);
        let applied = load_preference()
            .and_then(|name| find(&name))
            .or_else(|| find(DEFAULT_THEME_NAME))
            .unwrap_or(0);

        Self {
            entries,
            applied,
            cursor: applied,
            pending_since: None,
        }
    }

    pub fn theme(&self) -> &Theme {
        &self.entries[self.applied].theme
    }

    pub fn name(&self) -> &str {
        &self.entries[self.applied].display
    }

    /// Move the picker cursor and arm the preview debounce. The active
    /// theme stays untouched until [`tick`] elapses the quiet window
    /// or [`commit_preview`] is called.
    pub fn cycle(&mut self, delta: isize) {
        let n = self.entries.len() as isize;
        if n == 0 {
            return;
        }
        let mut next = (self.cursor as isize + delta) % n;
        if next < 0 {
            next += n;
        }
        self.cursor = next as usize;
        self.pending_since = Some(Instant::now());
    }

    /// Apply the cursor selection immediately and persist it. Called
    /// when the picker closes (Enter / Esc) so the close transition
    /// isn't gated on the debounce window.
    pub fn commit_preview(&mut self) {
        if self.cursor != self.applied {
            self.applied = self.cursor;
            save_preference(self.name());
        }
        self.pending_since = None;
    }

    /// Promote cursor → applied once the debounce window has elapsed.
    /// Idempotent and cheap to call every Tick.
    pub fn tick(&mut self) {
        if let Some(at) = self.pending_since
            && at.elapsed() >= PREVIEW_DEBOUNCE
        {
            self.commit_preview();
        }
    }

    pub fn bg(&self) -> Color {
        Color::from(self.theme().color("bg.base"))
    }

    pub fn panel_bg(&self) -> Color {
        Color::from(self.theme().color("bg.panel"))
    }

    pub fn text(&self) -> Color {
        Color::from(self.theme().color("text.primary"))
    }

    pub fn muted(&self) -> Color {
        Color::from(self.theme().color("text.muted"))
    }

    pub fn dim(&self) -> Color {
        let token = if self.theme().has_token("text.dim") {
            "text.dim"
        } else {
            "text.muted"
        };
        Color::from(self.theme().color(token))
    }

    pub fn accent(&self) -> Color {
        Color::from(self.theme().color("accent.primary"))
    }

    pub fn success(&self) -> Color {
        Color::from(self.theme().color("success"))
    }

    pub fn warning(&self) -> Color {
        Color::from(self.theme().color("warning"))
    }

    pub fn danger(&self) -> Color {
        Color::from(self.theme().color("error"))
    }

    pub fn info(&self) -> Color {
        Color::from(self.theme().color("info"))
    }

    pub fn indicator_color(&self, indicator: Indicator) -> Color {
        match indicator {
            Indicator::None => self.success(),
            Indicator::Minor => self.warning(),
            Indicator::Major | Indicator::Critical => self.danger(),
            Indicator::Maintenance => self.info(),
        }
    }

    pub fn component_color(&self, status: ComponentStatus) -> Color {
        match status {
            ComponentStatus::Operational => self.success(),
            ComponentStatus::DegradedPerformance | ComponentStatus::PartialOutage => self.warning(),
            ComponentStatus::MajorOutage => self.danger(),
            ComponentStatus::UnderMaintenance => self.info(),
        }
    }

    pub fn impact_color(&self, impact: Impact) -> Color {
        match impact {
            Impact::None => self.muted(),
            Impact::Minor => self.warning(),
            Impact::Major | Impact::Critical => self.danger(),
            Impact::Maintenance => self.info(),
        }
    }

    pub fn focused_border(&self) -> Style {
        Style::default()
            .fg(self.accent())
            .add_modifier(Modifier::BOLD)
    }

    pub fn unfocused_border(&self) -> Style {
        Style::default().fg(self.muted())
    }
}

fn taho_dir() -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_else(|_| ".".into());
    PathBuf::from(home).join(".taho")
}

fn taho_themes_dir() -> PathBuf {
    taho_dir().join("themes")
}

fn settings_path() -> PathBuf {
    taho_dir().join("partly-claudy").join("settings.toml")
}

fn load_preference() -> Option<String> {
    let text = std::fs::read_to_string(settings_path()).ok()?;
    let table: toml::Table = text.parse().ok()?;
    table
        .get("theme")
        .and_then(|v| v.as_str())
        .map(String::from)
}

fn save_preference(name: &str) {
    let path = settings_path();

    let mut table: toml::Table = std::fs::read_to_string(&path)
        .ok()
        .and_then(|s| s.parse().ok())
        .unwrap_or_default();

    table.insert("theme".into(), toml::Value::String(name.into()));

    if let Some(parent) = path.parent() {
        let _ = std::fs::create_dir_all(parent);
    }
    let _ = std::fs::write(&path, table.to_string());
}
