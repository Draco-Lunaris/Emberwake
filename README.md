<div align="center">

# Emberwake

**A fast, secure, self-hosted startpage — written end-to-end in Rust.**

[![License: Apache-2.0](https://img.shields.io/badge/License-Apache_2.0-blue.svg)](./LICENSE)
[![Built with Rust](https://img.shields.io/badge/built_with-Rust_1.95-orange.svg)](https://www.rust-lang.org/)
[![Frontend: Leptos](https://img.shields.io/badge/frontend-Leptos_0.8-9b59b6.svg)](https://leptos.dev/)
[![Runtime: ubuntu 26.04](https://img.shields.io/badge/runtime-ubuntu_26.04-e95420.svg)](https://ubuntu.com/)
[![Status: implemented](https://img.shields.io/badge/status-implemented-brightgreen.svg)](./CHANGELOG.md)

</div>

Emberwake is your services' home base: a single-container web app that renders your
applications and bookmarks as an instant, searchable dashboard, with built-in editors,
multi-user accounts, live service-status and weather widgets, optional Docker/Kubernetes
auto-discovery, and a modern theme builder. The whole stack — UI included — is Rust.

> **Status**: all nine user stories (US1–US9) are implemented across Phases 1–11.
> Phase 12 (Polish) finalizes CI/CD, performance validation, security verification, E2E
> tests, and documentation. See [`CHANGELOG.md`](./CHANGELOG.md) for the full feature list.

## Why Emberwake

- **End-to-end Rust** — [Leptos](https://leptos.dev/) (server-side rendering + island
  hydration) with typed server functions as the client/server boundary. No JavaScript runtime
  and no Python anywhere in the codebase or the image.
- **Security-first, by design** — multi-user Argon2id auth, server-side *revocable* sessions,
  CSRF protection, a strict Content-Security-Policy, rate limiting, optional OIDC SSO and
  WebAuthn passkeys, read-only Docker/Kubernetes access, an append-only audit log, and a
  hardened supply chain (cargo-deny/audit, SBOM, signed images). Full threat model in
  [`security.md`](./specs/001-greenfield/security.md).
- **Performance as a tested feature** — fine-grained reactive SSR (no virtual DOM), SQLite
  (WAL) with compile-time-checked SQLx, fully async. Budgets: ≤48 MB idle, <50 ms first-byte,
  <350 KB hydration bundle, <1.5 s cold start.
- **Modern capabilities** — live service-status monitoring and weather over SSE, live
  Docker-event discovery, a design-token theme builder, JSON/HTML/OPML import, and
  first-class observability (tracing + OpenTelemetry + Prometheus).

## Quick start (once implemented)

```bash
docker run -p 5005:5005 \
  -v emberwake-data:/var/lib/emberwake \
  -e DATA_DIR=/var/lib/emberwake \
  --user 10001:10001 --read-only --tmpfs /tmp \
  ghcr.io/draco-lunaris/emberwake:latest
```

Open `http://localhost:5005`, complete the first-run admin setup, and start adding services.
See [`specs/001-greenfield/quickstart.md`](./specs/001-greenfield/quickstart.md) for the full
development, build, and deployment workflow (including optional built-in HTTPS via ACME).

## Design documents

This is a spec-driven project. The design lives under [`specs/001-greenfield/`](./specs/001-greenfield/):

| Document | What it covers |
|----------|----------------|
| [`.specify/memory/constitution.md`](./.specify/memory/constitution.md) | The five governing principles (full-Rust, security, performance, testing, reproducible artifact) |
| [`spec.md`](./specs/001-greenfield/spec.md) | Prioritized user stories, functional requirements, success criteria |
| [`plan.md`](./specs/001-greenfield/plan.md) | Technical context, project structure, constitution check |
| [`research.md`](./specs/001-greenfield/research.md) | Technology decisions and rejected alternatives |
| [`data-model.md`](./specs/001-greenfield/data-model.md) | The SQLite schema |
| [`security.md`](./specs/001-greenfield/security.md) | STRIDE threat model and controls |
| [`contracts/`](./specs/001-greenfield/contracts/) | Typed server-function boundary + public REST/SSE/OIDC surface |
| [`tasks.md`](./specs/001-greenfield/tasks.md) | The phased, story-grouped implementation backlog |

## Tech stack

Leptos 0.8 (SSR + hydrate) on Axum/Tokio · SQLite (WAL) via SQLx · tower-sessions + Argon2id ·
optional OIDC (`openidconnect`) / passkeys (`webauthn-rs`) · rustls (no system OpenSSL) ·
bollard + kube-rs (read-only) · tracing + OpenTelemetry + Prometheus · built with `cargo-leptos`,
shipped as a signed multi-arch (`amd64`/`arm64`) `ubuntu:26.04` image.

## Contributing

Contributions are welcome under Apache-2.0. Please read
[`CONTRIBUTING.md`](./CONTRIBUTING.md) and the
[constitution](./.specify/memory/constitution.md) first — the security and testing principles
are hard gates. For vulnerabilities, see [`SECURITY.md`](./SECURITY.md).

## License & attribution

Emberwake is licensed under the [Apache License 2.0](./LICENSE).

It is an **independent, clean-room project**. It was *inspired by* the self-hosted startpage
idea popularized by [Flame](https://github.com/pawelmalak/flame) (by Paweł Malak) and its fork
Dragons-Flame, but it shares **no source code or assets** with either and is not a derivative
work of them. That inspiration is acknowledged here with thanks; see [`NOTICE`](./NOTICE).
