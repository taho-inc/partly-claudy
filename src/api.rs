#![allow(dead_code)]

use std::path::Path;
use std::time::Duration;

use chrono::{DateTime, Utc};
use color_eyre::eyre::{Context, Result};
use serde::Deserialize;

const DEFAULT_BASE: &str = "https://status.claude.com";
const USER_AGENT: &str = concat!("claude-status/", env!("CARGO_PKG_VERSION"));

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
    pub display_at: DateTime<Utc>,
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
    #[serde(default)]
    pub past_incidents: Vec<Incident>,
}

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
            Self::Live { base, http } => {
                let url = format!("{base}/api/v2/summary.json");
                let summary: Summary = http
                    .get(&url)
                    .send()
                    .await
                    .wrap_err_with(|| format!("GET {url}"))?
                    .error_for_status()
                    .wrap_err_with(|| format!("status check {url}"))?
                    .json()
                    .await
                    .wrap_err_with(|| format!("decode JSON from {url}"))?;
                Ok(summary)
            }
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
