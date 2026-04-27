use std::collections::HashMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, NaiveDate, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_overlay::OverlayState;

use crate::api::{Component, Incident, Summary};
use crate::bars::{self, UptimeRow};
use crate::events::AppEvent;
use crate::theme::AppTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Services,
    Events,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum ModalTarget {
    Empty,
    Incident(String),
    Day { row: usize, day: usize },
}

pub struct App {
    pub started_at: Instant,
    pub theme: AppTheme,
    pub overlay: OverlayState,
    pub focus: Pane,
    pub summary: Option<Summary>,
    pub bars: Vec<UptimeRow>,
    pub error: Option<String>,
    pub last_loaded_at: Option<DateTime<Utc>>,
    pub last_attempt_at: Option<Instant>,
    pub help_open: bool,
    pub theme_picker_open: bool,
    pub bars_day: usize,
    pub service_idx: usize,
    pub event_idx: usize,
    pub modal_target: ModalTarget,
    pub quit: bool,
    pub status_toast: Option<String>,
    pub toast_until: Option<Instant>,
}

impl App {
    pub fn new(theme: AppTheme) -> Self {
        Self {
            started_at: Instant::now(),
            theme,
            overlay: OverlayState::new(),
            focus: Pane::Services,
            summary: None,
            bars: Vec::new(),
            error: None,
            last_loaded_at: None,
            last_attempt_at: None,
            help_open: false,
            theme_picker_open: false,
            bars_day: bars::WINDOW_DAYS as usize - 1,
            service_idx: 0,
            event_idx: 0,
            modal_target: ModalTarget::Empty,
            quit: false,
            status_toast: None,
            toast_until: None,
        }
    }

    pub fn elapsed_ms(&self) -> u64 {
        self.started_at.elapsed().as_millis() as u64
    }

    pub fn is_loading(&self) -> bool {
        self.summary.is_none()
    }

    pub fn services(&self) -> Vec<&Component> {
        let Some(s) = self.summary.as_ref() else {
            return vec![];
        };
        let mut out: Vec<&Component> = s
            .components
            .iter()
            .filter(|c| c.group || c.group_id.is_none())
            .collect();
        out.sort_by_key(|c| (!c.group, c.position));
        out
    }

    pub fn selected_service(&self) -> Option<&Component> {
        self.services().get(self.service_idx).copied()
    }

    pub fn timeline(&self) -> Vec<(String, Vec<&Incident>)> {
        let Some(s) = self.summary.as_ref() else {
            return vec![];
        };
        let matches_service = self.incident_filter();
        let mut by_day: HashMap<NaiveDate, Vec<&Incident>> = HashMap::new();
        for inc in &s.incidents {
            if !matches_service(inc) {
                continue;
            }
            by_day
                .entry(inc.started_at.date_naive())
                .or_default()
                .push(inc);
        }

        let today = bars::today_utc().date_naive();
        (0..bars::WINDOW_DAYS)
            .map(|offset| {
                let d = today - chrono::Duration::days(offset);
                let mut incidents = by_day.remove(&d).unwrap_or_default();
                incidents.sort_by_key(|i| std::cmp::Reverse(i.started_at));
                (day_label(d), incidents)
            })
            .collect()
    }

    pub fn flat_incidents(&self) -> Vec<&Incident> {
        self.timeline().into_iter().flat_map(|(_, v)| v).collect()
    }

    fn incident_filter(&self) -> Box<dyn Fn(&Incident) -> bool + '_> {
        let Some(service) = self.selected_service() else {
            return Box::new(|_| true);
        };
        let Some(summary) = self.summary.as_ref() else {
            return Box::new(|_| true);
        };
        let service_id = service.id.clone();
        let leaf_ids: std::collections::HashSet<String> = summary
            .components
            .iter()
            .filter(|c| c.group_id.as_deref() == Some(service_id.as_str()))
            .map(|c| c.id.clone())
            .collect();
        Box::new(move |inc| {
            inc.components
                .iter()
                .any(|cr| cr.id == service_id || leaf_ids.contains(&cr.id))
        })
    }

    pub fn handle(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::Tick => {
                if let Some(until) = self.toast_until {
                    if Instant::now() >= until {
                        self.status_toast = None;
                        self.toast_until = None;
                    }
                }
            }
            AppEvent::Resize => {}
            AppEvent::Loaded(s) => {
                self.last_loaded_at = Some(Utc::now());
                self.last_attempt_at = Some(Instant::now());
                self.error = None;
                self.bars = bars::compute(&s.components, &s.incidents, bars::today_utc());
                self.summary = Some(*s);
            }
            AppEvent::LoadFailed(e) => {
                self.last_attempt_at = Some(Instant::now());
                self.error = Some(e.clone());
                self.toast(format!("refresh failed: {e}"));
            }
            AppEvent::Key(k) => self.handle_key(k),
        }
    }

    fn toast(&mut self, msg: String) {
        self.status_toast = Some(msg);
        self.toast_until = Some(Instant::now() + Duration::from_secs(4));
    }

    fn handle_key(&mut self, k: KeyEvent) {
        if self.help_open {
            if matches!(
                k.code,
                KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')
            ) {
                self.help_open = false;
            }
            return;
        }
        if self.theme_picker_open {
            match k.code {
                KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('q') => {
                    self.theme_picker_open = false
                }
                KeyCode::Up | KeyCode::Char('k') => self.theme.cycle(-1),
                KeyCode::Down | KeyCode::Char('j') => self.theme.cycle(1),
                KeyCode::Enter => self.theme_picker_open = false,
                _ => {}
            }
            return;
        }
        let plain = is_plain(k.modifiers);
        let ctrl = k.modifiers.contains(KeyModifiers::CONTROL);
        match k.code {
            KeyCode::Char('c' | 'd') if ctrl => self.quit = true,
            KeyCode::Char('q') if plain => self.quit = true,
            KeyCode::Esc => {
                if self.overlay.is_open() {
                    self.overlay.close();
                } else {
                    self.quit = true;
                }
            }
            KeyCode::Tab => self.cycle_focus(1),
            KeyCode::BackTab => self.cycle_focus(-1),
            KeyCode::Char('?') => self.help_open = true,
            KeyCode::Char('t') if plain => self.theme_picker_open = true,
            KeyCode::Char('r') if plain => self.toast("refreshing...".into()),
            KeyCode::Up => self.move_selection(-1),
            KeyCode::Char('k') if plain => self.move_selection(-1),
            KeyCode::Down => self.move_selection(1),
            KeyCode::Char('j') if plain => self.move_selection(1),
            KeyCode::Left => self.move_horizontal(-1),
            KeyCode::Char('h') if plain => self.move_horizontal(-1),
            KeyCode::Right => self.move_horizontal(1),
            KeyCode::Char('l') if plain => self.move_horizontal(1),
            KeyCode::Enter => self.open_modal_for_selection(),
            _ => {}
        }
    }

    fn cycle_focus(&mut self, _dir: i8) {
        self.focus = match self.focus {
            Pane::Services => Pane::Events,
            Pane::Events => Pane::Services,
        };
    }

    fn move_selection(&mut self, dir: i8) {
        match self.focus {
            Pane::Services => {
                let n = self.services().len();
                if n == 0 {
                    return;
                }
                self.service_idx = wrap(self.service_idx, dir, n);
                self.event_idx = 0;
            }
            Pane::Events => {
                let n = self.flat_incidents().len();
                if n == 0 {
                    return;
                }
                self.event_idx = wrap(self.event_idx, dir, n);
            }
        }
    }

    fn move_horizontal(&mut self, dir: i8) {
        if matches!(self.focus, Pane::Services) {
            let max = bars::WINDOW_DAYS as usize - 1;
            let next = self.bars_day as isize + dir as isize;
            self.bars_day = next.clamp(0, max as isize) as usize;
        }
    }

    pub fn open_modal_for_selection(&mut self) {
        match self.focus {
            Pane::Services => {
                self.modal_target = ModalTarget::Day {
                    row: self.service_idx,
                    day: self.bars_day,
                };
                self.overlay.open();
            }
            Pane::Events => {
                if let Some(i) = self.flat_incidents().get(self.event_idx) {
                    self.modal_target = ModalTarget::Incident(i.id.clone());
                    self.overlay.open();
                }
            }
        }
    }
}

fn is_plain(m: KeyModifiers) -> bool {
    m == KeyModifiers::NONE || m == KeyModifiers::SHIFT
}

fn wrap(idx: usize, dir: i8, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let n = len as isize;
    let next = (idx as isize + dir as isize) % n;
    if next < 0 {
        (next + n) as usize
    } else {
        next as usize
    }
}

fn day_label(d: NaiveDate) -> String {
    let today = Utc::now().date_naive();
    let delta = (today - d).num_days();
    match delta {
        0 => format!("Today · {}", d.format("%b %-d")),
        1 => format!("Yesterday · {}", d.format("%b %-d")),
        _ => d.format("%a %b %-d, %Y").to_string(),
    }
}
