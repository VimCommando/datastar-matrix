## Purpose

Define terminal rendering behavior for the Matrix output, including glyph/color presentation, resize semantics, and telemetry overlay controls.

## Requirements

### Requirement: Terminal matrix animation rendering
The system SHALL render a continuously updating Matrix-style animation in the active terminal using Ratatui.

#### Scenario: Animation loop starts in terminal mode
- **WHEN** the user launches the application in terminal mode
- **THEN** the terminal displays animated falling character columns without requiring browser access

### Requirement: Matrix glyph and color presentation
The terminal renderer SHALL display alphanumeric and katakana glyphs in green with top-to-bottom column descent behavior.

#### Scenario: Rendered frame uses required glyph classes and color
- **WHEN** a frame is rendered in the terminal
- **THEN** visible glyphs include characters from the configured alphanumeric and katakana sets and are styled in green

### Requirement: Terminal bounds compliance
The renderer MUST clip drawing operations to the current terminal viewport and MUST adapt to terminal resize events.

#### Scenario: Terminal window is resized during playback
- **WHEN** terminal dimensions change while animation is running
- **THEN** subsequent frames are rendered within new bounds without panics or out-of-bounds artifacts

### Requirement: Top-left anchored resize behavior
The terminal renderer MUST preserve top-left anchoring across viewport size changes.

#### Scenario: Terminal viewport shrinks
- **WHEN** terminal size decreases while animation is running
- **THEN** rendered output is clipped to the new viewport while preserving top-left-aligned content continuity

#### Scenario: Terminal viewport expands
- **WHEN** terminal size increases while animation is running
- **THEN** newly visible rows and columns are rendered as additional matrix space that fills through normal top-to-bottom character progression

### Requirement: Telemetry overlay toggle keybinding
The terminal renderer SHALL toggle a telemetry overlay when the user presses `?`.

#### Scenario: User presses question mark key
- **WHEN** the application is running and the user presses `?`
- **THEN** the telemetry overlay visibility toggles between hidden and visible

### Requirement: Bottom-right telemetry counters
When visible, the telemetry overlay MUST be anchored in the bottom-right of the terminal view and display `clients`, `frames`, `fps`, and `speed` counters.

#### Scenario: Telemetry overlay is enabled
- **WHEN** the overlay is visible during runtime
- **THEN** the terminal shows `clients`, `frames`, `fps`, and `speed` values in a bottom-right overlay
