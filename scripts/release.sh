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
APP_NAME="TheInbox"
RELEASE_DIR="release-artifacts"
CHANGE_LOG_FILE="${ROOT_DIR}/CHANGE_LOG.md"

if [[ -z "${TAURI_PRIVATE_KEY:-}" && -z "${TAURI_PRIVATE_KEY_PATH:-}" ]]; then
  echo "TAURI_PRIVATE_KEY or TAURI_PRIVATE_KEY_PATH is required for updater signing." >&2
  exit 1
fi

if [[ -z "${CODESIGN_IDENTITY:-}" && -z "${CODESIGN_IDENTITY_HASH:-}" ]]; then
  echo "CODESIGN_IDENTITY is required for notarization (Developer ID Application certificate)." >&2
  exit 1
fi

if [[ ! -f "$CHANGE_LOG_FILE" || -z "$(cat "$CHANGE_LOG_FILE" | tr -d '\n' | tr -d '\r')" ]]; then
  echo "CHANGE_LOG.md is required and cannot be empty." >&2
  exit 1
fi

RELEASE_NOTES=$(node -e "const fs=require('fs'); const s=fs.readFileSync('$CHANGE_LOG_FILE','utf8'); const match = s.match(/^##\\s+[^\\n]+\\n([\\s\\S]*?)(?=^##\\s+|\\s*$)/m); if (!match) { process.exit(1); } const body = match[1].trim(); console.log(body);")
if [[ -z "$(echo "$RELEASE_NOTES" | tr -d '\n' | tr -d '\r')" ]]; then
  echo "Latest entry in CHANGE_LOG.md is empty." >&2
  exit 1
fi
RELEASE_NOTES_JSON=$(node -e "const fs=require('fs'); const s=fs.readFileSync('$CHANGE_LOG_FILE','utf8'); const match = s.match(/^##\\s+[^\\n]+\\n([\\s\\S]*?)(?=^##\\s+|\\s*$)/m); if (!match) { process.exit(1); } const body = match[1].trim(); console.log(JSON.stringify(body));")

APP_BUNDLE_DIR="src-tauri/target/release/bundle/macos"
APP_BUNDLE_PATH="${APP_BUNDLE_DIR}/${APP_NAME}.app"
DMG_PATH="${RELEASE_DIR}/${APP_NAME}_${VERSION}_aarch64.dmg"
ZIP_PATH="${RELEASE_DIR}/${APP_NAME}.zip"
TAR_PATH="${APP_BUNDLE_DIR}/${APP_NAME}.app.tar.gz"
SIG_PATH="${APP_BUNDLE_DIR}/${APP_NAME}.app.tar.gz.sig"

if [[ -d "$APP_BUNDLE_PATH" && -f "$DMG_PATH" ]]; then
  echo "Existing build detected for ${TAG}; skipping build."
else
  echo "Building release for ${TAG}..."
  pnpm tauri build --bundles app
fi

if [[ ! -d "$APP_BUNDLE_PATH" ]]; then
  echo "Expected app bundle not found at ${APP_BUNDLE_PATH}." >&2
  exit 1
fi

mkdir -p "$RELEASE_DIR" "$RELEASE_DIR/dmg-root"
rm -rf "$RELEASE_DIR/dmg-root/${APP_NAME}.app"

CODESIGN_ID="${CODESIGN_IDENTITY_HASH:-${CODESIGN_IDENTITY:-}}"
echo "Re-signing app with ${CODESIGN_ID}..."
codesign --force --deep --options runtime --timestamp --sign "$CODESIGN_ID" "$APP_BUNDLE_PATH"

echo "Creating notarization ZIP..."
ditto -c -k --keepParent "$APP_BUNDLE_PATH" "$ZIP_PATH"

if ! command -v xcrun >/dev/null 2>&1; then
  echo "xcrun is required for notarization." >&2
  exit 1
fi
if [[ -n "${NOTARY_PROFILE:-}" ]]; then
  echo "Notarizing with keychain profile ${NOTARY_PROFILE}..."
  xcrun notarytool submit "$ZIP_PATH" --keychain-profile "$NOTARY_PROFILE" --wait
elif [[ -n "${APPLE_ID:-}" && -n "${APPLE_TEAM_ID:-}" && -n "${APPLE_PASSWORD:-}" ]]; then
  echo "Notarizing with Apple ID credentials..."
  xcrun notarytool submit "$ZIP_PATH" --apple-id "$APPLE_ID" --team-id "$APPLE_TEAM_ID" --password "$APPLE_PASSWORD" --wait
else
  echo "Notarization requires NOTARY_PROFILE or APPLE_ID/APPLE_TEAM_ID/APPLE_PASSWORD." >&2
  exit 1
fi

echo "Stapling app..."
xcrun stapler staple "$APP_BUNDLE_PATH"

echo "Packaging DMG/ZIP from stapled app..."
ditto "$APP_BUNDLE_PATH" "$RELEASE_DIR/dmg-root/${APP_NAME}.app"
ditto -c -k --keepParent "$APP_BUNDLE_PATH" "$ZIP_PATH"

hdiutil create -volname "$APP_NAME" \
  -srcfolder "$RELEASE_DIR/dmg-root" \
  -ov -format UDZO \
  "$DMG_PATH"

echo "Rebuilding updater tarball..."
COPYFILE_DISABLE=1 tar -czf \
  "$TAR_PATH" \
  -C "$APP_BUNDLE_DIR" "${APP_NAME}.app"

pnpm tauri signer sign "$TAR_PATH"

PUB_DATE=$(date -u +%Y-%m-%dT%H:%M:%SZ)
SIGNATURE=$(cat "$SIG_PATH")

cat <<EOF > "${RELEASE_DIR}/latest.json"
{
  "version": "${VERSION}",
  "notes": ${RELEASE_NOTES_JSON},
  "pub_date": "${PUB_DATE}",
  "platforms": {
    "darwin-aarch64": {
      "url": "https://github.com/ahmedash95/theinbox/releases/download/${TAG}/${APP_NAME}.app.tar.gz",
      "signature": "${SIGNATURE}"
    }
  }
}
EOF

ASSETS=(
  "$ZIP_PATH"
  "${DMG_PATH}"
  "$TAR_PATH"
  "$SIG_PATH"
  "${RELEASE_DIR}/latest.json"
)

if git rev-parse "$TAG" >/dev/null 2>&1; then
  echo "Tag ${TAG} already exists." >&2
  exit 1
fi

git tag -a "$TAG" -m "$TAG" -m "$RELEASE_NOTES"
git push origin "$TAG"

if gh release view "$TAG" >/dev/null 2>&1; then
  gh release upload "$TAG" "${ASSETS[@]}" --clobber
else
  gh release create "$TAG" "${ASSETS[@]}" --title "$TAG" --notes "$RELEASE_NOTES"
fi

echo "Release ${TAG} published."
