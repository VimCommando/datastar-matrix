## Context

The change introduces a single Rust application with two live outputs: a local terminal animation (Ratatui) and a browser view fed by Axum SSE and Datastar signals. The core constraint is visual parity: both outputs must represent the same evolving matrix state, not two independently generated animations. The application must run smoothly in a standard terminal while serving web clients concurrently with predictable frame cadence.

## Goals / Non-Goals

**Goals:**
- Define one shared simulation model that produces frame updates for all consumers.
- Render Matrix-style columns in the terminal using Ratatui with green alphanumeric and katakana glyphs.
- Expose simulation updates through Axum SSE in a format easy for Datastar to consume as signals.
- Ensure a browser connecting at runtime can render the same active matrix state and continue with the same timeline.
- Keep architecture modular so renderer, stream transport, and simulation can be tested independently.

**Non-Goals:**
- Pixel-perfect movie recreation, shader effects, or custom font packaging.
- Multi-node distributed synchronization across multiple processes or hosts.
- Full frontend framework complexity beyond Datastar signal-driven updates.
- Long-term recording/replay storage of matrix frames.

## Decisions

1. Shared simulation engine as source of truth
- Decision: Create a dedicated simulation module that owns column state and advances at a fixed target tick rate, configurable with a default of 60 FPS.
- Rationale: Prevents drift between terminal and web outputs while allowing fresh random matrix state on each run.
- Alternatives considered:
  - Separate generators for terminal and web. Rejected due to inevitable divergence and duplicated logic.
  - Browser-driven generation. Rejected because terminal output must remain primary and local.

2. Fan-out architecture with broadcast channel
- Decision: Simulation publishes snapshot/frame events through a Tokio broadcast channel; terminal renderer and SSE handlers consume from that stream.
- Rationale: Broadcast supports multiple subscribers with low coordination overhead and keeps producer/consumers decoupled.
- Alternatives considered:
  - Per-client direct callbacks from simulation. Rejected for tight coupling and complicated lifecycle handling.
  - Polling shared state from consumers. Rejected due to lock contention and inconsistent frame boundaries.

3. Frame-oriented data contract
- Decision: Define a serializable frame payload with sparse cell updates (deltas) per tick and periodic full keyframes for resynchronization.
- Rationale: Sparse updates significantly reduce per-frame payload size at large grid dimensions while periodic keyframes allow late join and recovery without unbounded replay.
- Alternatives considered:
  - Sending only random seeds to clients. Rejected because clients could still diverge by implementation details.
  - Sending full text buffers each tick. Rejected as too heavy for SSE clients at high cell counts.

4. Axum SSE endpoint for live stream
- Decision: Provide `/events/matrix` endpoint using `text/event-stream`, emitting one event per simulation tick plus a periodic keepalive. New subscribers receive a full keyframe on the next tick, then continue with sparse delta updates.
- Rationale: SSE is simple for one-way server push and aligns with Datastar signal updates.
- Alternatives considered:
  - WebSockets. Rejected for unnecessary bidirectional complexity in initial scope.
  - HTTP polling. Rejected for latency and overhead.

5. Terminal and server lifecycle under one runtime
- Decision: Run simulation task, Ratatui event loop, and Axum server in the same Tokio runtime with coordinated shutdown via cancellation token.
- Rationale: Simplifies deployment (single binary) while preserving explicit task ownership and clean exit behavior.
- Alternatives considered:
  - Separate binaries/processes. Rejected because it complicates synchronization and startup sequencing.

6. Unified clock with best-effort client rendering
- Decision: Keep a single simulation clock (default 60 FPS, configurable) as the authoritative cadence for terminal and SSE output, while allowing browser clients to drop stale frames locally under poor network conditions.
- Rationale: Preserves one source-of-truth timeline across outputs without slowing the simulation for lagging clients.
- Alternatives considered:
  - Independent client clocks. Rejected due to parity drift.
  - Server throttling to slowest client. Rejected because it degrades terminal and healthy clients.

7. Default glyph distribution with tunable weights
- Decision: Use weighted random glyph class selection with defaults of 60% numeric, 20% alphabetic, and 20% katakana, defined as configurable constants.
- Rationale: Establishes a strong default Matrix look while keeping balancing easy to tune without redesigning generation logic.
- Alternatives considered:
  - Equal distribution across classes. Rejected because it weakens the intended numeric-heavy aesthetic.
  - Hardcoded values with no tuning point. Rejected due to likely iteration during visual polish.

8. Top-left-anchored resize semantics
- Decision: Anchor matrix state to the top-left across resizes. On shrink, clip to new bounds. On growth, extend rows/columns and allow new cells to populate through normal top-to-bottom rain progression.
- Rationale: Provides predictable continuity of visible state while preserving matrix motion model in newly exposed space.
- Alternatives considered:
  - Re-center on resize. Rejected because it introduces visual jumps and breaks parity expectations.
  - Full state reset on every resize. Rejected because it is disruptive and discards ongoing animation state.

9. Minimal CLI surface with compile-time web toggle
- Decision: Expose only `--fps` and `--port` as runtime flags. If `--port` is omitted, bind web server to a random available port. Web server support is compiled in/out via a Cargo feature flag.
- Rationale: Keeps user-facing controls simple while enabling no-web builds and avoiding fixed-port collisions by default.
- Alternatives considered:
  - Mandatory fixed port input. Rejected due to poor usability and port conflict risk.
  - Runtime-only disable switch. Rejected because compile-time exclusion reduces binary surface for terminal-only deployments.

10. Coupled component failure and disconnected fallback UI
- Decision: Treat terminal loop and web server as coupled runtime components: if either fails, initiate shutdown for the full application. In browser terminal-style rendering, show `[ Disconnected ]` centered when backend updates stop.
- Rationale: Fail-fast termination avoids partial unhealthy runtime states, and explicit disconnected UI makes backend loss obvious to users.
- Alternatives considered:
  - Keep surviving component running after peer failure. Rejected because split-brain runtime behavior is misleading.
  - Silent stale-frame display in browser on disconnect. Rejected because it hides failure and implies live updates.

11. Toggleable terminal telemetry overlay
- Decision: Add a `?` keybinding that toggles a bottom-right telemetry overlay showing `clients`, `frames`, and `drops`.
- Rationale: Provides lightweight runtime visibility without permanently obscuring matrix visuals.
- Alternatives considered:
  - Always-on counters. Rejected because constant overlay reduces visual immersion.
  - External logging only. Rejected because live debugging needs in-context visibility.

12. Temporary no-auth HTTP posture
- Decision: Use simple unauthenticated HTTP connections for web/SSE access in this stage, with no authentication or transport hardening requirements.
- Rationale: Keeps initial implementation focused on rendering parity and runtime behavior before introducing security layers.
- Alternatives considered:
  - Add auth and TLS in initial scope. Rejected due to setup complexity not required for current milestone goals.

## Risks / Trade-offs

- [Terminal redraw cost may exceed tick interval on small/slow terminals] -> Mitigation: default to 60 FPS, allow user-configurable target FPS, and batch draw operations.
- [Slow SSE clients may lag or miss events] -> Mitigation: apply bounded per-client buffering and drop older queued frames so clients recover on newer frames instead of stalling the stream.
- [Different viewport sizes between terminal and browser can distort parity expectations] -> Mitigation: define simulation coordinates independent of renderer and provide renderer-specific scaling/clipping rules.
- [Katakana/alphanumeric glyph selection may include unsupported terminal glyphs] -> Mitigation: maintain validated glyph set with fallback to ASCII-safe characters.
