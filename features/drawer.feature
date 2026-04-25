Feature: Detail drawer
  As a user investigating an incident
  I want a side panel showing the full timeline of updates
  So that I do not have to open a browser to read the post-incident communication

  Background:
    Given the app is fully loaded with real or fixture data

  Scenario: Drawer is open by default and pinned to the right
    Then the drawer is open
    And the drawer is rendered against the right edge of the terminal
    And the drawer width is between 40 and 64 columns, scaled to the terminal width

  Scenario: Drawer reflects the focused pane
    Given focus is on Sections and a component group is selected
    Then the drawer shows the component's status, description, and child components

    Given focus is on Events and an incident is selected
    Then the drawer shows the incident's metadata followed by all updates newest-first

    Given focus is on the Uptime bars region and a day cell is highlighted
    Then the drawer shows the date, the day's status, and any incidents touching that day

  Scenario: Incident updates show a colored status pill
    Given the drawer is showing an incident with updates
    Then each update is prefixed with a timestamp and a status label
    And resolved updates use the "success" color
    And in-progress updates use the impact color

  Scenario: `d` toggles the drawer
    Given the drawer is open
    When the user presses `d`
    Then the drawer animates closed using tui-overlay's slide
    When the user presses `d` again
    Then the drawer animates open

  Scenario: Esc closes the drawer; a second Esc quits
    Given the drawer is open
    When the user presses Esc
    Then the drawer closes
    When the user presses Esc again
    Then the app exits cleanly and restores the terminal state
