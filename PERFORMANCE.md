# Performance Validation — Emberwake

**Phase**: 12 (Polish) · **Task**: T080 · **Date**: 2026-06-20

Maps to success criteria SC-001 through SC-004.

## Methodology

All metrics are measurable via `benches/seed_benchmark.sh` against a running
Emberwake server. SC-004 (WASM bundle) is verified at build time; the rest
require a live server instance.

## SC-001: SSR First-Paint TTFB < 50ms, Interactive < 1s

**Measurement**: `benches/seed_benchmark.sh` Step 3 measures TTFB via
`curl -w '%{time_starttransfer}'` (20 samples, reports avg + p50 + p95).
Step 4 measures total response time (interactive proxy latency).

**How to run**:
```bash
# Start server
DATA_DIR=./data cargo leptos watch &

# Run benchmark
./benches/seed_benchmark.sh
```

**Budget**: TTFB < 50ms, Interactive < 1s

## SC-002: CRUD Server-Function p95 < 25ms

**Measurement**: `benches/seed_benchmark.sh` Step 5 measures CRUD create
latency over 50 POST requests to `/api/v1/services`, reports avg + p95.

**Budget**: p95 < 25ms

## SC-003: Idle RSS ≤ 48MB, Cold Start to readyz < 1.5s

**Measurement**: `benches/seed_benchmark.sh` Step 6 measures idle RSS via
`/proc/<pid>/status` (VmRSS). Cold start is measured when `COLD_START=1` is set:
the script starts the binary, polls `/readyz`, and reports elapsed time
(5 samples, avg).

**How to measure cold start**:
```bash
# Stop any running server, then:
COLD_START=1 ./benches/seed_benchmark.sh
```

**Budget**: Idle RSS ≤ 48MB, Cold start < 1.5s

## SC-004: WASM Hydration Bundle < 350KB Compressed

**Status**: ✅ Verified

**Measurement**: `benches/seed_benchmark.sh` Step 7 measures WASM + JS + CSS
file sizes (raw + gzip) from `target/site/pkg/`.

Measured from `target/site/pkg/` (cargo-leptos build output):

| Asset | Raw Size | Gzip Compressed |
|-------|----------|-----------------|
| WASM  | 355,202 bytes (347 KB) | 86,367 bytes (84 KB) |
| JS loader | 21,452 bytes (21 KB) | 5,032 bytes (5 KB) |
| CSS | 1 byte | 58 bytes |
| **Total** | **376,655 bytes (368 KB)** | **90,778 bytes (89 KB)** |

**Verdict**: 89 KB gzip compressed — well within the 350 KB budget (25% of budget used).

## SC-005/006/007: Security & CI

These criteria are verified by existing tests and CI gates:
- SC-005: CSRF + rate limiting — verified by integration tests
- SC-006: Auth (password, OIDC, WebAuthn) — verified by integration tests
- SC-007: Supply chain (cargo-deny, cargo-audit, cosign) — verified by CI/release workflow

## Benchmark Script

`benches/seed_benchmark.sh` — seeds 200 services + 500 bookmarks, then measures:
1. SSR TTFB (20 samples, avg + p50 + p95) — SC-001
2. Total response time (20 samples, avg) — SC-001 interactive
3. CRUD create latency (50 samples, avg + p95) — SC-002
4. Idle RSS via /proc — SC-003
5. Cold start to /readyz (5 samples, avg) — SC-003 (set COLD_START=1)
6. WASM bundle size (raw + gzip) — SC-004

Usage:
```bash
# Requires a running Emberwake server on localhost:5005
DATA_DIR=./data cargo leptos watch &
./benches/seed_benchmark.sh

# To also measure cold start (stops/starts server automatically):
COLD_START=1 ./benches/seed_benchmark.sh
```

## Summary

| Criterion | Budget | Measured | Status |
|-----------|--------|----------|--------|
| SC-001 TTFB | < 50ms | via script | Measurable |
| SC-001 Interactive | < 1s | via script | Measurable |
| SC-002 CRUD p95 | < 25ms | via script | Measurable |
| SC-003 Idle RSS | ≤ 48MB | via script | Measurable |
| SC-003 Cold start | < 1.5s | via script (COLD_START=1) | Measurable |
| SC-004 WASM bundle | < 350KB | 89 KB gzip | ✅ Verified |
| SC-005 CSRF/rate | — | integration tests | ✅ Verified |
| SC-006 Auth | — | integration tests | ✅ Verified |
| SC-007 Supply chain | — | CI gates | ✅ Verified |

## CI Integration

The benchmark script should be run in CI after a successful build to validate SC-001
through SC-003. The WASM bundle size check (SC-004) can be automated in CI by:

```bash
# After cargo leptos build --release
WASM_SIZE=$(gzip -c target/site/pkg/*.wasm | wc -c)
JS_SIZE=$(gzip -c target/site/pkg/*.js | wc -c)
CSS_SIZE=$(gzip -c target/site/pkg/*.css | wc -c)
TOTAL=$((WASM_SIZE + JS_SIZE + CSS_SIZE))
if [ $TOTAL -gt 358400 ]; then  # 350 * 1024
  echo "FAIL: WASM bundle exceeds 350KB budget"
  exit 1
fi
```
