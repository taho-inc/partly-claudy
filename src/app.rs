use std::collections::BTreeMap;
use std::time::{Duration, Instant};

use chrono::{DateTime, Datelike, Utc};
use crossterm::event::{KeyCode, KeyEvent, KeyModifiers};
use tui_overlay::OverlayState;

use crate::api::{Component, Incident, Summary};
use crate::bars::{self, UptimeRow};
use crate::events::AppEvent;
use crate::theme::AppTheme;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Pane {
    Bars,
    Sections,
    Events,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum DrawerTarget {
    Empty,
    Component(String),
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
    pub bars_row: usize,
    pub bars_day: usize,
    pub section_idx: usize,
    pub event_idx: usize,
    pub drawer_target: DrawerTarget,
    pub quit: bool,
    pub status_toast: Option<String>,
    pub toast_until: Option<Instant>,
}

impl App {
    pub fn new(theme: AppTheme) -> Self {
        let mut overlay = OverlayState::new().with_duration(Duration::from_millis(140));
        overlay.open();
        Self {
            started_at: Instant::now(),
            theme,
            overlay,
            focus: Pane::Sections,
            summary: None,
            bars: Vec::new(),
            error: None,
            last_loaded_at: None,
            last_attempt_at: None,
            help_open: false,
            theme_picker_open: false,
            bars_row: 0,
            bars_day: bars::WINDOW_DAYS as usize - 1,
            section_idx: 0,
            event_idx: 0,
            drawer_target: DrawerTarget::Empty,
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

    pub fn section_rows(&self) -> Vec<&Component> {
        let Some(s) = self.summary.as_ref() else { return vec![] };
        let mut rows: Vec<&Component> = s.components.iter().filter(|c| c.group).collect();
        rows.sort_by_key(|c| c.position);
        let mut out: Vec<&Component> = Vec::new();
        for g in rows {
            out.push(g);
        }
        for c in &s.components {
            if !c.group && c.group_id.is_none() {
                out.push(c);
            }
        }
        out
    }

    pub fn children_of(&self, group_id: &str) -> Vec<&Component> {
        let Some(s) = self.summary.as_ref() else { return vec![] };
        let mut kids: Vec<&Component> = s
            .components
            .iter()
            .filter(|c| c.group_id.as_deref() == Some(group_id))
            .collect();
        kids.sort_by_key(|c| c.position);
        kids
    }

    pub fn timeline(&self) -> Vec<(String, Vec<&Incident>)> {
        let Some(s) = self.summary.as_ref() else { return vec![] };
        let mut all: Vec<&Incident> = s.incidents.iter().chain(s.past_incidents.iter()).collect();
        all.sort_by_key(|i| std::cmp::Reverse(i.started_at));

        let mut by_day: BTreeMap<i64, Vec<&Incident>> = BTreeMap::new();
        for inc in all {
            let day = inc.started_at.date_naive();
            let key = day.num_days_from_ce() as i64;
            by_day.entry(-key).or_default().push(inc);
        }
        by_day
            .into_values()
            .map(|mut v| {
                v.sort_by_key(|i| std::cmp::Reverse(i.started_at));
                let label = day_label(v[0].started_at);
                (label, v)
            })
            .collect()
    }

    pub fn flat_incidents(&self) -> Vec<&Incident> {
        self.timeline().into_iter().flat_map(|(_, v)| v).collect()
    }

    pub fn handle(&mut self, ev: AppEvent) {
        match ev {
            AppEvent::Tick => {
                self.overlay.tick(Duration::from_millis(50));
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
                self.bars = bars::compute(&s.components, &all_incidents(&s), bars::today_utc());
                self.summary = Some(*s);
                if matches!(self.drawer_target, DrawerTarget::Empty) {
                    self.refresh_drawer_default();
                }
            }
            AppEvent::LoadFailed(e) => {
                self.last_attempt_at = Some(Instant::now());
                self.error = Some(e.clone());
                self.toast(format!("refresh failed: {e}"));
            }
            AppEvent::Key(k) => self.handle_key(k),
        }
    }

    fn refresh_drawer_default(&mut self) {
        if let Some(c) = self.section_rows().first() {
            self.drawer_target = DrawerTarget::Component(c.id.clone());
        }
    }

    fn toast(&mut self, msg: String) {
        self.status_toast = Some(msg);
        self.toast_until = Some(Instant::now() + Duration::from_secs(4));
    }

    fn handle_key(&mut self, k: KeyEvent) {
        if self.help_open {
            if matches!(k.code, KeyCode::Esc | KeyCode::Char('?') | KeyCode::Char('q')) {
                self.help_open = false;
            }
            return;
        }
        if self.theme_picker_open {
            match k.code {
                KeyCode::Esc | KeyCode::Char('t') | KeyCode::Char('q') => self.theme_picker_open = false,
                KeyCode::Up | KeyCode::Char('k') => self.theme.cycle(-1),
                KeyCode::Down | KeyCode::Char('j') => self.theme.cycle(1),
                KeyCode::Enter => self.theme_picker_open = false,
                _ => {}
            }
            return;
        }
        match (k.code, k.modifiers) {
            (KeyCode::Char('c'), KeyModifiers::CONTROL) => self.quit = true,
            (KeyCode::Char('q'), _) => self.quit = true,
            (KeyCode::Esc, _) => {
                if self.overlay.is_open() {
                    self.overlay.close();
                } else {
                    self.quit = true;
                }
            }
            (KeyCode::Tab, _) => self.cycle_focus(1),
            (KeyCode::BackTab, _) => self.cycle_focus(-1),
            (KeyCode::Char('?'), _) => self.help_open = true,
            (KeyCode::Char('t'), _) => self.theme_picker_open = true,
            (KeyCode::Char('d'), _) => self.overlay.toggle(),
            (KeyCode::Char('r'), _) => self.toast("refreshing...".into()),
            (KeyCode::Up | KeyCode::Char('k'), _) => self.move_selection(-1),
            (KeyCode::Down | KeyCode::Char('j'), _) => self.move_selection(1),
            (KeyCode::Left | KeyCode::Char('h'), _) => self.move_horizontal(-1),
            (KeyCode::Right | KeyCode::Char('l'), _) => self.move_horizontal(1),
            (KeyCode::Enter, _) => {
                self.update_drawer_from_selection();
                if !self.overlay.is_open() {
                    self.overlay.open();
                }
            }
            _ => {}
        }
    }

    fn cycle_focus(&mut self, dir: i8) {
        let order = [Pane::Bars, Pane::Sections, Pane::Events];
        let idx = order.iter().position(|p| *p == self.focus).unwrap_or(1);
        let len = order.len() as isize;
        let next = ((idx as isize + dir as isize) % len + len) % len;
        self.focus = order[next as usize];
    }

    fn move_selection(&mut self, dir: i8) {
        match self.focus {
            Pane::Bars => {
                let n = self.bars.len();
                if n == 0 {
                    return;
                }
                self.bars_row = wrap(self.bars_row, dir, n);
                self.update_drawer_from_selection();
            }
            Pane::Sections => {
                let n = self.section_rows().len();
                if n == 0 {
                    return;
                }
                self.section_idx = wrap(self.section_idx, dir, n);
                self.update_drawer_from_selection();
            }
            Pane::Events => {
                let n = self.flat_incidents().len();
                if n == 0 {
                    return;
                }
                self.event_idx = wrap(self.event_idx, dir, n);
                self.update_drawer_from_selection();
            }
        }
    }

    fn move_horizontal(&mut self, dir: i8) {
        if matches!(self.focus, Pane::Bars) {
            let max = bars::WINDOW_DAYS as usize - 1;
            let next = self.bars_day as isize + dir as isize;
            self.bars_day = next.clamp(0, max as isize) as usize;
            self.update_drawer_from_selection();
        }
    }

    pub fn update_drawer_from_selection(&mut self) {
        match self.focus {
            Pane::Bars => {
                self.drawer_target = DrawerTarget::Day { row: self.bars_row, day: self.bars_day };
            }
            Pane::Sections => {
                if let Some(c) = self.section_rows().get(self.section_idx) {
                    self.drawer_target = DrawerTarget::Component(c.id.clone());
                }
            }
            Pane::Events => {
                if let Some(i) = self.flat_incidents().get(self.event_idx) {
                    self.drawer_target = DrawerTarget::Incident(i.id.clone());
                }
            }
        }
    }
}

fn wrap(idx: usize, dir: i8, len: usize) -> usize {
    if len == 0 {
        return 0;
    }
    let n = len as isize;
    let next = (idx as isize + dir as isize) % n;
    if next < 0 { (next + n) as usize } else { next as usize }
}

fn all_incidents(s: &Summary) -> Vec<Incident> {
    let mut v = Vec::with_capacity(s.incidents.len() + s.past_incidents.len() + s.scheduled_maintenances.len());
    v.extend(s.incidents.iter().cloned());
    v.extend(s.past_incidents.iter().cloned());
    v.extend(s.scheduled_maintenances.iter().cloned());
    v
}

fn day_label(ts: DateTime<Utc>) -> String {
    let today = Utc::now().date_naive();
    let d = ts.date_naive();
    let delta = (today - d).num_days();
    match delta {
        0 => format!("Today · {}", d.format("%b %-d")),
        1 => format!("Yesterday · {}", d.format("%b %-d")),
        _ => d.format("%a %b %-d, %Y").to_string(),
    }
}
