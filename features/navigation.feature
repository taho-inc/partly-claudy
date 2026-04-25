Feature: Keyboard navigation between panes
  As a terminal-only user
  I want to move between every region with the keyboard alone
  So that I never need a mouse to inspect Claude's status

  Background:
    Given the app is fully loaded with at least one component group and one incident

  Scenario: Default focus is the Sections pane
    Then the Sections pane has the focused border style
    And the Bars and Events panes have the unfocused border style

  Scenario: Tab cycles focus forward
    Given focus is on Sections
    When the user presses Tab
    Then focus moves to Events
    When the user presses Tab
    Then focus moves to the Uptime bars region
    When the user presses Tab
    Then focus returns to Sections

  Scenario: Shift-Tab cycles focus backward
    Given focus is on Sections
    When the user presses Shift-Tab
    Then focus moves to the Uptime bars region

  Scenario Outline: Vertical movement keys
    Given focus is on the "<pane>" pane
    When the user presses "<key>"
    Then the selection moves "<direction>" within that pane
    And the drawer updates to reflect the new selection

    Examples:
      | pane     | key | direction |
      | Sections | ↓   | down      |
      | Sections | j   | down      |
      | Sections | ↑   | up        |
      | Sections | k   | up        |
      | Events   | ↓   | down      |
      | Events   | k   | up        |

  Scenario: Selection wraps at the end of a list
    Given focus is on Sections and the last item is selected
    When the user presses ↓
    Then the selection wraps to the first item

  Scenario: Enter pins the selection into the drawer
    Given focus is on Events and an incident is selected
    When the user presses Enter
    Then the drawer opens (if closed) and shows that incident's detail
