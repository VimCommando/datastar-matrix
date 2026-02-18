## ADDED Requirements

### Requirement: Datastar signal mapping for matrix frames
The web client integration SHALL map incoming stream updates into Datastar signals representing the active matrix frame.

#### Scenario: SSE frame arrives in browser integration layer
- **WHEN** a valid matrix frame event is received by the web client
- **THEN** Datastar signals are updated so UI state reflects the new frame data

### Requirement: Browser view follows shared timeline
The browser renderer MUST apply updates in frame order and MUST ignore out-of-order stale frames.

#### Scenario: Out-of-order event arrives after a newer frame
- **WHEN** a frame with a lower identifier is received after a higher identifier has already been applied
- **THEN** the stale frame is discarded and current rendered state remains unchanged

### Requirement: Terminal-browser parity behavior
The system SHALL preserve matrix parity semantics so browser output represents the same simulation state family as terminal output.

#### Scenario: Browser connects while terminal animation is running
- **WHEN** the browser starts receiving streamed frame updates
- **THEN** rendered browser output converges to the live simulation rather than starting an independent matrix animation

### Requirement: Disconnected state indicator
The web rendering layer MUST display `[ Disconnected ]` centered in the terminal-style browser viewport when backend stream updates stop responding.

#### Scenario: Backend stream becomes unavailable
- **WHEN** SSE updates stop for a configured disconnect timeout or connection termination is detected
- **THEN** the browser view replaces live matrix rendering with a centered `[ Disconnected ]` indicator
