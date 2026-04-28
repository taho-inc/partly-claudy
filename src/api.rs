#![allow(dead_code)]

use std::collections::HashSet;
use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result};
use futures::stream::{self, StreamExt};
use serde::Deserialize;

const DEFAULT_BASE: &str = "https://status.claude.com";
const USER_AGENT: &str = concat!("partly-claudy/", env!("CARGO_PKG_VERSION"));

#[derive(Debug, Clone, Deserialize)]
pub struct Page {
    pub id: String,
    pub name: String,
    pub url: String,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Status {
    pub indicator: Indicator,
    pub description: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Indicator {
    None,
    Minor,
    Major,
    Critical,
    #[serde(other)]
    Maintenance,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Component {
    pub id: String,
    pub name: String,
    pub status: ComponentStatus,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub position: u32,
    #[serde(default)]
    pub description: Option<String>,
    #[serde(default)]
    pub group_id: Option<String>,
    #[serde(default)]
    pub group: bool,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ComponentStatus {
    Operational,
    DegradedPerformance,
    PartialOutage,
    MajorOutage,
    UnderMaintenance,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Incident {
    pub id: String,
    pub name: String,
    pub status: IncidentStatus,
    pub impact: Impact,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(default)]
    pub monitoring_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub resolved_at: Option<DateTime<Utc>>,
    pub started_at: DateTime<Utc>,
    #[serde(default)]
    pub shortlink: Option<String>,
    #[serde(default)]
    pub incident_updates: Vec<IncidentUpdate>,
    #[serde(default)]
    pub components: Vec<IncidentComponentRef>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum IncidentStatus {
    Investigating,
    Identified,
    Monitoring,
    Resolved,
    Postmortem,
    Scheduled,
    InProgress,
    Verifying,
    Completed,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Impact {
    None,
    Minor,
    Major,
    Critical,
    Maintenance,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncidentUpdate {
    pub id: String,
    pub status: IncidentStatus,
    pub body: String,
    pub created_at: DateTime<Utc>,
    #[serde(default)]
    pub display_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub affected_components: Option<Vec<AffectedComponent>>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AffectedComponent {
    pub code: String,
    pub name: String,
    pub old_status: String,
    pub new_status: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct IncidentComponentRef {
    pub id: String,
    pub name: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct Summary {
    pub page: Page,
    pub status: Status,
    #[serde(default)]
    pub components: Vec<Component>,
    #[serde(default)]
    pub incidents: Vec<Incident>,
    #[serde(default)]
    pub scheduled_maintenances: Vec<Incident>,
}

#[derive(Deserialize)]
struct IncidentList {
    #[serde(default)]
    incidents: Vec<Incident>,
}

#[derive(Deserialize)]
struct IncidentEnvelope {
    incident: Incident,
}

/// Schema returned by the (undocumented) `/history.json[?page=N]` endpoint.
/// Used to enumerate incident codes outside the most-recent v2 window so
/// we can fetch their full typed detail.
#[derive(Deserialize)]
struct HistoryResponse {
    #[serde(default)]
    months: Vec<HistoryMonth>,
}

#[derive(Deserialize)]
struct HistoryMonth {
    name: String,
    year: i32,
    #[serde(default)]
    incidents: Vec<HistoryIncident>,
}

#[derive(Deserialize)]
struct HistoryIncident {
    code: String,
}

const MONTH_NAMES: [&str; 12] = [
    "January", "February", "March", "April", "May", "June", "July", "August", "September",
    "October", "November", "December",
];

/// Returns true when this month's last day is on or after `cutoff` —
/// i.e. the month overlaps our window of interest.
fn month_overlaps(month: &HistoryMonth, cutoff: chrono::NaiveDate) -> bool {
    let Some(idx) = MONTH_NAMES.iter().position(|n| *n == month.name) else {
        return true; // unknown name — be permissive
    };
    let m = idx as u32 + 1;
    let last = if m == 12 {
        chrono::NaiveDate::from_ymd_opt(month.year, 12, 31)
    } else {
        chrono::NaiveDate::from_ymd_opt(month.year, m + 1, 1)
            .map(|d| d - chrono::Duration::days(1))
    };
    last.is_some_and(|d| d >= cutoff)
}

/// Concurrency cap on per-incident detail fetches. Keeps the public API
/// happy and bounds first-load latency.
const DETAIL_FETCH_CONCURRENCY: usize = 8;
/// History pages to scan when filling the older end of the window.
/// Each page is roughly three calendar months.
const HISTORY_PAGES: usize = 2;

#[derive(Clone)]
pub enum Source {
    Live { base: String, http: reqwest::Client },
    Fixture(std::path::PathBuf),
}

impl Source {
    pub fn live(base: Option<String>) -> Result<Self> {
        let http = reqwest::Client::builder()
            .user_agent(USER_AGENT)
            .timeout(Duration::from_secs(10))
            .build()
            .wrap_err("building HTTP client")?;
        Ok(Self::Live {
            base: base.unwrap_or_else(|| DEFAULT_BASE.to_string()),
            http,
        })
    }

    pub fn fixture(path: impl AsRef<Path>) -> Self {
        Self::Fixture(path.as_ref().to_path_buf())
    }

    pub async fn fetch_summary(&self) -> Result<Summary> {
        match self {
            Self::Live { base, http } => fetch_live(base, http).await,
            Self::Fixture(path) => {
                let bytes = tokio::fs::read(path)
                    .await
                    .wrap_err_with(|| format!("read fixture {}", path.display()))?;
                let summary: Summary = serde_json::from_slice(&bytes)
                    .wrap_err_with(|| format!("parse fixture {}", path.display()))?;
                Ok(summary)
            }
        }
    }
}

async fn fetch_live(base: &str, http: &reqwest::Client) -> Result<Summary> {
    let summary_url = format!("{base}/api/v2/summary.json");
    let recent_url = format!("{base}/api/v2/incidents.json");
    let history_urls: Vec<String> = (1..=HISTORY_PAGES)
        .map(|p| format!("{base}/history.json?page={p}"))
        .collect();

    // Phase 1: summary + recent typed incidents in parallel.
    let (summary, recent) = tokio::try_join!(
        fetch_json::<Summary>(http, &summary_url),
        fetch_json::<IncidentList>(http, &recent_url),
    )?;

    // History pages are best-effort; tolerate failures so the recent window
    // still renders if the undocumented endpoint changes shape.
    let history: Vec<HistoryResponse> = stream::iter(history_urls)
        .map(|url| async move { fetch_json::<HistoryResponse>(http, &url).await })
        .buffer_unordered(HISTORY_PAGES)
        .filter_map(|r| async move { r.ok() })
        .collect()
        .await;
    let cutoff = (Utc::now() - chrono::Duration::days(crate::bars::WINDOW_DAYS)).date_naive();
    let history_codes: Vec<String> = history
        .into_iter()
        .flat_map(|h| h.months.into_iter())
        .filter(|m| month_overlaps(m, cutoff))
        .flat_map(|m| m.incidents.into_iter().map(|i| i.code))
        .collect();

    // Phase 2: parallel detail fetch for codes not already in the recent window.
    let recent_ids: HashSet<String> = recent.incidents.iter().map(|i| i.id.clone()).collect();
    let missing: Vec<String> = history_codes
        .into_iter()
        .filter(|c| !recent_ids.contains(c))
        .collect();

    let detail_results: Vec<Result<Incident>> = stream::iter(missing)
        .map(|code| async move {
            let url = format!("{base}/api/v2/incidents/{code}.json");
            fetch_json::<IncidentEnvelope>(http, &url)
                .await
                .map(|e| e.incident)
        })
        .buffer_unordered(DETAIL_FETCH_CONCURRENCY)
        .collect()
        .await;

    let mut all = recent.incidents;
    for inc in detail_results.into_iter().flatten() {
        all.push(inc);
    }
    Ok(Summary {
        incidents: all,
        ..summary
    })
}

async fn fetch_json<T: serde::de::DeserializeOwned>(http: &reqwest::Client, url: &str) -> Result<T> {
    http.get(url)
        .send()
        .await
        .wrap_err_with(|| format!("GET {url}"))?
        .error_for_status()
        .wrap_err_with(|| format!("status check {url}"))?
        .json()
        .await
        .wrap_err_with(|| format!("decode JSON from {url}"))
}

impl ComponentStatus {
    pub fn is_operational(self) -> bool {
        matches!(self, Self::Operational)
    }
}

impl IncidentStatus {
    pub fn is_resolved(self) -> bool {
        matches!(self, Self::Resolved | Self::Completed | Self::Postmortem)
    }

    pub fn label(self) -> &'static str {
        match self {
            Self::Investigating => "INVESTIGATING",
            Self::Identified => "IDENTIFIED",
            Self::Monitoring => "MONITORING",
            Self::Resolved => "RESOLVED",
            Self::Postmortem => "POSTMORTEM",
            Self::Scheduled => "SCHEDULED",
            Self::InProgress => "IN PROGRESS",
            Self::Verifying => "VERIFYING",
            Self::Completed => "COMPLETED",
        }
    }
}

impl Impact {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "none",
            Self::Minor => "minor",
            Self::Major => "major",
            Self::Critical => "critical",
            Self::Maintenance => "maintenance",
        }
    }
}

impl Indicator {
    pub fn label(self) -> &'static str {
        match self {
            Self::None => "operational",
            Self::Minor => "minor",
            Self::Major => "major",
            Self::Critical => "critical",
            Self::Maintenance => "maintenance",
        }
    }
}
