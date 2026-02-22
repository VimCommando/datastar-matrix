# datastar-matrix

A realtime multiplayer, shared simulation of the matrix digital rain; simulitaneously rendered in the terminal and the browser.

[<img width="1496" height="938" alt="Matrix" src="https://github.com/user-attachments/assets/69e1e052-8f5c-4714-bf88-dff249c29daa" />
](https://youtu.be/r1ZfELi6a4g)

## What this is

This is an example of [Datastar](https://data-star.dev) and Rust, built with [Codex](https://chatgpt.com/codex) and [OpenSpec](https://openspec.dev).

- Rust simulation and terminal rendering with [`ratatui`](https://ratatui.rs)
- [Axum](https://github.com/tokio-rs/axum) SSE transport for frame streaming to modern browsers
- [Datastar](https://data-star.dev) powered browser web interface
- HTTPS will use [Brotli compression](https://andersmurphy.com/2025/04/15/why-you-should-use-brotli-sse.html) if available

## Install

```bash
cargo install --path .
```

### Run

Default HTTPS with auto-generated dev certificate:

```bash
datastar-matrix --port 40404
```

Insecure HTTP:

```bash
datastar-matrix --insecure --port 40404
```

Custom TLS cert/key:

```bash
datastar-matrix --tls-cert ./certs/dev-cert.pem --tls-key ./certs/dev-key.pem --port 40404
```

See full TLS notes in `docs/local-tls.md`.

## Controls

- `q`: quit
- `?`: toggle stats overlay
- `+` / `-`: speed up / slow down
- `0`: reset speed
- `space`: pause/resume
- Left click: create a glitch in the matrix
