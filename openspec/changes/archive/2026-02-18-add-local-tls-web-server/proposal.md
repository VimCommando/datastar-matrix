## Why

Local development currently serves the browser UI and `/events` stream over plain HTTP, which prevents Chrome from reliably advertising Brotli (`br`) and limits accurate validation of production-like compression behavior. We need first-class local TLS support so secure clients can negotiate `br` by default while keeping development setup explicit and repeatable.

## What Changes

- Make local HTTPS serving the default web transport mode.
- Add explicit TLS arguments for certificate and key paths.
- Auto-generate a self-signed development certificate/key when secure mode is used without explicit TLS files.
- Add an `--insecure` option that forces HTTP-only behavior as an opt-out.
- Keep existing compression behavior but ensure secure clients can negotiate Brotli first, with gzip retained as fallback.
- Preserve existing HTTP behavior only when explicitly requested via `--insecure`.
- Document local trust/certificate expectations for browser testing workflows.

## Capabilities

### New Capabilities
- `local-tls-web-transport`: Configure and run the local web server with TLS by default, with explicit cert/key inputs and an insecure HTTP opt-out mode.

### Modified Capabilities
- `sse-matrix-stream`: Clarify that the `/events` endpoint is available over HTTPS when TLS mode is enabled and remains behaviorally identical across transport modes.
- `datastar-matrix-sync`: Clarify browser connection expectations under secure origin usage so matrix updates continue through Datastar over HTTPS.

## Impact

- Affected code: `src/config.rs`, `src/lib.rs`, `src/web.rs`, startup/logging paths, and any docs/help text describing web access.
- Dependencies: add TLS server/runtime crates and certificate parsing support as needed.
- Runtime behavior: TLS is default transport for local development; HTTP remains available only via `--insecure`; no intended changes to frame cadence, control endpoints, or simulation semantics.
