# Statuspage API

We hit `https://status.claude.com/api/v2/summary.json`. The response
shape is documented at `https://status.claude.com/api#javascript-library`
— the same shape is used by every Atlassian Statuspage tenant.

## Fields we rely on

- `status.indicator` — `"none" | "minor" | "major" | "critical"` (we also
  accept `"maintenance"` as a fallback).
- `components[]` — `id`, `name`, `status`, `group_id`, `group: bool`,
  `position`, `created_at`. `group: true` means a top-level grouping.
- `incidents[]` — populated by [`api::fetch_live`](../src/api.rs) from
  three merged sources: `/api/v2/summary.json` (page meta +
  scheduled_maintenances), `/api/v2/incidents.json` (typed, ~50 most
  recent), and per-incident detail fetches for older codes discovered
  via `/history.json?page=N` (undocumented; codes only). Each incident
  carries `started_at`, `resolved_at`, `impact`, `components[]`,
  `incident_updates[]` (with `affected_components[]` recording
  per-component status transitions).

## Uptime math

Implemented by [`bars::compute`](../src/bars.rs) +
`accumulate_component_downtime`:

- Walk each incident's `incident_updates[*].affected_components[]` to
  build a per-component status timeline. Each transition opens a span
  ending at the next transition (same component) or earlier.
- Cap every span at the incident's **mitigation timestamp** — earliest
  update reaching `Monitoring | Resolved | Postmortem`. Statuspage stops
  counting downtime once an incident moves past identification, even if
  the affected_components payload still reads partial_outage during
  verification.
- Span weight: `major_outage → 1.0`, `partial_outage → 0.5`, everything
  else (degraded_performance, under_maintenance, operational) → 0.0.
- `Impact::None` and `Impact::Maintenance` incidents are skipped.

Matches Statuspage's published per-component 90-day uptime within ~0.2
percentage points across all six Claude services (verified by
reverse-engineering the JSON embedded in their home page HTML). We don't
call the paid `/components/{id}/uptime` endpoint.

## Fetch strategy

- Phase 1: `summary.json` + `incidents.json` + 2 history pages in
  parallel (`tokio::try_join!` + `stream::buffer_unordered`).
- Phase 2: for codes in history but not in the recent typed set,
  parallel-fetch `/api/v2/incidents/{code}.json` with concurrency capped
  at `DETAIL_FETCH_CONCURRENCY = 8`. History months whose last day falls
  before the 90-day cutoff are filtered out before detail fetches issue.
- All history calls are best-effort; failures don't fail the load.
