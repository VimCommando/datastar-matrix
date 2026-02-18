# Local TLS Development

This project now defaults to secure local web serving.

## Generate local certificate material

Use `mkcert` (recommended):

```bash
mkcert -install
mkcert -cert-file ./certs/dev-cert.pem -key-file ./certs/dev-key.pem localhost 127.0.0.1 ::1 ironhide.local
```

## Run in secure mode (default, auto-generated dev cert)

```bash
cargo run -- --port 40404
```

If you want to provide your own cert/key instead:

```bash
cargo run -- --tls-cert ./certs/dev-cert.pem --tls-key ./certs/dev-key.pem --port 40404
```

When started, the app prints the listening URL with the active scheme, for example:

```text
web: https://127.0.0.1:40404
```

## HTTP fallback mode

To force the previous HTTP-only behavior:

```bash
cargo run -- --insecure --port 40404
```

`--insecure` cannot be combined with `--tls-cert` or `--tls-key`.

## Browser compression validation

Open the HTTPS origin and inspect the `PATCH /events` request.
In secure mode, browsers should advertise `br` in `Accept-Encoding` and the server can negotiate Brotli.
