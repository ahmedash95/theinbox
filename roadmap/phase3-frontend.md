InboxCleanup Roadmap - Phase 3 Frontend (SQLite-backed UI)

Goal
- Read email lists and filters from SQLite via Tauri commands.
- Keep UI responsive by showing cached data immediately and syncing IMAP in the background.
- Add a sidebar progress indicator for background sync.

Frontend Changes Needed
1) Data flow changes
- Replace direct `gmail_fetch_unread` usage for initial list with cached list:
  - On mount: `gmail_list_cached_unread` -> populate `allEmails`.
  - Then kick off `gmail_sync_unread_background` to refresh cache.
  - After sync completes, re-fetch `gmail_list_cached_unread` to refresh UI.

2) Refresh behavior
- `refreshEmails` should:
  - Start background sync via `gmail_sync_unread_background`.
  - Keep existing list visible; do not block UI.
  - On completion event, re-load cached list.
- `forceRefresh` can be removed or repointed to the same background sync behavior.

3) Progress UI (bottom of sidebar)
- Listen for `imap_sync_progress` events from Tauri.
- Track state: idle | syncing | error.
- Display a compact progress bar or spinner + label in the sidebar footer.
- Suggested states:
  - start: show “Syncing…” with indeterminate bar.
  - complete: briefly show “Up to date” then fade.
  - error: show “Sync failed” + tooltip or banner.

4) Filter counts + filtered view
- Keep client-side matching for now, or update later to consume `filtered_emails`.
- If moving to DB-driven filtering later:
  - Add a command to load `filtered_emails` by filter id.
  - Use mappings instead of recomputing regex matches in the UI.

5) Mark as read
- After `gmail_mark_as_read`, re-fetch cached list to stay in sync with SQLite.
- Avoid directly splicing UI list unless you also update from cache.

6) Error handling
- Surface background sync errors from `imap_sync_progress` in the existing error banner.
- Keep cached data visible even if sync fails.

Backend APIs expected by frontend
- `gmail_list_cached_unread(email) -> StoredEmail[]`
- `gmail_sync_unread_background(email) -> void`
- Event: `imap_sync_progress` (payload: stage, processed, total, message?)

