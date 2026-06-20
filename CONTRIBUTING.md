# Contributing to Emberwake

Thanks for your interest in Emberwake. This is a spec-driven, security-first, full-Rust
project, and a few principles are **hard gates** rather than suggestions. Reading the
[constitution](./.specify/memory/constitution.md) first will save you time.

## Ground rules

- **Inbound = outbound**: contributions are accepted under the [Apache License 2.0](./LICENSE).
  By submitting a PR you agree your contribution is licensed under it.
- **Clean-room**: do not copy code or assets from Flame, Dragons-Flame, or any
  license-incompatible source. Reimplement functionality from scratch. Any deliberately
  vendored third-party code must be license-compatible and recorded in [`NOTICE`](./NOTICE).
- **No `unsafe`** in application code; no JavaScript runtime or Python in the codebase.

## Spec-driven workflow

This repo uses [GitHub Spec Kit](https://github.com/github/spec-kit). Non-trivial work flows
through the documents under [`specs/`](./specs/):

1. Significant features get a spec (`/specify`) and a plan (`/plan`) before code.
2. Work is tracked as tasks (`/tasks`) — see
   [`specs/001-greenfield/tasks.md`](./specs/001-greenfield/tasks.md).
3. Each task is small, independently testable, and maps to a user story.

For small fixes, a focused PR against an existing task is fine — reference the task ID.

## Definition of done (CI gates)

A change is not done until all of these pass locally and in CI:

```bash
cargo clippy --all-targets --all-features -- -D warnings   # no warnings
cargo leptos test                                          # server-fn + integration tests
cargo deny check                                           # advisories, licenses, bans
cargo audit                                                # RUSTSEC advisories
```

Additionally:

- **Tests first**: behavioral changes ship with tests; bug fixes start with a failing
  regression test. Untrusted-input parsers (import) require fuzz coverage.
- **Security wiring**: every mutating server function must enforce auth + CSRF + authorization
  before review. Reads must exclude private rows in SQL for unauthorized callers.
- **Budgets**: changes must not regress the performance budgets in the spec (Success Criteria).
- **Both arches**: CI runs on `linux/amd64` and `linux/arm64`; a change must pass on both.

## Commits & PRs

- Conventional, present-tense commit messages; reference task IDs (e.g. `T029`) where relevant.
- Keep PRs scoped to a task or a tight logical group; fill out the PR template checklist.
- Update `CHANGELOG.md` (Unreleased section) for user-visible changes.

## Reporting security issues

Do **not** use public issues for vulnerabilities — see [`SECURITY.md`](./SECURITY.md).

## Code of conduct

This project follows the [Contributor Covenant](./CODE_OF_CONDUCT.md).
