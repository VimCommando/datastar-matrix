## Purpose

Define secure-default local web transport behavior and HTTP opt-out semantics.

## ADDED Requirements

### Requirement: Local TLS listener configuration
The web runtime MUST default to local HTTPS serving, MUST accept explicit TLS certificate and key path arguments at startup, and MUST generate a self-signed development certificate when secure mode is used without explicit TLS files.

#### Scenario: Default secure mode startup
- **WHEN** the application is started without `--insecure` and without explicit TLS certificate/key arguments
- **THEN** the web server generates a development self-signed certificate and serves endpoints over `https://` on the configured host and port

#### Scenario: Secure mode with provided TLS files
- **WHEN** the application is started without `--insecure` and with valid TLS certificate and key arguments
- **THEN** the web server binds and serves endpoints over `https://` on the configured host and port

#### Scenario: Insecure opt-out mode
- **WHEN** the application is started with `--insecure`
- **THEN** the web server binds and serves endpoints over `http://` using the existing behavior

### Requirement: TLS startup validation
The runtime MUST fail startup with a clear error when running in default secure mode and TLS configuration is incomplete or invalid.

#### Scenario: Explicit TLS arguments are incomplete
- **WHEN** startup in default secure mode receives only one of `--tls-cert` or `--tls-key`
- **THEN** startup fails with an explicit configuration error describing the missing argument

#### Scenario: TLS files cannot be loaded
- **WHEN** startup in default secure mode cannot read or parse the configured TLS certificate or key content
- **THEN** startup fails before listening and returns an actionable error
