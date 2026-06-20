# DOX: specs/001-greenfield

## Purpose

Feature specification set for the greenfield full-Rust Emberwake rewrite. Contains the
complete design chain: spec, plan, research, data model, security threat model, server-function
and public-API contracts, task breakdown, and developer quickstart.

## Ownership

- `spec.md` — feature specification (user stories, requirements, success criteria)
- `plan.md` — implementation plan (technical context, constitution check, project structure)
- `research.md` — Phase 0 decisions with rejected alternatives and resolved items
- `data-model.md` — Phase 1 schema (all entities, relationships, migration notes)
- `security.md` — threat model (STRIDE, controls, verification hooks)
- `tasks.md` — Phase 2 task breakdown (84+ tasks across 12 phases, test-first)
- `quickstart.md` — developer and operator workflow
- `contracts/` — typed boundary contracts (see child DOX)

## Local Contracts

- Spec-driven workflow: significant features get a spec and plan before code.
- Constitution compliance: every plan must pass the Constitution Check gate (Principles I–V).
- Slice-based delivery: US1–US3 is MVP; each story is independently testable and deployable.
- Tests first: behavioral changes ship with tests written before implementation.
- Security wiring: every mutating server function enforces auth + CSRF + authorization.
- Performance budgets: changes must not regress SC-001 through SC-008.
- No legacy parity: clean-sheet design; one-way import is a future US9-style addition.

## Work Guidance

- Read the constitution (`.specify/memory/constitution.md`) before editing any spec file.
- Changes to requirements or success criteria require updating `spec.md` first, then
  propagating to `plan.md`, `tasks.md`, and relevant contract files.
- Resolved open items move from "Open Items" to "Resolved Items" in `research.md` with
  verified sources and trust grades.
- New entities require updates to `data-model.md`, `spec.md` Key Entities, `tasks.md`
  migration tasks, and `contracts/server-functions.md` if they expose server functions.
- The Constitution Check in `plan.md` must be re-checked after any design change.

## Verification

- CI `docs` job asserts spec package presence (constitution, spec, plan, tasks).
- Constitution Check gate in `plan.md` must pass before Phase 0 research.
- Success Criteria (SC-001–SC-008) are verified during Polish phase and CI.

## Child DOX Index

- [`contracts/`](contracts/AGENTS.md) — Typed server-function boundary and public REST API surface
