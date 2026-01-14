InboxCleanup Roadmap - Phase 3 (SQLite Local Cache)

Goal
- Make SQLite the source of truth for listing emails, filters, and filter results.
- Keep IMAP fetches running in the background and surface progress in the UI.
- Introduce a storage/repository interface so the data backend can be swapped later.

Scope
- Emails table: all received emails + read/unread flag.
- Filters table: persisted filter list.
- Filtered_emails table: many-to-many mapping (one email can match many filters).
- Move filter persistence from JSON to SQLite.
- Cache email headers + minimal body metadata; fetch full body on demand.

Plan
1) Data model + schema
- Define SQLite schema and migrations.
- Tables:
  - emails: id (pk), uid, message_id, subject, sender, date, is_read, snippet?, has_body?, created_at, updated_at.
  - filters: id (pk), name, pattern, field, is_regex, enabled, created_at, updated_at.
  - filtered_emails: email_id, filter_id, matched_at, primary key (email_id, filter_id).
- Indexes:
  - emails.uid, emails.message_id, emails.is_read, emails.date.
  - filtered_emails.filter_id and filtered_emails.email_id.

2) Storage abstraction
- Introduce a `Storage` trait (or repository interface) in `src-tauri/src/storage`.
- Provide `SqliteStorage` implementation (rusqlite or sqlx).
- Expose operations needed by UI + sync:
  - list_emails(filters?, search?, unread_only?).
  - upsert_email_headers(emails).
  - mark_emails_read(uids or ids).
  - get_filters / save_filters.
  - upsert_filtered_emails(mappings).
- Wire the tauri commands to call the storage trait instead of filesystem JSON.

3) SQLite lifecycle
- Pick a DB location under app config dir (same root as current filters.json).
- Create migrations on startup; maintain schema versioning.
- Add a light DB health check on startup (open + pragma + migrate).

4) IMAP sync + cache strategy
- Background fetch job: run IMAP fetch on app start and on manual refresh.
- Fetch headers for unread + recent window (define window size), upsert into SQLite.
- Track read/unread via IMAP flags, update SQLite `is_read`.
- Compute filter matches after insert/update and write to `filtered_emails`.
- Emit progress events via Tauri (e.g. total + processed).

5) UI updates
- Sidebar list and main email list read from SQLite via tauri command.
- Add a bottom-of-sidebar progress indicator driven by events.
- Display cached results immediately; IMAP fetch updates the cache as it completes.

6) Migration for existing users
- On first run, read existing `filters.json`, insert into `filters` table, then delete or ignore JSON.

7) Testing + validation
- Unit tests for storage queries (filters CRUD, email upsert, mappings).
- Smoke test: open app, cached emails render, background IMAP updates, progress UI visible.

Deliverables
- New `roadmap/phase3.md` (this plan).
- SQLite schema + migrations.
- Storage trait + Sqlite implementation.
- Tauri command updates to use storage.
- Background IMAP sync + progress events.
- UI progress bar in sidebar.

