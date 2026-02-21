## Context

The browser currently renders disconnect state as a separate DOM overlay (`[ Disconnected ]`) above the matrix canvas. This solves visibility but breaks the illusion that the browser view is itself a terminal-like matrix surface. We want disconnect messaging to be rendered as matrix characters inside the same canvas path, matching the visual language of the stream and avoiding extra UI layers.

## Goals / Non-Goals

**Goals:**
- Replace separate disconnect DOM overlay with in-band matrix character rendering.
- Render a centered ASCII-style `[ SIGNAL LOST ]` treatment that appears when stream updates are stale/disconnected.
- Preserve existing stale timeout and reconnect behavior so normal matrix rendering resumes automatically when frames return.
- Keep implementation within current browser rendering architecture (canvas + packed glyph/luminance frames).

**Non-Goals:**
- Changing server stream cadence, transport protocol, or heartbeat semantics.
- Adding new backend endpoints or disconnect event payload types.
- Introducing heavy animation systems beyond lightweight canvas text composition.

## Decisions

1. Disconnect treatment is rendered in canvas, not a separate DOM layer.
- Decision: remove/hide the dedicated disconnect element and draw `[ SIGNAL LOST ]` directly within the matrix canvas.
- Rationale: preserves a single rendering surface and consistent terminal aesthetic.
- Alternatives considered:
  - Keep existing overlay and restyle text: rejected because it still appears as a detached UI layer.

2. Use existing glyph rendering loop with an explicit disconnected frame pass.
- Decision: when stale timeout triggers, render a synthetic matrix frame that includes centered message glyphs and subdued background cells.
- Rationale: reuses established draw pipeline and avoids introducing a second rendering engine.
- Alternatives considered:
  - Replace canvas with DOM text grid: rejected due performance and architectural churn.

3. Recovery remains passive and immediate.
- Decision: first valid frame after disconnect clears signal-lost treatment and resumes normal rendering.
- Rationale: matches current behavior expectations and minimizes state complexity.
- Alternatives considered:
  - Explicit reconnect transition animation: rejected for scope and complexity.

4. Keep stale detection threshold unchanged.
- Decision: preserve existing timeout threshold and only change visual output.
- Rationale: avoids behavior regressions in connection health semantics.

## Risks / Trade-offs

- [ASCII message may reduce readability on very small viewports] -> Mitigation: clamp placement and degrade gracefully (clip or shorten) while keeping message recognizable.
- [Disconnect rendering path could conflict with stale-frame guard] -> Mitigation: isolate disconnected draw path from frame-id stale checks and cover with browser tests.
- [Visual parity with terminal may still not be exact] -> Mitigation: ensure style remains matrix-native (glyph palette and spacing), not overlay-like.

## Migration Plan

1. Remove standalone disconnect overlay usage from browser markup/script.
2. Implement disconnected canvas render path for `[ SIGNAL LOST ]` in matrix character space.
3. Keep existing stale timeout/recovery hooks, mapped to new in-band renderer.
4. Update browser tests to assert in-band disconnect behavior and absence of separate overlay dependency.
5. Validate manually in browser: disconnect shows `[ SIGNAL LOST ]`, reconnect resumes live matrix.

## Open Questions

- Should the message include a subtle blink/flicker effect in this change, or stay static for readability?
