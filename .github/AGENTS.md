# DOX: .github

## Purpose

GitHub automation, CI/CD gates, and contributor templates. The workflows encode the
constitution's hard gates (Principles II, IV, V) as automated checks that block merge or
publish on failure.

## Ownership

- `workflows/ci.yml` — CI pipeline: clippy, fmt, cargo-leptos test, cargo-deny, cargo-audit,
  fuzz smoke; runs on amd64 + arm64 matrix; always-on docs sanity job
- `workflows/release.yml` — release pipeline: multi-arch buildx → GHCR, SBOM (cyclonedx),
  cosign keyless signing; triggered by `v*` tags
- `dependabot.yml` — weekly dependency updates for cargo, github-actions, docker
- `ISSUE_TEMPLATE/` — bug report, feature request, config
- `PULL_REQUEST_TEMPLATE.md` — PR checklist enforcing definition-of-done gates

## Local Contracts

- CI must pass on both `linux/amd64` and `linux/arm64` (Principle IV, SC-008).
- `RUSTFLAGS: "-D warnings"` — clippy warnings are build failures (Principle I).
- `cargo-deny` and `cargo-audit` are hard gates — any advisory or disallowed license blocks
  merge (Principle II, SC-006).
- Fuzz smoke runs on every PR once fuzz targets exist (Principle IV, SC-007).
- Release images must be signed (cosign) and carry an SBOM (Principle V, SC-006).
- No unsigned or advisory-flagged build may publish.
- Workflows are guarded so the design-only repo (no Cargo.toml) still has a green check.

## Work Guidance

- New CI steps must not weaken constitution gates — adding checks is fine, removing or
  relaxing them requires a Complexity Tracking justification in the plan.
- Workflow changes that affect the release pipeline must be tested with a dry-run tag.
- Dependabot PRs are grouped (patch+minor) and labeled; review for advisories before merge.
- The `docs` job's file assertions must be updated if spec files are added or renamed.

## Verification

- CI green on push/PR (all jobs pass on both arches).
- Release pipeline produces a signed image with SBOM attached to the GitHub Release.
- `has-cargo` guard ensures workflows degrade gracefully pre-workspace.

## Child DOX Index

None — `workflows/` and `ISSUE_TEMPLATE/` are tool-config subdirectories, not durable
boundaries with their own contracts.
