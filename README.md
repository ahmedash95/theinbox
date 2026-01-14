# InboxCleanup

This is a simple macos app built on top of Rust. It uses Gmail IMAP with an App Password to read unread emails and mark them as read in bulk.

## Auto-updates (Tauri updater)

This app uses the Tauri updater plugin and GitHub Releases. The updater manifest is served from:
`https://github.com/ahmedash95/theinbox/releases/latest/download/latest.json`

### Setup

1) Generate a signing key once:
`pnpm tauri signer generate -w ~/.tauri/inboxcleanup.key`
2) Export env vars (used by build and updater):
`export TAURI_SIGNING_PRIVATE_KEY=\"$(cat ~/.tauri/inboxcleanup.key)\"`
`export TAURI_SIGNING_PRIVATE_KEY_PASSWORD=\"<password>\"`
`export TAURI_SIGNING_PUBLIC_KEY=\"<public-key-from-generate>\"`

### Release

`make publish` will:
- build a signed release
- create a git tag from `src-tauri/tauri.conf.json` version
- create/update a GitHub Release with the `.dmg`, updater artifacts, and `latest.json`


### The Problem

In my work email, there are several recurring emails that I'm not interested in it. few emails about marketing campaigns we are running. or few notification emails from facebook. such emails are just taking time from my day just to mark them as read. while I could spend this time on emails that matter the most for me

### Solution

What I have in mind is to have this app list unread emails and filter them out based on a few regex patterns in Subject, From, or even part of the content. After I define this list, the main window of the app shows the filtered unread emails so I can quickly take a look at the subject list and mark them all as read or adjust my patterns until I clean it up.
