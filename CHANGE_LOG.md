# Change Log

## 0.1.0
- Initial release.
- Gmail IMAP integration with app passwords.
- Filter emails by sender/subject with regex support.
- Bulk mark emails as read.
- Background sync with progress indicator.
- Auto-update with download progress and error details.
- Fix updater signature verification by hardcoding the public key.
- Show detailed error information with collapsible details when updates fail.
- Migrate filter IDs to auto-increment integers with a one-time DB remap for existing data.
- Update filter CRUD to refresh only affected filters instead of full backfills.
- Fix Tauri command argument casing to restore filter refresh/count calls.
- Add Rematch Filters menu action and keep filter counts formatted in the sidebar.
- Auto-mark opened emails as read and add a per-email Mark Unread action.
- Render email bodies on a white background for dark-mode readability and open links in the default browser.
- In-app update dialog so Check for Updates always shows feedback.
- Timeout and clearer error messaging when update checks fail.
- Improved sync progress refresh behavior during background sync.
