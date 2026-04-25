Feature: Data refresh
  As a user keeping a status terminal open during an incident
  I want fresh data to appear automatically without flicker
  So that I always see the current state without having to interact

  Background:
    Given the app is running and has rendered an initial set of data

  Scenario: Auto-refresh fires on a fixed interval
    When 60 seconds elapse since the last successful fetch
    Then the app issues a new GET to /api/v2/summary.json
    And the "↻ <time> ago" indicator updates to "just now" on success

  Scenario: Manual refresh
    When the user presses `r`
    Then the app issues an immediate GET to /api/v2/summary.json
    And a transient "refreshing..." toast appears in the footer

  Scenario: Stale-while-revalidate
    Given a refresh is in flight
    Then the previously rendered data remains visible
    And no skeleton placeholder is shown

  Scenario: Network error surfaces a non-fatal toast
    When the next refresh fails
    Then a toast appears in the footer with the error message
    And the app does not exit
    And the last successful timestamp is preserved

  Scenario: Custom refresh interval via flag
    When the user runs `claude-status --refresh 30`
    Then auto-refresh fires every 30 seconds instead of every 60

  Scenario: Fixture mode reloads from disk
    Given the user runs `claude-status --fixture <path>`
    When the user presses `r`
    Then the file is re-read and the UI updates from the new contents
