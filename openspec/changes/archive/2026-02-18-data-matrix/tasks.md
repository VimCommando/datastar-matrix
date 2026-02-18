## 1. Project Setup and Feature Gating

- [x] 1.1 Create crate/module structure for simulation, terminal rendering, web streaming, telemetry, and shared frame types
- [x] 1.2 Add Cargo feature flag for web server support and gate Axum/SSE modules behind it
- [x] 1.3 Implement CLI parsing for `--fps`, `--port`, and `--server`
- [x] 1.4 Implement random available port selection when `--port` is omitted and web feature is enabled

## 2. Shared Simulation Engine

- [x] 2.1 Implement simulation state model with top-left anchored grid coordinates and frame identifier tracking
- [x] 2.2 Implement non-deterministic run initialization so starting grid varies each run
- [x] 2.3 Implement configurable target framerate with default 60 FPS and `--fps` override
- [x] 2.4 Implement glyph pools (numeric, alphabetic, katakana) and default weighted selection (60/20/20)
- [x] 2.5 Define glyph weight constants/config points to allow future tuning without algorithm rewrites
- [x] 2.6 Implement resize handling in simulation: clip on shrink, extend on growth, preserve top-left alignment

## 3. Frame Contract and Fan-Out

- [x] 3.1 Define shared frame payload schema supporting sparse delta events and full keyframe events
- [x] 3.2 Implement broadcast-based fan-out from simulation tick loop to terminal and web consumers
- [x] 3.3 Implement per-tick emission using shared simulation clock for both terminal and SSE paths
- [x] 3.4 Add frame ordering guarantees (monotonic frame id) and stale-frame detection helpers for clients

## 4. Terminal Renderer and Telemetry Overlay

- [x] 4.1 Implement Ratatui matrix rendering for green alphanumeric/katakana falling columns
- [x] 4.2 Implement terminal bounds-safe drawing with no out-of-bounds writes
- [x] 4.3 Implement terminal resize behavior: top-left anchored continuity, clip on shrink, fill new space on growth
- [x] 4.4 Implement `?` keybinding to toggle telemetry overlay visibility
- [x] 4.5 Implement bottom-right telemetry overlay showing `clients`, `frames`, `fps`, and `speed`

## 5. Axum SSE Server

- [x] 5.1 Implement web server bootstrap on configured or random port when web feature is enabled
- [x] 5.2 Implement `/events` SSE endpoint with unauthenticated HTTP access
- [x] 5.3 Implement late-join behavior: send full keyframe on next tick, then sparse deltas
- [x] 5.4 Implement multi-client streaming with bounded buffering and slow-client stale-frame dropping
- [x] 5.5 Implement stream telemetry counter updates for active clients, emitted frames, and dropped frames

## 6. Browser Datastar Sync

- [x] 6.1 Implement Datastar signal mapping from SSE keyframe and delta payloads
- [x] 6.2 Implement in-order frame application with out-of-order stale frame rejection
- [x] 6.3 Implement browser parity rendering path that converges to live simulation state
- [x] 6.4 Implement disconnect detection and centered `[ Disconnected ]` fallback display

## 7. Runtime Coordination and Failure Semantics

- [x] 7.1 Implement coordinated runtime startup for simulation, terminal loop, and optional web server
- [x] 7.2 Implement fail-fast shutdown: terminal failure stops web/simulation and web failure stops terminal/simulation
- [x] 7.3 Implement graceful cancellation and cleanup for all tasks and channels

## 8. Validation and Regression Tests

- [x] 8.1 Add unit tests for glyph weighting defaults and configurable weight constants
- [x] 8.2 Add unit tests for resize semantics (top-left clip/extend behavior)
- [x] 8.3 Add integration tests for default 60 FPS and `--fps` override handling
- [x] 8.4 Add integration tests for random-port binding when `--port` is omitted
- [x] 8.5 Add integration tests for keyframe-on-join and sparse delta streaming behavior
- [x] 8.6 Add integration tests for slow-client frame dropping and telemetry counter updates
- [x] 8.7 Add integration tests for coupled failure shutdown behavior
- [x] 8.8 Add browser/client tests for stale-frame rejection and `[ Disconnected ]` indicator behavior
