# DOX: crates

## Purpose

Cargo workspace for the Emberwake application: a Leptos 0.8.x SSR+hydrate full-stack
Rust web app on Axum. Three crates form the canonical cargo-leptos workspace shape.

## Ownership

- `app/` — Leptos UI components + server functions + shared domain types. Compiled
  for both `ssr` and `hydrate` features. Crate-type: `cdylib` + `rlib`.
- `frontend/` — Thin WASM hydrate entry point. Crate-type: `cdylib`. Depends on
  `app` with `hydrate` feature.
- `server/` — Axum binary (`emberwake`). Depends on `app` with `ssr` feature.
  Owns all non-UI concerns (db, auth, security middleware, integrations, telemetry).

## Local Contracts

- Workspace deps are pinned in root `Cargo.toml` `[workspace.dependencies]`.
- Feature flags: `ssr` (server-side rendering) and `hydrate` (WASM client hydration).
- `leptos` 0.8.x with `nonce` feature enabled (CSP nonce support).
- `uuid` v7 with `js` feature for WASM randomness.
- `getrandom` 0.3 with `wasm_js` feature + `.cargo/config.toml` cfg flag for WASM.
- cargo-leptos config lives in root `Cargo.toml` `[[workspace.metadata.leptos]]`.
- Binary name: `emberwake` (set via `[[bin]]` in server and `bin-exe-name` in metadata).
- No `unsafe` in application code (Constitution Principle I).
- Security-critical code (auth, CSRF, sessions) lives in `server/`, never in `app/`.

## Work Guidance

- New shared domain types go in `app/src/domain/`.
- New UI components go in `app/src/components/`.
- New server functions go in `app/src/server/`.
- New routes go in `app/src/routes/`.
- Server-side infrastructure (db, auth, middleware) goes in `server/src/`.
- Changes to workspace deps require updating root `Cargo.toml` and `Cargo.lock`.

## Verification

- `cargo build` — workspace compiles
- `cargo clippy --all-targets --all-features -- -D warnings` — no warnings
- `cargo fmt --all --check` — formatting clean
- `cargo leptos build` — both WASM frontend and server binary build

## Child DOX Index

None — `app/`, `frontend/`, and `server/` are individual crate packages, not
durable boundaries with their own contracts.
