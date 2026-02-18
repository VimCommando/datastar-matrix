## Why

There is no single executable that renders a Matrix-style animation locally in the terminal while also exposing the same live character stream to a browser client. Building this now establishes a shared rendering model for terminal and web outputs, enabling both immersive CLI visuals and remote viewing from the same source of truth.

## What Changes

- Add a Rust command-line application that renders Matrix-style falling columns in the active terminal using Ratatui.
- Generate streams with green alphanumeric and katakana characters that descend top-to-bottom in independently evolving columns.
- Add an Axum web server in the same application that publishes the same frame/character state over Server-Sent Events.
- Add a Datastar-compatible frontend contract driven by server signals so the browser reproduces the same Matrix output as the terminal view.
- Define synchronization behavior so terminal and web consumers are fed from the same simulation timeline.
- Add a configurable target framerate for simulation/render updates with a default of 60 FPS.
- Expose CLI options `--fps`, `--port`, and `--server`, defaulting port to a random available value when omitted, and gate web server availability behind a compile-time feature flag.

## Capabilities

### New Capabilities
- `terminal-matrix-renderer`: Render Matrix-style animated columns in a terminal UI with Ratatui, including green alphanumeric and katakana glyph output.
- `matrix-state-simulation`: Produce deterministic or shared-timeframe column/character evolution that can be consumed by multiple outputs.
- `sse-matrix-stream`: Stream matrix state updates from Axum over SSE for browser clients.
- `datastar-matrix-sync`: Represent incoming stream updates as Datastar signals so the browser renders the same matrix state as the terminal.

### Modified Capabilities
- None.

## Impact

- Affected code: new Rust crate/modules for simulation, terminal rendering, HTTP/SSE transport, and frontend integration assets.
- APIs: new local HTTP endpoints for SSE streaming and optional static frontend delivery.
- Dependencies: Ratatui/TUI stack, Axum async web stack, SSE support, and Datastar frontend runtime.
- Systems: terminal runtime performance and frame cadence now influence both local rendering and browser stream consistency.
