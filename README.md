# TheInbox

TheInbox is a focused, open source macOS app for cleaning up Gmail inboxes. It connects via Gmail IMAP (using an app password), lets you define filter patterns, and helps you bulk mark low-priority mail as read.

## Current features

- Gmail IMAP integration via app password
- Unread and inbox views with counts
- Filter rules using plain text or regex (Subject, Sender, or Any)
- Bulk mark-as-read for matched messages
- Local cache for faster reloads
- Background sync with progress status
- Built-in updater using Tauri + GitHub Releases

## Requirements

- macOS
- Node.js + pnpm
- Rust toolchain

## Development

Install dependencies:

```bash
pnpm install
```

Run the app in dev:

```bash
pnpm tauri dev
```

## Usage

1) Open Settings and add your Gmail address and app password.
2) Create filters that match the mail you want to triage.
3) Review the filtered list and bulk mark as read.

## Auto-updates (Tauri updater)

The updater manifest is served from:
`https://github.com/ahmedash95/theinbox/releases/latest/download/latest.json`

### Setup

1) Generate a signing key once:
`pnpm tauri signer generate -w ~/.tauri/inboxcleanup.key`
2) Export env vars:
`export TAURI_SIGNING_PRIVATE_KEY="$(cat ~/.tauri/inboxcleanup.key)"`
`export TAURI_SIGNING_PRIVATE_KEY_PASSWORD="<password>"`
`export TAURI_SIGNING_PUBLIC_KEY="<public-key-from-generate>"`

### Release

`make publish` will:
- build a signed release
- create a git tag from `src-tauri/tauri.conf.json` version
- create/update a GitHub Release with the `.dmg`, updater artifacts, and `latest.json`

Release notes are pulled from `CHANGE_LOG.md` (latest entry at the top).
