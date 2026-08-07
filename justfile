# Emberwake task runner — single source of truth for local + CI.
# Local:   just check           (dev loop; warm cache on the dev box)
# Release: just release patch    (bump -> commit -> tag -> push; CI builds + publishes the Docker image)

default:
    @just --list

# --- quality gates (the dev loop; `just check` runs all) ---
fmt:
    cargo fmt --all --check

clippy:
    cargo clippy --all-targets --all-features -- -D warnings

test:
    cargo leptos test

deny:
    cargo deny check

# Fuzz the import parsers (short smoke; needs nightly + cargo-fuzz)
fuzz-smoke:
    #!/usr/bin/env bash
    set -euo pipefail
    for t in import_html import_json import_opml; do
      if [ -f "fuzz/fuzz_targets/${t}.rs" ]; then
        cargo +nightly fuzz run "$t" -- -max_total_time=60 -runs=100000
      fi
    done

check: fmt clippy test deny fuzz-smoke
    @echo "all gates passed"

# --- build ---
build:
    cargo leptos build --release

# --- release (CI builds + publishes the Docker image from the tag) ---
release KIND:
    bash scripts/release.sh "{{KIND}}"
