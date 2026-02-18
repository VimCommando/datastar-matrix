## Context

The application currently starts an Axum web server over plain HTTP and serves browser UI and `/events` from the same listener. Chrome only advertises Brotli in secure contexts for this workflow, so local HTTP prevents validating Brotli-first behavior even though server-side Brotli support is enabled. The change is cross-cutting because it touches CLI/config parsing, web-server startup, runtime logging, and browser connectivity expectations.

## Goals / Non-Goals

**Goals:**
- Make local HTTPS serving the default behavior for existing web routes.
- Add explicit CLI inputs for TLS certificate and key files.
- Add an `--insecure` flag that forces existing HTTP-only behavior when explicitly requested.
- Preserve endpoint behavior (`/`, `/events`, `/cmd/*`) and simulation cadence semantics across HTTP and HTTPS modes.
- Make TLS mode explicit and observable at startup so developers know which scheme to use.

**Non-Goals:**
- Automatic certificate generation or OS trust-store automation.
- HTTP-to-HTTPS redirection or dual listeners in the first iteration.
- Production certificate lifecycle management (ACME, rotation, revocation).

## Decisions

1. Add explicit TLS config inputs on CLI.
- Decision: introduce explicit CLI args for cert and key paths (`--tls-cert`, `--tls-key`) and default to TLS mode when `--insecure` is not set; when secure mode runs without explicit files, generate a self-signed dev certificate in-memory.
- Rationale: aligns local behavior with secure-browser expectations while keeping explicit operator control.
- Alternatives considered:
  - Keep TLS opt-in only: rejected because it keeps the default path on HTTP and undermines Brotli-default validation goals.
  - Single `--tls` boolean with implicit default paths: rejected because path conventions vary by machine.
  - Environment-only config: rejected because current app conventions are CLI-first.

2. Use Rustls-backed Axum serving path.
- Decision: use an Axum-compatible Rustls server integration (e.g. `axum-server` with rustls) when TLS is configured; keep current `axum::serve` path for HTTP mode.
- Rationale: minimizes custom TLS plumbing while keeping router/state unchanged.
- Alternatives considered:
  - Manual `tokio-rustls` accept loop + Hyper wiring: rejected due higher complexity and maintenance cost.

3. Keep route and compression middleware identical between modes.
- Decision: construct one router and reuse it for either HTTP or HTTPS serving.
- Rationale: prevents behavioral drift and keeps SSE/compression behavior consistent regardless of transport.
- Alternatives considered:
  - Separate app builders per mode: rejected due duplicate code and increased divergence risk.

4. Fail fast on invalid TLS material.
- Decision: when running in default secure mode, validate readability/compatibility of certificate and key at startup and return a clear error before serving; skip TLS validation only when `--insecure` is set.
- Rationale: avoids partial startup and ambiguous browser failures.
- Alternatives considered:
  - Lazy failure on first connection: rejected because debugging becomes harder.

## Risks / Trade-offs

- [Self-signed certificates can still show browser warnings] -> Mitigation: document recommended local certificate workflow (for example `mkcert`) and trusted-host requirements.
- [Adding TLS dependency increases binary size and compile time] -> Mitigation: keep dependencies limited to required rustls path only.
- [Misconfiguration if cert/key paths are missing or partial in default TLS mode] -> Mitigation: enforce strict startup validation with actionable error messaging and document required flags.
- [Potential differences in browser behavior between `localhost` and custom local domains] -> Mitigation: document host/certificate SAN expectations and test both.

## Migration Plan

1. Add CLI/config fields for `--tls-cert`, `--tls-key`, and `--insecure`, plus startup validation rules.
2. Make HTTPS serving the default branch and keep HTTP as explicit `--insecure` branch.
3. Update startup logs/tests to report effective scheme (`http` or `https`) and listening URL.
4. Validate local browser flow against HTTPS endpoint and confirm Brotli negotiation.
5. Rollback strategy: run with `--insecure` to return to existing HTTP behavior.
