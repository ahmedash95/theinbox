#!/usr/bin/env bash
set -euo pipefail

ROOT_DIR="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
cd "$ROOT_DIR"

if ! command -v gh >/dev/null 2>&1; then
  echo "GitHub CLI (gh) is required for releases." >&2
  exit 1
fi

if [[ -n "$(git status --porcelain)" ]]; then
  echo "Working tree is not clean. Commit or stash changes first." >&2
  exit 1
fi

VERSION=$(node -p "require('./src-tauri/tauri.conf.json').version")
TAG="v${VERSION}"

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY:-}" ]]; then
  echo "TAURI_SIGNING_PRIVATE_KEY is not set." >&2
  exit 1
fi

if [[ -z "${TAURI_SIGNING_PRIVATE_KEY_PASSWORD:-}" ]]; then
  echo "TAURI_SIGNING_PRIVATE_KEY_PASSWORD is not set." >&2
  exit 1
fi

echo "Building release for ${TAG}..."
pnpm tauri build

BUNDLE_DIR="src-tauri/target/release/bundle"
mapfile -d '' ASSETS < <(
  find "$BUNDLE_DIR" -type f \( \
    -name "*.dmg" -o \
    -name "*.dmg.sig" -o \
    -name "*.app.tar.gz" -o \
    -name "*.tar.gz" -o \
    -name "*.sig" -o \
    -name "latest.json" \
  \) -print0
)

if [[ ${#ASSETS[@]} -eq 0 ]]; then
  echo "No release assets found in ${BUNDLE_DIR}." >&2
  exit 1
fi

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Tag ${TAG} already exists." >&2
  exit 1
fi

git tag -a "$TAG" -m "$TAG"
git push origin "$TAG"

if gh release view "$TAG" >/dev/null 2>&1; then
  gh release upload "$TAG" "${ASSETS[@]}" --clobber
else
  gh release create "$TAG" "${ASSETS[@]}" --title "$TAG" --notes "Release $TAG"
fi

echo "Release ${TAG} published."
