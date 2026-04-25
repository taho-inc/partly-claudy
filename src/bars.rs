use std::collections::HashMap;

use chrono::{DateTime, Datelike, Duration, NaiveDate, TimeZone, Utc};

use crate::api::{Component, Impact, Incident};

pub const WINDOW_DAYS: i64 = 90;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum DayStatus {
    Unknown,
    Operational,
    Maintenance,
    Degraded,
    Outage,
}

#[derive(Debug, Clone)]
pub struct UptimeRow {
    pub group_id: String,
    pub group_name: String,
    pub days: Vec<DayStatus>,
    pub uptime_pct: f32,
}

pub fn compute(components: &[Component], incidents: &[Incident], today: DateTime<Utc>) -> Vec<UptimeRow> {
    let today_date = today.date_naive();
    let start_date = today_date - Duration::days(WINDOW_DAYS - 1);

    let mut child_to_group: HashMap<&str, &str> = HashMap::new();
    for c in components {
        if let Some(gid) = &c.group_id {
            child_to_group.insert(c.id.as_str(), gid.as_str());
        }
    }

    let mut rows: Vec<UptimeRow> = components
        .iter()
        .filter(|c| c.group)
        .map(|c| {
            let mut days = vec![DayStatus::Operational; WINDOW_DAYS as usize];
            let created = c.created_at.date_naive();
            for (i, day) in days.iter_mut().enumerate() {
                let d = start_date + Duration::days(i as i64);
                if d < created {
                    *day = DayStatus::Unknown;
                }
            }
            UptimeRow {
                group_id: c.id.clone(),
                group_name: c.name.clone(),
                days,
                uptime_pct: 100.0,
            }
        })
        .collect();

    let row_idx: HashMap<String, usize> = rows
        .iter()
        .enumerate()
        .map(|(i, r)| (r.group_id.clone(), i))
        .collect();

    for inc in incidents {
        let severity = severity_for(inc.impact);
        if matches!(severity, DayStatus::Operational | DayStatus::Unknown) {
            continue;
        }

        let started = inc.started_at;
        let ended = inc.resolved_at.unwrap_or(today);
        let mut group_targets: Vec<String> = inc
            .components
            .iter()
            .map(|c| {
                child_to_group
                    .get(c.id.as_str())
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| c.id.clone())
            })
            .collect();
        group_targets.sort();
        group_targets.dedup();

        for gid in group_targets {
            let Some(&i) = row_idx.get(&gid) else { continue };
            let row = &mut rows[i];
            paint(row, started, ended, severity, start_date);
        }
    }

    for row in &mut rows {
        let downtime_minutes: i64 = row
            .days
            .iter()
            .map(|d| match d {
                DayStatus::Outage => 24 * 60,
                DayStatus::Degraded => 12 * 60,
                DayStatus::Maintenance => 0,
                _ => 0,
            })
            .sum();
        let total_minutes = (WINDOW_DAYS * 24 * 60) as f32;
        row.uptime_pct = (1.0 - downtime_minutes as f32 / total_minutes) * 100.0;
    }

    rows
}

fn paint(row: &mut UptimeRow, started: DateTime<Utc>, ended: DateTime<Utc>, sev: DayStatus, start_date: NaiveDate) {
    let from = started.date_naive().max(start_date);
    let to = ended.date_naive();
    let mut d = from;
    while d <= to {
        let offset = (d - start_date).num_days();
        if (0..WINDOW_DAYS).contains(&offset) {
            let i = offset as usize;
            if row.days[i] != DayStatus::Unknown && sev > row.days[i] {
                row.days[i] = sev;
            }
        }
        d = d.succ_opt().unwrap_or(d);
        if d == to.succ_opt().unwrap_or(d) && d > to {
            break;
        }
    }
}

fn severity_for(impact: Impact) -> DayStatus {
    match impact {
        Impact::None => DayStatus::Operational,
        Impact::Minor => DayStatus::Degraded,
        Impact::Major | Impact::Critical => DayStatus::Outage,
        Impact::Maintenance => DayStatus::Maintenance,
    }
}

pub fn date_for_day(today: DateTime<Utc>, day_index: usize) -> NaiveDate {
    let start = today.date_naive() - Duration::days(WINDOW_DAYS - 1);
    start + Duration::days(day_index as i64)
}

pub fn today_utc() -> DateTime<Utc> {
    let now = Utc::now();
    Utc.with_ymd_and_hms(now.year(), now.month(), now.day(), 23, 59, 59)
        .single()
        .unwrap_or(now)
}
