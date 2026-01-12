# Publishing TheInbox (macOS)

This guide walks you through the exact keys and environment variables required
to run `make publish` successfully, including signing, notarization, and
updater artifacts.

## One-time setup

### 1) Developer ID Application certificate

- Install your **Developer ID Application** certificate in Keychain.
- Note your Team ID from https://developer.apple.com/account → Membership.

To find the signing identities and SHA-1 hashes:

```bash
security find-identity -v -p codesigning
```

Pick the SHA-1 hash for your **Developer ID Application** certificate.

### 2) Apple notarization credentials

Create an **app-specific password** at https://appleid.apple.com.

Store notarization credentials in the keychain (recommended):

```bash
xcrun notarytool store-credentials theinbox-notary \
  --apple-id "you@apple.com" \
  --team-id "TEAMID" \
  --password "app-specific-password"
```

### 3) Tauri updater keys

Generate a Tauri updater signing key (once):

```bash
pnpm tauri signer generate -w ~/.tauri/theinbox.key
```

Save the printed **public key** and set it in `src-tauri/tauri.conf.json`:

```json
"pubkey": "<printed-public-key>"
```

If you want an **empty password**, hit Enter when prompted during key
generation. If you set a password, you must use the same one later.

## Required environment variables

### A) Tauri updater signing

```bash
export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/theinbox.key)"
export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=""
```

If your key uses a non-empty password, set it instead of `""`.

### B) Code signing (Developer ID)

Use the SHA-1 hash to avoid ambiguous certificate names:

```bash
export CODESIGN_IDENTITY_HASH="YOUR_SHA1_HASH"
```

### C) Notarization (choose one)

Option 1: Keychain profile (recommended)

```bash
export NOTARY_PROFILE="theinbox-notary"
```

Option 2: Direct Apple ID credentials

```bash
export APPLE_ID="you@apple.com"
export APPLE_TEAM_ID="TEAMID"
export APPLE_PASSWORD="app-specific-password"
```

## Publish

```bash
make publish
```

## Common failures and fixes

- **Wrong password for that key**
  - Your updater key has a password. Set it correctly or regenerate a new key
    without a password.
- **Signature invalid / no hardened runtime / no timestamp**
  - Ensure `CODESIGN_IDENTITY_HASH` is set to your Developer ID Application
    certificate SHA-1.
- **Ambiguous signing identity**
  - Use the SHA-1 hash via `CODESIGN_IDENTITY_HASH` instead of the certificate
    display name.
- **Notarization status: Invalid**
  - Fetch the log and inspect:
    ```bash
    xcrun notarytool log <JOB_ID> --keychain-profile theinbox-notary
    ```
    Fix the reported error, then re-run `make publish`.
