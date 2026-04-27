Feature: Detail modal
  As a user investigating an incident
  I want a center-anchored modal showing the full timeline of updates
  So that I do not have to open a browser to read the post-incident communication

  Background:
    Given the app is fully loaded with real or fixture data

  Scenario: Modal is closed by default
    Then the modal is not visible
    And the rest of the UI is fully interactive

  Scenario: Enter on the Events pane opens the selected incident
    Given focus is on Events and an incident is selected
    When the user presses Enter
    Then the modal opens centered with a backdrop dimming the rest of the UI
    And the modal shows the incident's metadata followed by all updates newest-first

  Scenario: Enter on the Services pane opens the focused service-day
    Given focus is on Services and a day cell is highlighted
    When the user presses Enter
    Then the modal opens showing the date, the day's status, and any
    incidents touching that day for the focused service

  Scenario: Incident updates show a colored status pill
    Given the modal is showing an incident with updates
    Then each update is prefixed with a timestamp and a status label
    And resolved updates use the "success" color
    And in-progress updates use the impact color

  Scenario: Esc closes the modal; a second Esc quits
    Given the modal is open
    When the user presses Esc
    Then the modal closes
    When the user presses Esc again
    Then the app exits cleanly and restores the terminal state

  Scenario: Arrow keys navigate without opening the modal
    Given the modal is closed and focus is on Events
    When the user presses ↓
    Then the selection in Events advances
    And the modal remains closed
