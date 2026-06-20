#!/usr/bin/env bash
# Multi-arch build script for Emberwake (amd64 + arm64)
# Placeholder — finalized in Phase 12 (T077).
set -euo pipefail

docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f .docker/Dockerfile \
  -t ghcr.io/draco-lunaris/emberwake:latest \
  .
