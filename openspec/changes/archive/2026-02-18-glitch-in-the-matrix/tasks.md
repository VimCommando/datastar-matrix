## 1. Simulation Ripple Core

- [x] 1.1 Add ripple state model (`origin`, timing, max radius, lifecycle) to shared matrix simulation state.
- [x] 1.2 Implement per-tick ripple lifecycle updates (expand wavefront, fade strength, expire at 500ms).
- [x] 1.3 Enforce ripple constraints in simulation logic (max radius 16 cells, one new ripple creation per frame).

## 2. Frame Synthesis Integration

- [x] 2.1 Integrate ripple influence into cell synthesis so both glyph and luminance are perturbed for affected cells.
- [x] 2.2 Implement weighted-random glyph substitution strategy based on ripple influence strength.
- [x] 2.3 Clamp/normalize luminance and glyph outputs to keep packed frame encoding valid.

## 3. Backend Command Path

- [x] 3.1 Add `POST /cmd/glitch` endpoint and request payload parsing for `{"x": int, "y": int}`.
- [x] 3.2 Validate and clamp incoming glitch coordinates to current matrix bounds before simulation enqueue.
- [x] 3.3 Route accepted glitch commands into simulation input so ripple creation occurs on the next frame cycle.

## 4. Web Input and Sync

- [x] 4.1 Add canvas click handler that maps pointer position to matrix cell coordinates using current cell metrics.
- [x] 4.2 Send mapped coordinates to backend via `POST /cmd/glitch` using existing Datastar-friendly event wiring.
- [x] 4.3 Ensure resize-aware coordinate mapping stays top-left anchored and does not emit out-of-range indices.

## 5. Stream and Cross-View Consistency

- [x] 5.1 Ensure emitted `/events` frame payloads include ripple-adjusted glyph/luminance state without changing cadence semantics.
- [x] 5.2 Verify terminal and browser renderers display the same ripple-influenced frame progression for the same frame IDs.

## 6. Validation and Regression Coverage

- [x] 6.1 Add/extend tests for ripple lifecycle constraints (duration, radius, per-frame spawn limit).
- [x] 6.2 Add/extend tests for `POST /cmd/glitch` validation and coordinate clamping behavior.
- [x] 6.3 Run manual end-to-end checks for click-triggered ripple visuals, fade behavior, and performance under rapid clicking.
