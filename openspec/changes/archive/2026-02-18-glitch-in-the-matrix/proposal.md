## Why

The matrix display currently supports continuous rain and control toggles, but it has no direct interactive visual response to pointer input. Adding a click-triggered ripple effect introduces a distinctive “glitch” interaction that makes the experience feel more alive while preserving the core matrix aesthetic.

## What Changes

- Add click-driven ripple events that originate at the clicked matrix cell and radiate outward in a circular wave.
- Apply ripple influence to both glyph and luminance of existing cells with a fade profile over time.
- Enforce ripple constraints: maximum radius 16 characters and total animation duration 500ms.
- Integrate ripple updates into the existing shared simulation so browser and terminal remain visually consistent for the same frame state.
- Extend web command/control flow to send click coordinates into the backend simulation.
- Define a dedicated glitch command endpoint: `POST /cmd/glitch` with JSON payload `{"x": int, "y": int}` indicating the clicked matrix cell.

## Capabilities

### New Capabilities
- `matrix-ripple-effects`: Interactive, time-bounded circular ripple simulation triggered by click coordinates and merged with matrix frame generation.

### Modified Capabilities
- `datastar-matrix-sync`: Add pointer-input command handling and coordinate mapping from viewport space to matrix cell space.
- `sse-matrix-stream`: Ensure ripple-updated frames are emitted through the existing stream cadence without breaking pause/heartbeat behavior.

## Impact

- Affected code:
  - Simulation/state engine (new ripple lifecycle state, per-tick decay, radius expansion)
  - Web frontend canvas input handling (click-to-cell translation and command dispatch)
  - Backend control endpoints/handlers (new click/ripple command)
  - Frame packing/stream path (propagation of ripple-influenced glyph/luminance)
- APIs:
  - Add `POST /cmd/glitch` with body `{"x": int, "y": int}` for click/ripple initiation.
- Dependencies:
  - No new external runtime dependencies expected.
