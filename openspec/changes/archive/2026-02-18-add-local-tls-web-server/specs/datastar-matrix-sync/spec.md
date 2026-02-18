## MODIFIED Requirements

### Requirement: Datastar signal mapping for matrix frames
The web client integration SHALL map incoming stream updates into Datastar signals representing the active matrix frame, including ripple-influenced glyph and luminance values produced by the backend simulation, and this mapping SHALL remain consistent when the page is served from a secure HTTPS origin.

#### Scenario: SSE frame arrives in browser integration layer
- **WHEN** a valid matrix frame event is received by the web client
- **THEN** Datastar signals are updated so UI state reflects the new frame data

#### Scenario: Browser is connected to secure origin
- **WHEN** the web UI is loaded over HTTPS in the default secure mode and receives `/events` updates from the same secure origin
- **THEN** the renderer applies frame updates without changing parity behavior relative to the terminal output
