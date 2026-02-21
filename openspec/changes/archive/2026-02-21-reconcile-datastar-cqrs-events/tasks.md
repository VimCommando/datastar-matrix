## 1. Codify CQRS + Datastar Contracts

- [x] 1.1 Add/adjust tests to assert `/cmd/*` returns `204 No Content` with no state payload and that unsupported ops do not return frame data.
- [x] 1.2 Add/adjust tests to assert UI-observable state changes are sourced from `/events` SSE payloads after commands.
- [x] 1.3 Update architecture/docs comments in web transport code to reflect command-ingress/query-egress contract.

## 2. Shift Frontend Interaction Wiring Toward Datastar Attributes

- [x] 2.1 Refactor keyboard command dispatch so event triggers originate from `data-on:*` attribute handlers.
- [x] 2.2 Refactor resize and pointer-triggered command dispatch to originate from `data-on:*` handlers, keeping helper JS focused on coordinate/canvas mechanics.
- [x] 2.3 Reduce global controller surface (`window.__matrixDatastar`) to rendering and minimal helpers only.
- [x] 2.4 Verify the refactored attribute-driven interactions and helper layer remain compatible with supported browsers (no browser-specific regressions).

## 3. Verify Stream Semantics Remain Stable

- [x] 3.1 Confirm `/events` payload and cadence semantics remain unchanged in HTTP and HTTPS modes after frontend wiring refactors.
- [x] 3.2 Add/maintain tests that prove command acknowledgements do not carry duplicated frame state and SSE remains authoritative.
- [x] 3.3 Validate Signal Lost stale/recovery behavior continues to operate correctly with refactored attribute-driven command initiation.
