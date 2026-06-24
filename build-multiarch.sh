#!/usr/bin/env bash
# Multi-arch build script for Emberwake (amd64 + arm64) — T077.
# Builds and pushes a multi-arch image to GHCR with version + latest tags.
set -euo pipefail

# --- Configuration -----------------------------------------------------------
REGISTRY="ghcr.io"
IMAGE="${REGISTRY}/draco-lunaris/emberwake"

# Version: from git tag, or "dev" if not on a tag.
VERSION="$(git describe --tags --always --dirty 2>/dev/null || echo dev)"

# --- Pre-flight checks -------------------------------------------------------
if ! docker buildx version >/dev/null 2>&1; then
  echo "ERROR: docker buildx is required but not found." >&2
  exit 1
fi

# Ensure buildx builder exists (creates one if none).
BUILDER="emberwake-builder"
if ! docker buildx inspect "${BUILDER}" >/dev/null 2>&1; then
  docker buildx create --name "${BUILDER}" --use
else
  docker buildx use "${BUILDER}"
fi

# --- Build & push ------------------------------------------------------------
echo "Building ${IMAGE}:${VERSION} and ${IMAGE}:latest for amd64+arm64..."

docker buildx build \
  --platform linux/amd64,linux/arm64 \
  -f .docker/Dockerfile \
  -t "${IMAGE}:${VERSION}" \
  -t "${IMAGE}:latest" \
  --push \
  .

echo "Verifying multi-arch manifest..."
docker buildx imagetools inspect "${IMAGE}:latest" --format '{{range .Manifest.Manifests}}{{.Platform.OS}}/{{.Platform.Architecture}}{{end}}'
if [ $? -ne 0 ]; then
  echo "ERROR: Failed to verify multi-arch manifest"
  exit 1
fi

echo "Multi-arch build complete:"
echo "  ${IMAGE}:${VERSION}"
echo "  ${IMAGE}:latest"
echo "Verify with: docker buildx imagetools inspect ${IMAGE}:${VERSION}"
