use opaline::Theme;
use ratatui::style::{Color, Modifier, Style};

use crate::api::{ComponentStatus, Impact, Indicator};

pub struct AppTheme {
    pub theme: Theme,
    pub name: String,
    pub builtins: Vec<(&'static str, &'static str)>,
    pub selected: usize,
}

impl AppTheme {
    pub fn default_builtin() -> Self {
        let builtins: Vec<(&'static str, &'static str)> = opaline::builtins::builtin_names().to_vec();
        let theme = opaline::builtins::silkcircuit_neon();
        let name = "SilkCircuit Neon".to_string();
        let selected = builtins
            .iter()
            .position(|(id, _)| *id == "silkcircuit-neon")
            .unwrap_or(0);
        Self { theme, name, builtins, selected }
    }

    pub fn cycle(&mut self, delta: isize) {
        let n = self.builtins.len() as isize;
        if n == 0 {
            return;
        }
        let mut next = (self.selected as isize + delta) % n;
        if next < 0 {
            next += n;
        }
        self.selected = next as usize;
        let (id, display) = self.builtins[self.selected];
        if let Some(t) = opaline::builtins::load_by_name(id) {
            self.theme = t;
            self.name = display.to_string();
        }
    }

    pub fn bg(&self) -> Color {
        Color::from(self.theme.color("bg.base"))
    }

    pub fn panel_bg(&self) -> Color {
        Color::from(self.theme.color("bg.panel"))
    }

    pub fn text(&self) -> Color {
        Color::from(self.theme.color("text.primary"))
    }

    pub fn muted(&self) -> Color {
        Color::from(self.theme.color("text.muted"))
    }

    pub fn dim(&self) -> Color {
        let token = if self.theme.has_token("text.dim") { "text.dim" } else { "text.muted" };
        Color::from(self.theme.color(token))
    }

    pub fn accent(&self) -> Color {
        Color::from(self.theme.color("accent.primary"))
    }

    pub fn success(&self) -> Color {
        Color::from(self.theme.color("success"))
    }

    pub fn warning(&self) -> Color {
        Color::from(self.theme.color("warning"))
    }

    pub fn danger(&self) -> Color {
        Color::from(self.theme.color("error"))
    }

    pub fn info(&self) -> Color {
        Color::from(self.theme.color("info"))
    }

    pub fn indicator_color(&self, indicator: Indicator) -> Color {
        match indicator {
            Indicator::None => self.success(),
            Indicator::Minor => self.warning(),
            Indicator::Major => self.danger(),
            Indicator::Critical => self.danger(),
            Indicator::Maintenance => self.info(),
        }
    }

    pub fn component_color(&self, status: ComponentStatus) -> Color {
        match status {
            ComponentStatus::Operational => self.success(),
            ComponentStatus::DegradedPerformance => self.warning(),
            ComponentStatus::PartialOutage => self.warning(),
            ComponentStatus::MajorOutage => self.danger(),
            ComponentStatus::UnderMaintenance => self.info(),
        }
    }

    pub fn impact_color(&self, impact: Impact) -> Color {
        match impact {
            Impact::None => self.muted(),
            Impact::Minor => self.warning(),
            Impact::Major => self.danger(),
            Impact::Critical => self.danger(),
            Impact::Maintenance => self.info(),
        }
    }

    pub fn focused_border(&self) -> Style {
        Style::default().fg(self.accent()).add_modifier(Modifier::BOLD)
    }

    pub fn unfocused_border(&self) -> Style {
        Style::default().fg(self.muted())
    }
}
