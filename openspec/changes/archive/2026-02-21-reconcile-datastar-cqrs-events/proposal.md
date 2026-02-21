## Why

Current behavior intentionally uses CQRS semantics where browser commands are accepted via REST (`POST /cmd/*` returning `204 No Content`) and live state is delivered through SSE. This is valid for the project, but the current specs do not explicitly codify this split contract or how Datastar-first attribute usage should coexist with that command path.

## What Changes

- Clarify and formalize CQRS transport boundaries: command ingress via REST `POST /cmd/*` with `204`, query/signal egress via `/events` SSE.
- Define Datastar frontend interaction expectations that favor `data-*` attributes for event wiring and state flow while allowing minimal imperative JavaScript for canvas rendering and coordinate translation.
- Specify resilience and consistency guarantees so command acknowledgements do not rely on HTTP response bodies and UI updates continue to be derived from SSE-delivered signals.

## Capabilities

### New Capabilities
- `datastar-cqrs-command-contract`: Define explicit command/query transport behavior between Datastar frontend and Axum backend.

### Modified Capabilities
- `datastar-matrix-sync`: Extend browser integration requirements to prefer `data-*` event attributes while preserving the intentional REST command path and SSE-driven rendering updates.
- `sse-matrix-stream`: Clarify that command-side HTTP `204` acknowledgements do not change stream payload semantics and SSE remains the source of truth for observable UI state.

## Impact

- Affected code: `/Users/reno/Development/data-matrix/src/web.rs` frontend markup/event handlers and Axum routing/handlers for `/events` and `/cmd/*`.
- APIs: no net-new public transport required; existing command and stream endpoints are normalized by spec.
- Dependencies/systems: no new dependencies expected; primary impact is behavior clarification and targeted refactors toward Datastar attribute-driven interaction.
