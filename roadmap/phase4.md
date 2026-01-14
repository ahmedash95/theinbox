InboxCleanup Roadmap - Phase 4 (Full Inbox + Sync Scheduler)

Goal
- Import and display all emails (not just unread).
- Add Inbox/Unread tabs in the sidebar for quick filtering.
- Add a refresh interval setting and use it to schedule background sync.
- Show sync progress in the titlebar toolbar with a progress bar during import.

Scope
1) Backend data flow (all mail)
- Add a command to list all cached emails (or reuse list with unread_only=false).
- Ensure background sync updates all cached emails (not only unread).
- Keep existing unread-only list if it’s still useful for quick filtering.

2) Sidebar tabs
- Add tabs: Inbox (all cached emails), Unread (is_read = false).
- Switch displayed list based on active tab.
- Keep filter tags visible regardless of enabled filters (already in UI).

3) Refresh interval (settings + scheduling)
- Add a dropdown in Settings > Account for refresh interval:
  - 1, 5, 15, 30, 60 minutes.
- Persist interval in local settings.
- Use interval to trigger background sync (with caching):
  - On interval tick: run sync, keep list visible.
  - On sync completion event: refresh cached list for the active tab.

4) Toolbar import progress
- During background sync, show a progress bar in the titlebar toolbar area.
- Use `imap_sync_progress` stage and counts to show:
  - Indeterminate at start.
  - Completion + brief “Up to date” state.
  - Error state in toolbar + existing banner.

5) Error handling
- Sync errors surface in toolbar and error banner.
- Keep cached data visible on failures.

Acceptance criteria
- Inbox tab shows all cached emails.
- Unread tab filters to unread only.
- Sync imports all emails and updates the cache.
- Refresh interval triggers background sync automatically.
- Toolbar shows a progress indicator during sync.

Notes
- If DB needs a new list command, add `gmail_list_cached_all`.
- If backend fetch is still “unread-only,” we’ll add a new fetch for all.
