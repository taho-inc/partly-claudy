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

pub fn compute(
    components: &[Component],
    incidents: &[Incident],
    today: DateTime<Utc>,
) -> Vec<UptimeRow> {
    let today_date = today.date_naive();
    let start_date = today_date - Duration::days(WINDOW_DAYS - 1);

    let mut child_to_group: HashMap<&str, &str> = HashMap::new();
    for c in components {
        if let Some(gid) = &c.group_id {
            child_to_group.insert(c.id.as_str(), gid.as_str());
        }
    }

    let mut services: Vec<&Component> = components
        .iter()
        .filter(|c| c.group || c.group_id.is_none())
        .collect();
    services.sort_by_key(|c| (!c.group, c.position));

    let mut rows: Vec<UptimeRow> = services
        .into_iter()
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
            let Some(&i) = row_idx.get(&gid) else {
                continue;
            };
            let row = &mut rows[i];
            paint(row, started, ended, severity, start_date);
        }
    }

    let window_start = start_date
        .and_hms_opt(0, 0, 0)
        .map(|n| n.and_utc())
        .unwrap_or(today - Duration::days(WINDOW_DAYS - 1));
    let mut downtime_minutes = vec![0f64; rows.len()];
    for inc in incidents {
        if matches!(inc.impact, Impact::None | Impact::Maintenance) {
            continue;
        }
        accumulate_component_downtime(
            inc,
            window_start,
            today,
            &child_to_group,
            &row_idx,
            &mut downtime_minutes,
        );
    }

    let total_minutes = (WINDOW_DAYS * 24 * 60) as f32;
    for (i, row) in rows.iter_mut().enumerate() {
        let down = downtime_minutes[i] as f32;
        row.uptime_pct = ((1.0 - down / total_minutes) * 100.0).clamp(0.0, 100.0);
    }

    rows
}

/// Walk an incident's update timeline and credit weighted downtime to
/// each affected service. Each `affected_components[].new_status`
/// transition begins a span; spans end at the next same-component
/// transition, or — and this is the key bit — at the incident's
/// **mitigation** timestamp (first update with `monitoring` or
/// `resolved` status). Statuspage stops crediting downtime once an
/// incident moves past identification, even if the component status
/// payload still reads partial_outage during the verification phase.
///
/// Span weight comes from the component status:
///   major_outage         → 1.0
///   partial_outage       → 0.5
///   degraded_performance → 0.0  (Statuspage doesn't credit degraded
///                                hours toward published uptime)
///   under_maintenance    → 0.0
///   operational          → 0.0
fn accumulate_component_downtime(
    inc: &Incident,
    window_start: DateTime<Utc>,
    today: DateTime<Utc>,
    child_to_group: &HashMap<&str, &str>,
    row_idx: &HashMap<String, usize>,
    downtime_minutes: &mut [f64],
) {
    let inc_end = inc.resolved_at.unwrap_or(today).min(today);
    let mitigation_end = mitigation_timestamp(inc).unwrap_or(inc_end).min(inc_end);

    let mut updates = inc.incident_updates.clone();
    updates.sort_by_key(|u| u.display_at.unwrap_or(u.created_at));
    let mut timelines: HashMap<String, Vec<(DateTime<Utc>, &str)>> = HashMap::new();
    for u in &updates {
        let ts = u.display_at.unwrap_or(u.created_at);
        let Some(affected) = u.affected_components.as_ref() else {
            continue;
        };
        for ac in affected {
            let entry = timelines.entry(ac.code.clone()).or_default();
            if entry.last().map(|(_, s)| *s) != Some(ac.new_status.as_str()) {
                entry.push((ts, ac.new_status.as_str()));
            }
        }
    }

    for (cid, spans) in timelines {
        let sid = if row_idx.contains_key(&cid) {
            cid.clone()
        } else if let Some(parent) = child_to_group.get(cid.as_str()) {
            parent.to_string()
        } else {
            continue;
        };
        let Some(&row) = row_idx.get(&sid) else {
            continue;
        };

        for (i, (start, status)) in spans.iter().enumerate() {
            let weight = status_weight(status);
            if weight == 0.0 {
                continue;
            }
            let span_end = spans.get(i + 1).map(|(t, _)| *t).unwrap_or(inc_end);
            // Cap span end at the mitigation timestamp — once the incident
            // moves to monitoring/resolved, downtime stops accruing
            // regardless of what the component status field says.
            let effective_end = span_end.min(mitigation_end);
            let s = (*start).max(window_start);
            let e = effective_end.min(today);
            if e <= s {
                continue;
            }
            let mins = (e - s).num_minutes().max(0) as f64;
            downtime_minutes[row] += mins * weight;
        }
    }
}

/// Earliest update timestamp where the incident reached `monitoring`,
/// `resolved`, or `postmortem` — i.e. the moment Statuspage considers
/// the impact mitigated. `None` if the incident never advanced that far.
fn mitigation_timestamp(inc: &Incident) -> Option<DateTime<Utc>> {
    inc.incident_updates
        .iter()
        .filter(|u| {
            matches!(
                u.status,
                crate::api::IncidentStatus::Monitoring
                    | crate::api::IncidentStatus::Resolved
                    | crate::api::IncidentStatus::Postmortem
            )
        })
        .map(|u| u.display_at.unwrap_or(u.created_at))
        .min()
}

fn status_weight(status: &str) -> f64 {
    match status {
        "major_outage" => 1.0,
        "partial_outage" => 0.5,
        _ => 0.0,
    }
}

fn paint(
    row: &mut UptimeRow,
    started: DateTime<Utc>,
    ended: DateTime<Utc>,
    sev: DayStatus,
    start_date: NaiveDate,
) {
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

#[cfg(test)]
mod tests {
    use super::*;
    use crate::api::ComponentStatus;

    fn component(id: &str, name: &str, group: bool, group_id: Option<&str>, pos: u32) -> Component {
        let now = Utc::now();
        Component {
            id: id.into(),
            name: name.into(),
            status: ComponentStatus::Operational,
            created_at: now - Duration::days(365),
            updated_at: now,
            position: pos,
            description: None,
            group_id: group_id.map(|s| s.into()),
            group,
        }
    }

    #[test]
    fn standalone_components_become_rows() {
        let components = vec![
            component("a", "claude.ai", false, None, 1),
            component("b", "Claude API", false, None, 2),
            component("c", "Claude Code", false, None, 3),
        ];
        let rows = compute(&components, &[], today_utc());
        assert_eq!(rows.len(), 3, "every standalone is a row");
        assert_eq!(
            rows.iter()
                .map(|r| r.group_name.as_str())
                .collect::<Vec<_>>(),
            vec!["claude.ai", "Claude API", "Claude Code"],
        );
    }

    #[test]
    fn days_before_component_creation_render_as_unknown() {
        let now = today_utc();
        let mut c = component("new", "Claude Cowork", false, None, 1);
        c.created_at = now - Duration::days(5);
        let rows = compute(&[c], &[], now);
        assert_eq!(rows.len(), 1);
        let unknowns = rows[0]
            .days
            .iter()
            .filter(|d| matches!(d, DayStatus::Unknown))
            .count();
        assert_eq!(
            unknowns,
            (WINDOW_DAYS - 6) as usize,
            "every day before created_at is Unknown (created_at day itself is data)",
        );
    }

    #[test]
    fn major_outage_status_credits_full_duration() {
        use crate::api::{IncidentComponentRef, IncidentStatus};
        let now = today_utc();
        let comp = component("svc", "svc", false, None, 1);
        let outage_start = now - Duration::hours(2);
        let recovery = now - Duration::hours(1); // 1h spent in major_outage
        let inc = Incident {
            id: "i".into(),
            name: "n".into(),
            status: IncidentStatus::Resolved,
            impact: Impact::Major,
            created_at: outage_start,
            updated_at: recovery,
            monitoring_at: None,
            resolved_at: Some(recovery),
            started_at: outage_start,
            shortlink: None,
            incident_updates: vec![
                update(outage_start, "svc", "operational", "major_outage"),
                update(recovery, "svc", "major_outage", "operational"),
            ],
            components: vec![IncidentComponentRef {
                id: "svc".into(),
                name: "svc".into(),
            }],
        };
        let rows = compute(&[comp], &[inc], now);
        // 60 min × weight 1.0 ÷ (90 × 24 × 60) ≈ 99.954% uptime
        let pct = rows[0].uptime_pct;
        assert!((99.94..100.0).contains(&pct), "expected ~99.95%, got {pct}");
    }

    #[test]
    fn partial_outage_status_credits_half_duration() {
        use crate::api::{IncidentComponentRef, IncidentStatus};
        let now = today_utc();
        let comp = component("svc", "svc", false, None, 1);
        let outage_start = now - Duration::hours(2);
        let recovery = now - Duration::hours(0); // 2h spent in partial_outage
        let inc = Incident {
            id: "i".into(),
            name: "n".into(),
            status: IncidentStatus::Resolved,
            impact: Impact::Major,
            created_at: outage_start,
            updated_at: recovery,
            monitoring_at: None,
            resolved_at: Some(recovery),
            started_at: outage_start,
            shortlink: None,
            incident_updates: vec![
                update(outage_start, "svc", "operational", "partial_outage"),
                update(recovery, "svc", "partial_outage", "operational"),
            ],
            components: vec![IncidentComponentRef {
                id: "svc".into(),
                name: "svc".into(),
            }],
        };
        let rows = compute(&[comp], &[inc], now);
        // 120 min × weight 0.5 = 60 effective ÷ (90 × 24 × 60) ≈ 99.954%
        let pct = rows[0].uptime_pct;
        assert!((99.94..100.0).contains(&pct), "expected ~99.95%, got {pct}");
    }

    #[test]
    fn downtime_stops_at_mitigation_status() {
        use crate::api::{IncidentComponentRef, IncidentStatus, IncidentUpdate};
        let now = today_utc();
        let comp = component("svc", "svc", false, None, 1);
        let outage_start = now - Duration::hours(2);
        let monitoring_ts = now - Duration::hours(1); // we declare mitigation here
        let resolved_ts = now - Duration::hours(0); // close incident an hour later
                                                    // Component never gets an explicit "operational" transition; the
                                                    // last visible status remains partial_outage. The new logic must
                                                    // stop accruing downtime at monitoring_ts, not span all the way
                                                    // to resolved_ts.
        let inc = Incident {
            id: "i".into(),
            name: "n".into(),
            status: IncidentStatus::Resolved,
            impact: Impact::Major,
            created_at: outage_start,
            updated_at: resolved_ts,
            monitoring_at: Some(monitoring_ts),
            resolved_at: Some(resolved_ts),
            started_at: outage_start,
            shortlink: None,
            incident_updates: vec![
                IncidentUpdate {
                    id: "u1".into(),
                    status: IncidentStatus::Investigating,
                    body: String::new(),
                    created_at: outage_start,
                    display_at: Some(outage_start),
                    affected_components: Some(vec![crate::api::AffectedComponent {
                        code: "svc".into(),
                        name: "svc".into(),
                        old_status: "operational".into(),
                        new_status: "partial_outage".into(),
                    }]),
                },
                IncidentUpdate {
                    id: "u2".into(),
                    status: IncidentStatus::Monitoring,
                    body: String::new(),
                    created_at: monitoring_ts,
                    display_at: Some(monitoring_ts),
                    affected_components: None,
                },
                IncidentUpdate {
                    id: "u3".into(),
                    status: IncidentStatus::Resolved,
                    body: String::new(),
                    created_at: resolved_ts,
                    display_at: Some(resolved_ts),
                    affected_components: None,
                },
            ],
            components: vec![IncidentComponentRef {
                id: "svc".into(),
                name: "svc".into(),
            }],
        };
        let rows = compute(&[comp], &[inc], now);
        // 60 minutes of partial_outage × 0.5 weight = 30 effective minutes
        // (outage_start..monitoring_ts), NOT the full 120 min to resolved_ts.
        // 30 / (90 × 24 × 60) ≈ 99.977% uptime
        let pct = rows[0].uptime_pct;
        assert!(
            (99.97..100.0).contains(&pct),
            "expected ~99.977%, got {pct}"
        );
    }

    #[test]
    fn degraded_status_does_not_credit_downtime() {
        use crate::api::{IncidentComponentRef, IncidentStatus};
        let now = today_utc();
        let comp = component("svc", "svc", false, None, 1);
        let inc = Incident {
            id: "i".into(),
            name: "n".into(),
            status: IncidentStatus::Resolved,
            impact: Impact::Minor,
            created_at: now - Duration::hours(5),
            updated_at: now - Duration::hours(4),
            monitoring_at: None,
            resolved_at: Some(now - Duration::hours(4)),
            started_at: now - Duration::hours(5),
            shortlink: None,
            incident_updates: vec![
                update(
                    now - Duration::hours(5),
                    "svc",
                    "operational",
                    "degraded_performance",
                ),
                update(
                    now - Duration::hours(4),
                    "svc",
                    "degraded_performance",
                    "operational",
                ),
            ],
            components: vec![IncidentComponentRef {
                id: "svc".into(),
                name: "svc".into(),
            }],
        };
        let rows = compute(&[comp], &[inc], now);
        assert_eq!(rows[0].uptime_pct, 100.0);
    }

    fn update(ts: DateTime<Utc>, code: &str, old: &str, new: &str) -> crate::api::IncidentUpdate {
        crate::api::IncidentUpdate {
            id: format!("u-{}", ts.timestamp()),
            status: crate::api::IncidentStatus::Identified,
            body: String::new(),
            created_at: ts,
            display_at: Some(ts),
            affected_components: Some(vec![crate::api::AffectedComponent {
                code: code.into(),
                name: code.into(),
                old_status: old.into(),
                new_status: new.into(),
            }]),
        }
    }

    #[test]
    fn groups_precede_standalones_then_sort_by_position() {
        let components = vec![
            component("standalone", "anthropic.com", false, None, 3),
            component("g1", "Anthropic API", true, None, 2),
            component("leaf", "api Messages", false, Some("g1"), 1),
            component("g0", "Claude.ai", true, None, 1),
        ];
        let rows = compute(&components, &[], today_utc());
        assert_eq!(
            rows.iter()
                .map(|r| r.group_name.as_str())
                .collect::<Vec<_>>(),
            vec!["Claude.ai", "Anthropic API", "anthropic.com"],
            "groups (pos 1, 2) before standalone (pos 3); leaf is excluded",
        );
    }
}
