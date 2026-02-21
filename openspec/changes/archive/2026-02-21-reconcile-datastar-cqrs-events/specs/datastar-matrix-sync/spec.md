## MODIFIED Requirements

### Requirement: Datastar signal mapping for matrix frames
The web client integration SHALL map incoming stream updates into Datastar signals representing the active matrix frame, including ripple-influenced glyph and luminance values produced by the backend simulation, and this mapping SHALL remain consistent when the page is served from a secure HTTPS origin. During stale periods, the browser rendering layer SHALL present Signal Lost state through in-band matrix character rendering rather than a separate overlay element. Frontend interaction wiring SHALL prefer Datastar `data-*` attributes for event handling and state transitions, while limiting imperative JavaScript to canvas rendering and coordinate math that cannot be expressed declaratively.

#### Scenario: SSE frame arrives in browser integration layer
- **WHEN** a valid matrix frame event is received by the web client
- **THEN** Datastar signals are updated so UI state reflects the new frame data

#### Scenario: Browser is connected to secure origin
- **WHEN** the web UI is loaded over HTTPS in the default secure mode and receives `/events` updates from the same secure origin
- **THEN** the renderer applies frame updates without changing parity behavior relative to the terminal output

#### Scenario: Stream stalls long enough to be considered Signal Lost
- **WHEN** backend updates stop beyond stale timeout threshold
- **THEN** the browser renders an in-band `[ SIGNAL LOST ]` matrix treatment instead of showing a separate overlay element

#### Scenario: Stream recovers after Signal Lost state
- **WHEN** a fresh frame is received after Signal Lost rendering was shown
- **THEN** in-band Signal Lost rendering is cleared and live matrix rendering continues

## ADDED Requirements

### Requirement: Datastar attributes initiate command workflows
Command-producing browser interactions MUST be initiated from Datastar `data-on:*` attribute handlers, and any required command payload derivation MAY be delegated to small helper functions. This helper layer SHALL remain minimal and MUST preserve compatibility with supported browsers.

#### Scenario: Keyboard command is triggered
- **WHEN** the user presses a mapped control key
- **THEN** a Datastar `data-on:*` handler initiates the corresponding command request

#### Scenario: Pointer glitch command requires coordinate mapping
- **WHEN** a pointer event occurs on the matrix canvas
- **THEN** a Datastar `data-on:*` handler invokes minimal helper logic to map viewport coordinates and initiates the glitch command request
