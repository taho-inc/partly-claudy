Feature: Services pane (status + 90-day uptime + meta)
  As a user investigating Claude's reliability
  I want each service shown as a multi-line row with its status, uptime
  bar, percentage, and recent incident summary
  So that I can read the full picture without flipping panes

  Background:
    Given the app is fully loaded with real or fixture data

  Scenario: One row per service (group or standalone)
    Then the Services pane shows one row per top-level component
    And rows are ordered groups-first, then by `position`

  Scenario: Header line shows dot, name, bar, percentage
    Then each row's first line has a status dot, the service name,
    a 30-cell uptime bar, and the rolling uptime percentage

  Scenario: Meta line summarises status and recent incidents
    Then each row's second line shows the current status pill
    And when there are incidents in the window, the count and the
    age of the most recent ("· N incidents · last Xd ago")

  Scenario Outline: Day cells colored by worst severity
    Given a service with no incidents on day "<day>"
    Then the cell for day "<day>" is rendered in the theme's "success" color

    Given a service with a "<impact>" incident overlapping day "<day>"
    Then the cell for day "<day>" is rendered in the theme's "<token>" color

    Examples:
      | impact      | token   |
      | minor       | warning |
      | major       | error   |
      | critical    | error   |
      | maintenance | info    |

  Scenario: Days before component creation render as Unknown
    Given a service whose `created_at` is 10 days ago
    Then the first 20 day cells are rendered in the "dim" color
    And the last 10 day cells reflect actual incident history

  Scenario: Coalesce worst-of when bar area is narrower than 30
    Given the bar area is 15 columns wide
    Then the renderer aggregates two days per cell
    And the cell uses the worst severity of the two days

  Scenario: Scrubbing days highlights the focused cell on the focused row
    Given focus is on Services
    When the user presses ←/→
    Then the day cell at the cursor on the focused service highlights
    And the modal does not open

  Scenario: Enter on a focused service-day opens the day detail
    Given focus is on Services and a day cell is highlighted
    When the user presses Enter
    Then the modal opens showing that day's date and any incidents
    on that day for the focused service

  Scenario: Percentage colored by uptime threshold
    Then the percentage is colored "success" when >= 99.9%, "warning" when >= 99%, "error" otherwise
