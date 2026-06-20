## What & why

<!-- What does this change and why? Link the task ID (e.g. T029) or issue. -->

Closes #

## Type of change

- [ ] Bug fix
- [ ] New feature / user story slice
- [ ] Refactor / internal
- [ ] Docs / spec
- [ ] CI / build / supply chain

## Checklist (Definition of Done)

- [ ] `cargo clippy --all-targets --all-features -- -D warnings` is clean
- [ ] `cargo leptos test` passes (tests added/updated; bug fixes include a regression test)
- [ ] `cargo deny check` and `cargo audit` pass
- [ ] Untrusted-input parsers touched? Fuzz target updated.
- [ ] Mutating server function touched? Enforces **auth + CSRF + authorization**; private rows
      excluded in SQL for unauthorized callers.
- [ ] No `unsafe`, no system OpenSSL, no JS/Python introduced.
- [ ] Performance budgets (spec Success Criteria) not regressed.
- [ ] `CHANGELOG.md` (Unreleased) updated for user-visible changes.
- [ ] Inbound = outbound: I license this contribution under Apache-2.0 and copied no
      license-incompatible code/assets.

## Notes for reviewers
