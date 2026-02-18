## Context

The current system runs a shared matrix simulation in Rust and renders it to both terminal and browser, with browser frames delivered through Datastar signals and a packed base64 payload (`glyph`, `luminance` per cell). Existing controls include pause and speed changes through `/cmd/{op}` endpoints. The new change adds click-triggered ripple effects that must stay visually consistent across terminal and web output while preserving current frame cadence, pause semantics, and top-left anchored viewport behavior.

Constraints:
- Ripple is interactive and time-bounded (750ms total).
- Ripple radius is capped at 16 character cells.
- Ripple modifies both glyph and luminance for existing cells.
- Browser click coordinates must map deterministically to matrix cell coordinates.

## Goals / Non-Goals

**Goals:**
- Add a command contract `POST /cmd/glitch` with payload `{"x": int, "y": int}`.
- Apply a circular ripple effect centered on the clicked cell, expanding over time and fading out by 750ms.
- Merge ripple influence into the existing frame generation pipeline so terminal and browser views reflect the same simulation state.
- Support multiple concurrent ripples with bounded memory and predictable per-tick cost.

**Non-Goals:**
- Deterministic replay across runs.
- New authentication or security hardening for command endpoints.
- Pixel-perfect sub-cell interpolation; ripple is cell-based.
- Replacing existing matrix rain mechanics.

## Decisions

1. **Represent ripples as transient simulation entities**
- Decision: Add a `Ripple` state object containing `origin_x`, `origin_y`, `start_tick_or_time`, `duration_ms=750`, and `max_radius=16`.
- Rationale: Keeps ripple behavior in the core simulation so both render targets consume the same computed state.
- Alternative considered: Client-only ripple effects in browser.
  - Rejected because it would diverge from terminal output and break shared-state guarantees.

2. **Compute ripple influence during per-frame cell synthesis**
- Decision: During each frame, compute each active ripple’s influence on candidate cells and blend onto the base rain cell output.
- Rationale: Reuses the existing frame emission path and avoids protocol changes beyond the new command.
- Alternative considered: Precompute and patch sparse cell deltas.
  - Deferred due to higher complexity for first version and additional merge logic.

3. **Use ring-band distance with temporal fade**
- Decision: Ripple visibility is strongest near the current wavefront radius `r(t)` and fades with both distance-from-band and time-to-expiry.
- Rationale: Produces recognizable “raindrop on puddle” circles while preserving readability of matrix columns.
- Alternative considered: Full filled-disc perturbation.
  - Rejected because it overwhelms too many cells and weakens rain structure.

4. **Glyph and luminance perturbation are both applied**
- Decision: Ripple modifies luminance (brighten then decay) and glyph (controlled remap/randomized substitution within configured glyph pool) with clamped bounds.
- Rationale: Matches requested effect while preventing invalid glyph encoding or overbright artifacts.
- Alternative considered: Luminance-only effect.
  - Rejected because requirement explicitly includes glyph impact.

4b. **Leading-edge luminance hold**
- Decision: Keep leading-edge luminance at full brightness for roughly the first 400ms of ripple lifetime, then apply decay through expiry.
- Rationale: Improves readability/impact of the ripple while preserving a clear fade phase.
- Alternative considered: Immediate linear fade from ripple start.
  - Rejected as visually too dim in early lifecycle.

5. **Integrate command through CQRS endpoint family**
- Decision: Add `POST /cmd/glitch` in the same command router style as existing `/cmd/{op}` controls.
- Rationale: Keeps Datastar control flow consistent and avoids reintroducing legacy query-style endpoints.
- Alternative considered: Multiplex glitch under existing generic control op.
  - Rejected for weaker API clarity and typing.

6. **Viewport coordinate mapping occurs in web input layer**
- Decision: Browser converts click location to matrix cell coordinates based on canvas cell metrics and sends integer `{x,y}`.
- Rationale: Minimizes backend ambiguity and keeps backend focused on simulation semantics.
- Alternative considered: Send pixel coordinates and map server-side.
  - Rejected due to extra server context requirements and potential mismatch on resize timing.

7. **Bounded concurrent ripples**
- Decision: Keep an upper bound on active ripples (e.g., fixed small vector/ring buffer); drop oldest expired-first when full.
- Rationale: Caps worst-case per-frame work during rapid clicking.
- Alternative considered: Unbounded ripple list.
  - Rejected for unbounded CPU growth risk.

8. **Glitch creation rate limit**
- Decision: Allow creation of at most one new glitch/ripple per simulation frame.
- Rationale: Provides a natural rate limit tied to tick cadence and prevents click bursts from causing unbounded spawn pressure.
- Alternative considered: Time-window throttling independent of frame rate.
  - Rejected for added control complexity without clear benefit.

## Risks / Trade-offs

- **[Performance overhead from ripple-cell blending]** -> Mitigation: Use radius-bounded checks, prune expired ripples each tick, and cap concurrent ripple count.
- **[Visual noise if glyph changes are too aggressive]** -> Mitigation: Limit glyph substitution probability by influence strength; keep strongest effect near wavefront only.
- **[Coordinate mismatch on resize boundaries]** -> Mitigation: Use latest reactive viewport dimensions in click mapping and clamp `{x,y}` to current matrix bounds server-side.
- **[Concurrent ripples overpower baseline luminance model]** -> Mitigation: Use saturating blend/clamp and weight each ripple contribution.

## Migration Plan

1. Add simulation data model support for active ripples and lifecycle pruning.
2. Add `POST /cmd/glitch` handler and validation/clamping for `{x,y}` payload.
3. Integrate ripple blending into frame synthesis for glyph+luminance.
4. Add web click handler that maps canvas click to cell coordinates and sends command.
5. Validate behavior manually in terminal and web: radius cap (16), duration (750ms), bright-hold (~400ms), fade, and cross-view consistency.

Rollback:
- Disable ripple integration path and reject/ignore `/cmd/glitch` while preserving all existing controls and rendering.

## Open Questions

- None currently.
