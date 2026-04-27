Feature: Keyboard navigation between panes
  As a terminal-only user
  I want to move between every region with the keyboard alone
  So that I never need a mouse to inspect Claude's status

  Background:
    Given the app is fully loaded with at least one service and one incident

  Scenario: Default focus is the Services pane
    Then the Services pane has the focused border style
    And the Events pane has the unfocused border style

  Scenario: Tab toggles focus between Services and Events
    Given focus is on Services
    When the user presses Tab
    Then focus moves to Events
    When the user presses Tab
    Then focus returns to Services

  Scenario: Shift-Tab toggles focus the other way (same with two panes)
    Given focus is on Services
    When the user presses Shift-Tab
    Then focus moves to Events

  Scenario Outline: Vertical movement keys
    Given focus is on the "<pane>" pane
    When the user presses "<key>"
    Then the selection moves "<direction>" within that pane
    And the modal does not open

    Examples:
      | pane     | key | direction |
      | Services | ↓   | down      |
      | Services | j   | down      |
      | Services | ↑   | up        |
      | Services | k   | up        |
      | Events   | ↓   | down      |
      | Events   | k   | up        |

  Scenario: Horizontal keys scrub days when Services has focus
    Given focus is on Services
    When the user presses ←/→
    Then the day cursor on the focused service moves left/right
    And the modal does not open

  Scenario: Selection wraps at the end of a list
    Given focus is on Services and the last service is selected
    When the user presses ↓
    Then the selection wraps to the first service

  Scenario: Enter on Services opens the focused service-day in the modal
    Given focus is on Services and a day cell is highlighted
    When the user presses Enter
    Then the modal opens showing that day's incidents for the focused service

  Scenario: Enter on Events opens the selected incident in the modal
    Given focus is on Events and an incident is selected
    When the user presses Enter
    Then the modal opens and shows that incident's detail
