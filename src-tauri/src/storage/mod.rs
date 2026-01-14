use crate::filters::{FilterField, FilterPattern};
use crate::gmail::GmailEmail;
use rusqlite::{params, Connection, OptionalExtension, ToSql};
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Storage interface so we can swap implementations later.
pub trait Storage: Send + Sync {
    fn list_emails(&self, account: &str, unread_only: bool) -> Result<Vec<StoredEmail>, String>;
    fn upsert_emails(
        &self,
        account: &str,
        mailbox: &str,
        emails: &[GmailEmail],
        is_read: bool,
    ) -> Result<(), String>;
    fn mark_emails_read(&self, account: &str, uids: &[u32]) -> Result<usize, String>;
    fn get_filters(&self) -> Result<Vec<FilterPattern>, String>;
    fn save_filters(&self, patterns: &[FilterPattern]) -> Result<(), String>;
    fn set_email_filters(
        &self,
        account: &str,
        uid: u32,
        filter_ids: &[String],
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredEmail {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub date: String,
    pub mailbox: String,
    pub account: String,
    pub is_read: bool,
}

pub struct SqliteStorage {
    conn: Mutex<Connection>,
}

impl SqliteStorage {
    pub fn new() -> Result<Self, String> {
        let path = get_db_path()?;
        let mut conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {}", e))?;
        conn.pragma_update(None, "foreign_keys", &"ON")
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        migrate(&conn)?;
        maybe_import_filters(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }

    #[cfg(test)]
    pub fn new_with_path(path: PathBuf) -> Result<Self, String> {
        let mut conn = Connection::open(path).map_err(|e| format!("Failed to open DB: {}", e))?;
        conn.pragma_update(None, "foreign_keys", &"ON")
            .map_err(|e| format!("Failed to enable foreign keys: {}", e))?;
        migrate(&conn)?;
        maybe_import_filters(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl Storage for SqliteStorage {
    fn list_emails(&self, account: &str, unread_only: bool) -> Result<Vec<StoredEmail>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let mut stmt = if unread_only {
            conn.prepare(
                "SELECT uid, message_id, subject, sender, date, mailbox, account, is_read \
                 FROM emails \
                 WHERE account = ?1 AND is_read = 0 \
                 ORDER BY id DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?
        } else {
            conn.prepare(
                "SELECT uid, message_id, subject, sender, date, mailbox, account, is_read \
                 FROM emails \
                 WHERE account = ?1 \
                 ORDER BY id DESC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?
        };

        let rows = stmt
            .query_map(params![account], |row| {
                Ok(StoredEmail {
                    uid: row.get(0)?,
                    message_id: row.get(1)?,
                    subject: row.get(2)?,
                    sender: row.get(3)?,
                    date: row.get(4)?,
                    mailbox: row.get(5)?,
                    account: row.get(6)?,
                    is_read: row.get::<_, i64>(7)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query emails: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email: {}", e))?);
        }
        Ok(results)
    }

    fn upsert_emails(
        &self,
        account: &str,
        mailbox: &str,
        emails: &[GmailEmail],
        is_read: bool,
    ) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO emails \
                        (uid, message_id, subject, sender, date, mailbox, account, is_read) \
                 VALUES \
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8) \
                 ON CONFLICT(account, uid) DO UPDATE SET \
                    message_id = excluded.message_id,\
                        subject = excluded.subject,\
                        sender = excluded.sender,\
                        date = excluded.date,\
                        mailbox = excluded.mailbox,\
                        account = excluded.account,\
                        is_read = excluded.is_read,\
                        updated_at = CURRENT_TIMESTAMP",
                )
                .map_err(|e| format!("Failed to prepare upsert: {}", e))?;

            for email in emails {
                stmt.execute(params![
                    email.uid,
                    email.message_id,
                    email.subject,
                    email.sender,
                    email.date,
                    mailbox,
                    account,
                    if is_read { 1 } else { 0 }
                ])
                .map_err(|e| format!("Failed to upsert email: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        Ok(())
    }

    fn mark_emails_read(&self, account: &str, uids: &[u32]) -> Result<usize, String> {
        if uids.is_empty() {
            return Ok(0);
        }

        let mut conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        let mut total = 0;
        for chunk in uids.chunks(200) {
            let placeholders = chunk
                .iter()
                .enumerate()
                .map(|(i, _)| format!("?{}", i + 2))
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!(
                "UPDATE emails SET is_read = 1, updated_at = CURRENT_TIMESTAMP \
                 WHERE account = ?1 AND uid IN ({})",
                placeholders
            );

            let mut params_vec: Vec<&dyn ToSql> = Vec::with_capacity(chunk.len() + 1);
            params_vec.push(&account);
            for uid in chunk {
                params_vec.push(uid);
            }

            let updated = tx
                .execute(&sql, params_vec.as_slice())
                .map_err(|e| format!("Failed to mark read: {}", e))?;
            total += updated;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        Ok(total)
    }

    fn get_filters(&self) -> Result<Vec<FilterPattern>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let mut stmt = conn
            .prepare(
                "SELECT id, name, pattern, field, is_regex, enabled \
                 FROM filters ORDER BY rowid ASC",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?;

        let rows = stmt
            .query_map([], |row| {
                let field: String = row.get(3)?;
                Ok(FilterPattern {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    pattern: row.get(2)?,
                    field: parse_filter_field(&field)?,
                    is_regex: row.get::<_, i64>(4)? != 0,
                    enabled: row.get::<_, i64>(5)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query filters: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read filter: {}", e))?);
        }
        Ok(results)
    }

    fn save_filters(&self, patterns: &[FilterPattern]) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        tx.execute("DELETE FROM filters", [])
            .map_err(|e| format!("Failed to clear filters: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT INTO filters \
                        (id, name, pattern, field, is_regex, enabled) \
                     VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                )
                .map_err(|e| format!("Failed to prepare filter insert: {}", e))?;

            for filter in patterns {
                stmt.execute(params![
                    filter.id,
                    filter.name,
                    filter.pattern,
                    filter_field_to_string(&filter.field),
                    if filter.is_regex { 1 } else { 0 },
                    if filter.enabled { 1 } else { 0 }
                ])
                .map_err(|e| format!("Failed to insert filter: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        Ok(())
    }

    fn set_email_filters(
        &self,
        account: &str,
        uid: u32,
        filter_ids: &[String],
    ) -> Result<(), String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;

        let email_id: Option<i64> = conn
            .query_row(
                "SELECT id FROM emails WHERE account = ?1 AND uid = ?2",
                params![account, uid],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to lookup email id: {}", e))?;

        let Some(email_id) = email_id else {
            return Ok(());
        };

        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        tx.execute(
            "DELETE FROM filtered_emails WHERE email_id = ?1",
            params![email_id],
        )
        .map_err(|e| format!("Failed to clear mappings: {}", e))?;

        {
            let mut stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO filtered_emails (email_id, filter_id) \
                     VALUES (?1, ?2)",
                )
                .map_err(|e| format!("Failed to prepare mapping insert: {}", e))?;

            for filter_id in filter_ids {
                stmt.execute(params![email_id, filter_id])
                    .map_err(|e| format!("Failed to insert mapping: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        Ok(())
    }
}

fn get_db_path() -> Result<PathBuf, String> {
    Ok(get_db_dir()?.join("inboxcleanup.sqlite3"))
}

pub fn get_db_file_path() -> Result<PathBuf, String> {
    get_db_path()
}

pub fn get_db_dir() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?
        .join("InboxCleanup");
    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;
    Ok(config_dir)
}

fn migrate(conn: &Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE IF NOT EXISTS emails (
           id INTEGER PRIMARY KEY,
           uid INTEGER NOT NULL,
           message_id TEXT NOT NULL,
           subject TEXT NOT NULL,
           sender TEXT NOT NULL,
           date TEXT NOT NULL,
           mailbox TEXT NOT NULL,
           account TEXT NOT NULL,
           is_read INTEGER NOT NULL DEFAULT 0,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           UNIQUE(account, uid)
         );
         CREATE TABLE IF NOT EXISTS filters (
           id TEXT PRIMARY KEY,
           name TEXT NOT NULL,
           pattern TEXT NOT NULL,
           field TEXT NOT NULL,
           is_regex INTEGER NOT NULL DEFAULT 0,
           enabled INTEGER NOT NULL DEFAULT 1,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS filtered_emails (
           email_id INTEGER NOT NULL,
           filter_id TEXT NOT NULL,
           matched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           PRIMARY KEY (email_id, filter_id),
           FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
           FOREIGN KEY (filter_id) REFERENCES filters(id) ON DELETE CASCADE
         );
         CREATE INDEX IF NOT EXISTS idx_emails_uid ON emails(uid);
         CREATE INDEX IF NOT EXISTS idx_emails_message_id ON emails(message_id);
         CREATE INDEX IF NOT EXISTS idx_emails_is_read ON emails(is_read);
         CREATE INDEX IF NOT EXISTS idx_emails_date ON emails(date);
         CREATE INDEX IF NOT EXISTS idx_filtered_emails_filter_id ON filtered_emails(filter_id);
         CREATE INDEX IF NOT EXISTS idx_filtered_emails_email_id ON filtered_emails(email_id);
         COMMIT;",
    )
    .map_err(|e| format!("Failed to migrate DB: {}", e))?;
    Ok(())
}

fn maybe_import_filters(conn: &mut Connection) -> Result<(), String> {
    let existing: i64 = conn
        .query_row("SELECT COUNT(*) FROM filters", [], |row| row.get(0))
        .map_err(|e| format!("Failed to count filters: {}", e))?;
    if existing > 0 {
        return Ok(());
    }

    let config = crate::filters::load_filters()?;
    if config.patterns.is_empty() {
        return Ok(());
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start transaction: {}", e))?;
    {
        let mut stmt = tx
            .prepare(
                "INSERT INTO filters \
                    (id, name, pattern, field, is_regex, enabled) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            )
            .map_err(|e| format!("Failed to prepare filter import: {}", e))?;

        for filter in config.patterns {
            stmt.execute(params![
                filter.id,
                filter.name,
                filter.pattern,
                filter_field_to_string(&filter.field),
                if filter.is_regex { 1 } else { 0 },
                if filter.enabled { 1 } else { 0 }
            ])
            .map_err(|e| format!("Failed to import filter: {}", e))?;
        }
    }

    tx.commit()
        .map_err(|e| format!("Failed to commit filter import: {}", e))?;
    Ok(())
}

fn parse_filter_field(value: &str) -> Result<FilterField, rusqlite::Error> {
    match value {
        "subject" => Ok(FilterField::Subject),
        "sender" => Ok(FilterField::Sender),
        "any" => Ok(FilterField::Any),
        _ => Ok(FilterField::Any),
    }
}

fn filter_field_to_string(field: &FilterField) -> &'static str {
    match field {
        FilterField::Subject => "subject",
        FilterField::Sender => "sender",
        FilterField::Any => "any",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filters::FilterPattern;
    use crate::gmail::GmailEmail;
    use std::time::{SystemTime, UNIX_EPOCH};

    fn temp_db_path(label: &str) -> PathBuf {
        let mut path = std::env::temp_dir();
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        path.push(format!(
            "inboxcleanup-test-{}-{}-{}.sqlite3",
            label,
            std::process::id(),
            nanos
        ));
        path
    }

    #[test]
    fn upsert_and_mark_read_roundtrip() {
        let path = temp_db_path("upsert");
        {
            let storage = SqliteStorage::new_with_path(path.clone()).unwrap();
            let emails = vec![
                GmailEmail {
                    uid: 101,
                    message_id: "msg-101".to_string(),
                    subject: "Hello".to_string(),
                    sender: "Alice <alice@example.com>".to_string(),
                    date: "2024-01-01T10:00:00Z".to_string(),
                },
                GmailEmail {
                    uid: 102,
                    message_id: "msg-102".to_string(),
                    subject: "Update".to_string(),
                    sender: "Bob <bob@example.com>".to_string(),
                    date: "2024-01-02T12:00:00Z".to_string(),
                },
            ];

            storage
                .upsert_emails("test@example.com", "INBOX", &emails, false)
                .unwrap();

            let unread = storage.list_emails("test@example.com", true).unwrap();
            assert_eq!(unread.len(), 2);
            assert_eq!(unread[0].account, "test@example.com");
            assert!(!unread[0].is_read);

            let updated = storage
                .mark_emails_read("test@example.com", &[101])
                .unwrap();
            assert_eq!(updated, 1);

            let unread_after = storage.list_emails("test@example.com", true).unwrap();
            assert_eq!(unread_after.len(), 1);
            assert_eq!(unread_after[0].uid, 102);
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn save_and_load_filters() {
        let path = temp_db_path("filters");
        {
            let storage = SqliteStorage::new_with_path(path.clone()).unwrap();
            let patterns = vec![
                FilterPattern {
                    id: "f1".to_string(),
                    name: "Subject contains".to_string(),
                    pattern: "Hello".to_string(),
                    field: FilterField::Subject,
                    is_regex: false,
                    enabled: true,
                },
                FilterPattern {
                    id: "f2".to_string(),
                    name: "Sender regex".to_string(),
                    pattern: "example.com$".to_string(),
                    field: FilterField::Sender,
                    is_regex: true,
                    enabled: false,
                },
            ];

            storage.save_filters(&patterns).unwrap();
            let loaded = storage.get_filters().unwrap();
            assert_eq!(loaded.len(), 2);
            assert_eq!(loaded[0].id, "f1");
            assert_eq!(loaded[1].id, "f2");
        }
        let _ = std::fs::remove_file(path);
    }
}
