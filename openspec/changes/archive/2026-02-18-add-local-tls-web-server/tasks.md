## 1. Configuration and dependency setup

- [x] 1.1 Add CLI/config fields for `--tls-cert`, `--tls-key`, and `--insecure`, then parse them into runtime config.
- [x] 1.2 Add and wire Rustls/Axum TLS serving dependencies needed for optional HTTPS mode.
- [x] 1.3 Add config validation that enforces required TLS arguments in default secure mode and returns clear startup errors.

## 2. Web server transport implementation

- [x] 2.1 Refactor web server startup to default to HTTPS and select HTTP only when `--insecure` is set.
- [x] 2.2 Ensure the same router and middleware stack (including compression and `/events`) is used in both modes.
- [x] 2.3 Surface effective scheme/URL in startup output so developers can reliably open the correct origin.

## 3. Behavior and compatibility verification

- [x] 3.1 Add unit/integration tests for TLS config validation cases (missing pair, invalid files, valid pair).
- [x] 3.2 Add/adjust server tests to confirm existing routes and SSE behavior remain unchanged across transport modes.
- [x] 3.3 Manually verify local HTTPS flow in browser and confirm `Accept-Encoding` includes `br` with Brotli negotiated when available.

## 4. Documentation and developer workflow

- [x] 4.1 Document required local certificate generation/trust workflow for development (for example with `mkcert`).
- [x] 4.2 Document secure-default startup and HTTP fallback behavior via `--insecure`.
