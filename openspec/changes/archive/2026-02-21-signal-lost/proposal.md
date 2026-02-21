## Why

The current disconnect handling overlays a separate `[ Disconnected ]` UI element, which breaks the terminal illusion and looks detached from the matrix itself. Embedding the disconnect state directly into the matrix characters as an ASCII-style `[ SIGNAL LOST ]` treatment preserves the diegetic terminal aesthetic while still clearly communicating stream loss.

## What Changes

- Replace the standalone disconnect overlay element in the browser renderer with an in-band matrix-rendered disconnect message.
- Render a centered ASCII-art style `[ SIGNAL LOST ]` treatment using matrix glyph cells when stream updates are stale/disconnected.
- Keep current stale timeout behavior and recovery semantics so live updates resume normal matrix rendering without page reload.
- Ensure the embedded disconnect treatment integrates with existing canvas rendering path and does not reintroduce out-of-order frame issues.

## Capabilities

### New Capabilities
- `signal-lost-overlay-in-band`: Defines in-band matrix disconnect presentation rules for browser rendering.

### Modified Capabilities
- `datastar-matrix-sync`: Update disconnect indicator requirements to use embedded matrix character rendering instead of a separate HTML overlay element.

## Impact

- Affected code: browser markup/script in `src/web.rs` (disconnect state management and canvas rendering), associated web tests.
- APIs/protocols: no backend API changes; stream contract remains unchanged.
- Runtime behavior: visual disconnect state changes in browser only; terminal and transport cadence are unchanged.
