# Security Policy

Security is a first-class, non-negotiable principle of Emberwake (see the
[constitution](./.specify/memory/constitution.md), Principle II) and the full threat model
lives in [`specs/001-greenfield/security.md`](./specs/001-greenfield/security.md).

## Supported versions

Until the first stable release, only the `main` branch is supported. After `1.0.0`, the latest
minor release line receives security fixes.

## Reporting a vulnerability

**Please do not open a public issue for security vulnerabilities.**

Report privately via GitHub's **"Report a vulnerability"** flow (Security → Advisories) on this
repository, which opens a private advisory. Include:

- a description of the issue and its impact,
- steps to reproduce or a proof of concept,
- affected version/commit, and any suggested remediation.

You can expect an acknowledgement within a few days. We will work with you on a fix and a
coordinated disclosure timeline, and will credit you in the advisory unless you prefer to
remain anonymous.

## Scope

In scope: authentication/session handling, CSRF, access control and visibility enforcement,
injection, the import parsers, the Docker/Kubernetes integrations, secret handling, the
container/runtime hardening, and the supply-chain pipeline.

Out of scope (v1, by design — see the threat model): findings that require an already-
compromised host or operator account; absence of a built-in WAF; and issues only reachable
when the operator has explicitly disabled a documented security default.

## Hardening expectations for operators

- Terminate TLS at a trusted proxy or enable the built-in ACME HTTPS; never serve Emberwake
  over plaintext on an untrusted network.
- Run the published, **signed** image; verify the signature and review the attached SBOM.
- Mount the Docker socket read-only and only if you use Docker discovery.
- Keep secrets in files via the `*_FILE` convention, not baked into images or compose files.
