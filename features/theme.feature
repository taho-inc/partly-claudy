Feature: Theme support via opaline
  As a user
  I want to pick a theme that matches my terminal palette
  So that the status UI is comfortable to read alongside the rest of my work

  Background:
    Given the app is running

  Scenario: Default builtin theme is applied at startup
    Then every widget uses colors from opaline's "SilkCircuit Neon" theme
    And no widget uses a hardcoded RGB or named color

  Scenario: `t` opens the theme picker
    When the user presses `t`
    Then a centered overlay appears listing all builtin themes
    And the currently active theme is highlighted

  Scenario: Selecting a theme updates colors live
    Given the theme picker is open
    When the user moves the selection with ↑/↓
    Then the active theme switches on each move
    And every visible widget re-renders in the new theme on the next frame

  Scenario: Closing the theme picker
    Given the theme picker is open
    When the user presses Esc, `t`, `q`, or Enter
    Then the picker closes
    And the most recently selected theme remains active

  Scenario: Semantic tokens map to status meanings
    Then operational items use the "success" token
    And degraded items use the "warning" token
    And major or critical incidents use the "error" token
    And maintenance windows use the "info" token

  Scenario: User-supplied themes are discoverable (future)
    Given a TOML file at "~/.config/claude-status/themes/my-theme.toml"
    Then the app discovers it via opaline's `discovery` feature
    And it appears in the theme picker alongside the builtins
