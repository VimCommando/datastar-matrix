## MODIFIED Requirements

### Requirement: Datastar signal mapping for matrix frames
The web client integration SHALL map incoming stream updates into Datastar signals representing the active matrix frame, including ripple-influenced glyph and luminance values produced by the backend simulation.

#### Scenario: SSE frame arrives in browser integration layer
- **WHEN** a valid matrix frame event is received by the web client
- **THEN** Datastar signals are updated so UI state reflects the new frame data

## ADDED Requirements

### Requirement: Pointer click mapping to glitch command
The browser integration MUST convert pointer click position into integer matrix cell coordinates and SHALL issue `POST /cmd/glitch` with payload `{"x": int, "y": int}` for ripple initiation.

#### Scenario: User clicks on visible matrix canvas cell
- **WHEN** the user clicks within the visible matrix viewport
- **THEN** the browser sends a glitch command with mapped cell coordinates to the backend

#### Scenario: Click lands outside active matrix bounds after resize
- **WHEN** coordinate mapping yields values outside current matrix bounds
- **THEN** the client clamps or normalizes coordinates before sending the glitch command
