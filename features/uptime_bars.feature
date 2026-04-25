Feature: 90-day uptime bars
  As a user reviewing Claude's recent reliability
  I want a colored row per Claude product showing the last 90 days
  So that I can spot historical patterns and recent regressions at a glance

  Background:
    Given the app is fully loaded with real or fixture data

  Scenario: One row per top-level component group
    Then the Uptime bars region shows one row per component with `group: true`
    And each row has a name on the left, day cells in the middle, and uptime % on the right

  Scenario Outline: Cells are colored by the worst severity that day
    Given a component group with no incidents on day "<day>"
    Then the cell for day "<day>" is rendered in the theme's "success" color

    Given a component group with a "<impact>" incident overlapping day "<day>"
    Then the cell for day "<day>" is rendered in the theme's "<token>" color

    Examples:
      | impact      | token   |
      | minor       | warning |
      | major       | error   |
      | critical    | error   |
      | maintenance | info    |

  Scenario: Days before the component existed are muted
    Given a component group whose `created_at` is 30 days ago
    Then the first 60 day cells are rendered as "unknown" (muted)
    And the last 30 day cells reflect actual incident history

  Scenario: Cells coalesce worst-of when narrower than 90
    Given the bars area is 45 cells wide
    Then the renderer aggregates two days per cell
    And the cell uses the worst severity of the two days

  Scenario: Scrubbing days updates the drawer
    Given focus is on the Uptime bars region
    When the user presses ←/→
    Then the focused day cell highlights
    And the drawer updates to show that day's date and any incidents on that day

  Scenario: Right-edge label shows the rolling 90-day uptime percentage
    Then each row's right column shows a percentage formatted to two decimals
    And the percentage is colored "success" when >= 99.9%, "warning" when >= 99%, "error" otherwise
