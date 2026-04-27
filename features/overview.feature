Feature: At-a-glance overall status
  As a developer or Claude user
  I want to see Claude's overall status the moment the app opens
  So that I can decide whether to keep working in another window or stop and triage

  Background:
    Given the Claude Statuspage v2 API is reachable at https://status.claude.com
    And the user runs `claude-status`

  Scenario: Cold start shows skeleton placeholders
    Given the app has just launched and no data has loaded yet
    Then the Services pane shows a tui-skeleton SkeletonList
    And the Events pane shows a tui-skeleton SkeletonList

  Scenario: First successful fetch replaces the skeletons
    Given the app is showing skeleton placeholders
    When `/api/v2/summary.json` returns a 200 response
    Then both panes render real data within one frame
    And the header shows the indicator dot in the color matching the status indicator
    And the right side of the header shows "page updated <relative>"

  Scenario Outline: Header indicator color matches the API status
    Given the API returned a status indicator of "<indicator>"
    Then the header dot is rendered in the theme's "<token>" color

    Examples:
      | indicator | token   |
      | none      | success |
      | minor     | warning |
      | major     | error   |
      | critical  | error   |

  Scenario: Network failure preserves the last-known data
    Given the app has previously rendered live data
    When the next refresh fails with a network error
    Then the panes still show the last-known data
    And a transient toast appears in the footer with the error message
    And the toast clears itself after a few seconds
