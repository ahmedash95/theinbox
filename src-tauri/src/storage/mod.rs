use crate::filters::{FilterField, FilterPattern};
use crate::gmail::GmailEmail;
use rusqlite::{params, Connection, OptionalExtension, ToSql};
use chrono::DateTime;
use regex::RegexBuilder;
use std::collections::HashMap;
use std::time::Duration;
use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;

/// Storage interface so we can swap implementations later.
pub trait Storage: Send + Sync {
    fn list_emails(
        &self,
        account: &str,
        unread_only: bool,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoredEmail>, String>;
    fn count_emails(&self, account: &str, unread_only: bool) -> Result<u64, String>;
    fn list_filtered_emails(
        &self,
        account: &str,
        filter_ids: &[i64],
        unread_only: bool,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoredEmail>, String>;
    fn count_filtered_emails(
        &self,
        account: &str,
        filter_ids: &[i64],
        unread_only: bool,
    ) -> Result<u64, String>;
    fn filter_match_counts(
        &self,
        account: &str,
        unread_only: bool,
    ) -> Result<Vec<(i64, u64)>, String>;
    fn refresh_filtered_emails(
        &self,
        account: &str,
        chunk_size: u32,
        force_full: bool,
    ) -> Result<usize, String>;
    fn get_last_uid(&self, account: &str) -> Result<u32, String>;
    fn set_last_uid(&self, account: &str, last_uid: u32) -> Result<(), String>;
    fn get_max_uid(&self, account: &str) -> Result<Option<u32>, String>;
    fn upsert_emails(
        &self,
        account: &str,
        mailbox: &str,
        emails: &[GmailEmail],
    ) -> Result<(), String>;
    fn mark_emails_read(&self, account: &str, uids: &[u32]) -> Result<usize, String>;
    fn mark_emails_unread(&self, account: &str, uids: &[u32]) -> Result<usize, String>;
    fn get_email_body(&self, account: &str, uid: u32) -> Result<Option<crate::gmail::EmailBody>, String>;
    fn set_email_bodies(
        &self,
        account: &str,
        bodies: &[crate::gmail::GmailEmailBody],
    ) -> Result<(), String>;
    fn get_filters(&self) -> Result<Vec<FilterPattern>, String>;
    fn save_filters(&self, patterns: &[FilterPattern]) -> Result<Vec<FilterPattern>, String>;
    fn set_email_filters(
        &self,
        account: &str,
        uid: u32,
        filter_ids: &[i64],
    ) -> Result<(), String>;
}

#[derive(Debug, Clone, serde::Serialize, serde::Deserialize)]
pub struct StoredEmail {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub date: String,
    pub date_epoch: i64,
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
        migrate(&mut conn)?;
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
        migrate(&mut conn)?;
        maybe_import_filters(&mut conn)?;
        Ok(Self {
            conn: Mutex::new(conn),
        })
    }
}

impl Storage for SqliteStorage {
    fn list_emails(
        &self,
        account: &str,
        unread_only: bool,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoredEmail>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let mut stmt = if unread_only {
            conn.prepare(
                "SELECT uid, message_id, subject, sender, date, IFNULL(date_epoch, 0), mailbox, account, is_read \
                 FROM emails \
                 WHERE account = ?1 AND is_read = 0 \
                 ORDER BY date_epoch DESC \
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?
        } else {
            conn.prepare(
                "SELECT uid, message_id, subject, sender, date, IFNULL(date_epoch, 0), mailbox, account, is_read \
                 FROM emails \
                 WHERE account = ?1 \
                 ORDER BY date_epoch DESC \
                 LIMIT ?2 OFFSET ?3",
            )
            .map_err(|e| format!("Failed to prepare query: {}", e))?
        };

        let rows = stmt
            .query_map(params![account, limit, offset], |row| {
                Ok(StoredEmail {
                    uid: row.get(0)?,
                    message_id: row.get(1)?,
                    subject: row.get(2)?,
                    sender: row.get(3)?,
                    date: row.get(4)?,
                    date_epoch: row.get(5)?,
                    mailbox: row.get(6)?,
                    account: row.get(7)?,
                    is_read: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query emails: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email: {}", e))?);
        }
        Ok(results)
    }

    fn count_emails(&self, account: &str, unread_only: bool) -> Result<u64, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let sql = if unread_only {
            "SELECT COUNT(*) FROM emails WHERE account = ?1 AND is_read = 0"
        } else {
            "SELECT COUNT(*) FROM emails WHERE account = ?1"
        };
        let count: u64 = conn
            .query_row(sql, params![account], |row| row.get(0))
            .map_err(|e| format!("Failed to count emails: {}", e))?;
        Ok(count)
    }

    fn list_filtered_emails(
        &self,
        account: &str,
        filter_ids: &[i64],
        unread_only: bool,
        limit: u32,
        offset: u32,
    ) -> Result<Vec<StoredEmail>, String> {
        if filter_ids.is_empty() {
            return Ok(Vec::new());
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let placeholders = std::iter::repeat("?")
            .take(filter_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = if unread_only {
            format!(
                "SELECT DISTINCT e.uid, e.message_id, e.subject, e.sender, e.date, IFNULL(e.date_epoch, 0), e.mailbox, e.account, e.is_read \
                 FROM emails e \
                 JOIN filtered_emails fe ON fe.email_id = e.id \
                 WHERE e.account = ?1 AND e.is_read = 0 AND fe.filter_id IN ({}) \
                 ORDER BY e.date_epoch DESC \
                 LIMIT ? OFFSET ?",
                placeholders
            )
        } else {
            format!(
                "SELECT DISTINCT e.uid, e.message_id, e.subject, e.sender, e.date, IFNULL(e.date_epoch, 0), e.mailbox, e.account, e.is_read \
                 FROM emails e \
                 JOIN filtered_emails fe ON fe.email_id = e.id \
                 WHERE e.account = ?1 AND fe.filter_id IN ({}) \
                 ORDER BY e.date_epoch DESC \
                 LIMIT ? OFFSET ?",
                placeholders
            )
        };

        let mut params: Vec<&dyn ToSql> = Vec::with_capacity(1 + filter_ids.len() + 2);
        params.push(&account);
        for filter_id in filter_ids {
            params.push(filter_id);
        }
        params.push(&limit);
        params.push(&offset);

        let mut stmt = conn
            .prepare(&sql)
            .map_err(|e| format!("Failed to prepare filtered query: {}", e))?;
        let rows = stmt
            .query_map(params.as_slice(), |row| {
                Ok(StoredEmail {
                    uid: row.get(0)?,
                    message_id: row.get(1)?,
                    subject: row.get(2)?,
                    sender: row.get(3)?,
                    date: row.get(4)?,
                    date_epoch: row.get(5)?,
                    mailbox: row.get(6)?,
                    account: row.get(7)?,
                    is_read: row.get::<_, i64>(8)? != 0,
                })
            })
            .map_err(|e| format!("Failed to query filtered emails: {}", e))?;

        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read email: {}", e))?);
        }
        Ok(results)
    }

    fn count_filtered_emails(
        &self,
        account: &str,
        filter_ids: &[i64],
        unread_only: bool,
    ) -> Result<u64, String> {
        if filter_ids.is_empty() {
            return Ok(0);
        }

        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let placeholders = std::iter::repeat("?")
            .take(filter_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = if unread_only {
            format!(
                "SELECT COUNT(DISTINCT e.id) \
                 FROM emails e \
                 JOIN filtered_emails fe ON fe.email_id = e.id \
                 WHERE e.account = ?1 AND e.is_read = 0 AND fe.filter_id IN ({})",
                placeholders
            )
        } else {
            format!(
                "SELECT COUNT(DISTINCT e.id) \
                 FROM emails e \
                 JOIN filtered_emails fe ON fe.email_id = e.id \
                 WHERE e.account = ?1 AND fe.filter_id IN ({})",
                placeholders
            )
        };

        let mut params: Vec<&dyn ToSql> = Vec::with_capacity(1 + filter_ids.len());
        params.push(&account);
        for filter_id in filter_ids {
            params.push(filter_id);
        }

        let count: u64 = conn
            .query_row(&sql, params.as_slice(), |row| row.get(0))
            .map_err(|e| format!("Failed to count filtered emails: {}", e))?;
        Ok(count)
    }

    fn filter_match_counts(
        &self,
        account: &str,
        unread_only: bool,
    ) -> Result<Vec<(i64, u64)>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let sql = "SELECT f.id, COUNT(e.id) \
            FROM filters f \
            LEFT JOIN filtered_emails fe ON fe.filter_id = f.id \
            LEFT JOIN emails e ON e.id = fe.email_id AND e.account = ?1 AND (?2 = 0 OR e.is_read = 0) \
            GROUP BY f.id \
            ORDER BY f.rowid ASC";
        let mut stmt = conn
            .prepare(sql)
            .map_err(|e| format!("Failed to prepare filter counts: {}", e))?;
        let rows = stmt
            .query_map(params![account, if unread_only { 1 } else { 0 }], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, u64>(1)?))
            })
            .map_err(|e| format!("Failed to query filter counts: {}", e))?;
        let mut results = Vec::new();
        for row in rows {
            results.push(row.map_err(|e| format!("Failed to read filter count: {}", e))?);
        }
        Ok(results)
    }

    fn refresh_filtered_emails(
        &self,
        account: &str,
        chunk_size: u32,
        force_full: bool,
    ) -> Result<usize, String> {
        let mut attempts = 0u32;
        let mut conn = loop {
            match self.conn.try_lock() {
                Ok(guard) => break guard,
                Err(_) => {
                    attempts += 1;
                    if attempts % 20 == 0 {
                        println!("[InboxCleanup] Waiting for DB lock to refresh filters...");
                    }
                    std::thread::sleep(Duration::from_millis(50));
                }
            }
        };

        if force_full {
            println!("[InboxCleanup] Filter refresh forcing full backfill (manual)");
            conn.execute(
                "DELETE FROM filtered_emails WHERE email_id IN (SELECT id FROM emails WHERE account = ?1)",
                params![account],
            )
            .map_err(|e| format!("Failed to clear filtered emails: {}", e))?;
            conn.execute(
                "DELETE FROM filter_sync_state_v2 WHERE account = ?1 AND scope = ?2",
                params![account, FILTER_SYNC_SCOPE],
            )
            .map_err(|e| format!("Failed to reset filter sync state: {}", e))?;
        }

        let mut last_id = get_filter_last_email_id(&conn, account)?;
        let filtered_count: u64 = conn
            .query_row(
                "SELECT COUNT(*) FROM filtered_emails fe \
                 JOIN emails e ON e.id = fe.email_id \
                 WHERE e.account = ?1",
                params![account],
                |row| row.get(0),
            )
            .map_err(|e| format!("Failed to count filtered emails: {}", e))?;
        if filtered_count == 0 && last_id > 0 {
            println!(
                "[InboxCleanup] Filter refresh forcing full backfill (last_id was {})",
                last_id
            );
            last_id = 0;
            set_filter_last_email_id(&conn, account, last_id)?;
        }
        let filters = load_filters_from_conn(&conn)?;
        let compiled_filters = compile_filters(&filters);
        println!(
            "[InboxCleanup] Filter refresh chunk start (last_id: {}, filters: {}, chunk_size: {})",
            last_id,
            compiled_filters.len(),
            chunk_size
        );

        let batch = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, uid, subject, sender \
                     FROM emails \
                     WHERE account = ?1 AND id > ?2 \
                     ORDER BY id ASC \
                     LIMIT ?3",
                )
                .map_err(|e| format!("Failed to prepare filter refresh query: {}", e))?;

            let rows = stmt
                .query_map(params![account, last_id, chunk_size], |row| {
                    Ok((
                        row.get::<_, i64>(0)?,
                        row.get::<_, u32>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                    ))
                })
                .map_err(|e| format!("Failed to query emails for filter refresh: {}", e))?;

            let mut batch = Vec::new();
            for row in rows {
                batch.push(row.map_err(|e| format!("Failed to read email row: {}", e))?);
            }
            batch
        };

        if batch.is_empty() {
            println!("[InboxCleanup] Filter refresh chunk empty; nothing to process.");
            return Ok(0);
        }

        let max_id = batch.last().map(|row| row.0).unwrap_or(last_id);
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start filter refresh transaction: {}", e))?;

        {
            let mut insert_stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO filtered_emails (email_id, filter_id) \
                     VALUES (?1, ?2)",
                )
                .map_err(|e| format!("Failed to prepare filter insert: {}", e))?;

            for (email_id, _uid, subject, sender) in &batch {
                let matches = match_filters(subject, sender, &compiled_filters);
                for filter_id in matches {
                    insert_stmt
                        .execute(params![email_id, filter_id])
                        .map_err(|e| format!("Failed to insert filter match: {}", e))?;
                }
            }
        }

        set_filter_last_email_id(&tx, account, max_id)?;
        tx.commit()
            .map_err(|e| format!("Failed to commit filter refresh: {}", e))?;

        println!(
            "[InboxCleanup] Filter refresh chunk committed (rows: {})",
            batch.len()
        );
        Ok(batch.len())
    }

    fn get_last_uid(&self, account: &str) -> Result<u32, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let last_uid: Option<u32> = conn
            .query_row(
                "SELECT last_uid FROM sync_state WHERE account = ?1",
                params![account],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| format!("Failed to read sync state: {}", e))?;
        Ok(last_uid.unwrap_or(0))
    }

    fn set_last_uid(&self, account: &str, last_uid: u32) -> Result<(), String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        conn.execute(
            "INSERT INTO sync_state (account, last_uid, updated_at)\
             VALUES (?1, ?2, CURRENT_TIMESTAMP)\
             ON CONFLICT(account) DO UPDATE SET\
                last_uid = excluded.last_uid,\
                updated_at = CURRENT_TIMESTAMP",
            params![account, last_uid],
        )
        .map_err(|e| format!("Failed to update sync state: {}", e))?;
        Ok(())
    }

    fn get_max_uid(&self, account: &str) -> Result<Option<u32>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let max_uid: Option<u32> = conn
            .query_row("SELECT MAX(uid) FROM emails WHERE account = ?1", params![account], |row| {
                row.get(0)
            })
            .optional()
            .map_err(|e| format!("Failed to read max uid: {}", e))?;
        Ok(max_uid)
    }

    fn upsert_emails(
        &self,
        account: &str,
        mailbox: &str,
        emails: &[GmailEmail],
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
                        (uid, message_id, subject, sender, date, date_epoch, mailbox, account, is_read) \
                 VALUES \
                    (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9) \
                 ON CONFLICT(account, uid) DO UPDATE SET \
                    message_id = excluded.message_id,\
                    subject = excluded.subject,\
                    sender = excluded.sender,\
                    date = excluded.date,\
                    date_epoch = excluded.date_epoch,\
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
                    email.date_epoch,
                    mailbox,
                    account,
                    if email.is_read { 1 } else { 0 }
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

    fn mark_emails_unread(&self, account: &str, uids: &[u32]) -> Result<usize, String> {
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
                "UPDATE emails SET is_read = 0, updated_at = CURRENT_TIMESTAMP \
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
                .map_err(|e| format!("Failed to mark unread: {}", e))?;
            total += updated;
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;
        Ok(total)
    }

    fn get_email_body(&self, account: &str, uid: u32) -> Result<Option<crate::gmail::EmailBody>, String> {
        let conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;

        let row: Option<(Option<String>, Option<String>)> = conn
            .query_row(
                "SELECT body_html, body_text FROM emails WHERE account = ?1 AND uid = ?2",
                params![account, uid],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|e| format!("Failed to query email body: {}", e))?;

        Ok(row.and_then(|(html, text)| {
            if html.is_some() || text.is_some() {
                Some(crate::gmail::EmailBody { html, text })
            } else {
                None
            }
        }))
    }

    fn set_email_bodies(
        &self,
        account: &str,
        bodies: &[crate::gmail::GmailEmailBody],
    ) -> Result<(), String> {
        if bodies.is_empty() {
            return Ok(());
        }

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
                    "UPDATE emails SET body_html = ?1, body_text = ?2, updated_at = CURRENT_TIMESTAMP \
                     WHERE account = ?3 AND uid = ?4",
                )
                .map_err(|e| format!("Failed to prepare body update: {}", e))?;

            for body in bodies {
                stmt.execute(params![
                    body.body.html.as_deref(),
                    body.body.text.as_deref(),
                    account,
                    body.uid
                ])
                .map_err(|e| format!("Failed to update body: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit body updates: {}", e))?;
        Ok(())
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
                    id: row.get::<_, i64>(0)?,
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

    fn save_filters(&self, patterns: &[FilterPattern]) -> Result<Vec<FilterPattern>, String> {
        let mut conn = self
            .conn
            .lock()
            .map_err(|_| "Failed to lock DB".to_string())?;
        let existing_filters = load_filters_from_conn(&conn)?;
        let mut existing_map: HashMap<i64, FilterPattern> = HashMap::new();
        for filter in existing_filters {
            existing_map.insert(filter.id.clone(), filter);
        }

        let mut to_delete: Vec<i64> = Vec::new();
        let mut to_insert: Vec<FilterPattern> = Vec::new();
        let mut to_update: Vec<FilterPattern> = Vec::new();
        let mut to_touch: Vec<FilterPattern> = Vec::new();

        for filter in patterns {
            if let Some(previous) = existing_map.remove(&filter.id) {
                let needs_refresh = previous.pattern != filter.pattern
                    || previous.is_regex != filter.is_regex
                    || filter_field_to_string(&previous.field) != filter_field_to_string(&filter.field);
                if needs_refresh {
                    to_update.push(filter.clone());
                } else if previous.name != filter.name || previous.enabled != filter.enabled {
                    to_touch.push(filter.clone());
                }
            } else {
                to_insert.push(filter.clone());
            }
        }

        for (id, _) in existing_map {
            to_delete.push(id);
        }

        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start transaction: {}", e))?;

        if !to_delete.is_empty() {
            let placeholders = std::iter::repeat("?")
                .take(to_delete.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("DELETE FROM filters WHERE id IN ({})", placeholders);
            let mut params: Vec<&dyn ToSql> = Vec::with_capacity(to_delete.len());
            for id in &to_delete {
                params.push(id);
            }
            tx.execute(&sql, params.as_slice())
                .map_err(|e| format!("Failed to delete filters: {}", e))?;
        }

        if !to_update.is_empty() {
            let update_ids: Vec<i64> = to_update.iter().map(|filter| filter.id).collect();
            let placeholders = std::iter::repeat("?")
                .take(update_ids.len())
                .collect::<Vec<_>>()
                .join(",");
            let sql = format!("DELETE FROM filtered_emails WHERE filter_id IN ({})", placeholders);
            let mut params: Vec<&dyn ToSql> = Vec::with_capacity(update_ids.len());
            for id in &update_ids {
                params.push(id);
            }
            tx.execute(&sql, params.as_slice())
                .map_err(|e| format!("Failed to clear filter mappings: {}", e))?;
        }

        let mut inserted_filters: Vec<FilterPattern> = Vec::new();
        {
            let mut insert_autoinc_stmt = tx
                .prepare(
                    "INSERT INTO filters \
                        (name, pattern, field, is_regex, enabled) \
                     VALUES (?1, ?2, ?3, ?4, ?5)",
                )
                .map_err(|e| format!("Failed to prepare filter insert: {}", e))?;

            let mut update_stmt = tx
                .prepare(
                    "UPDATE filters \
                     SET name = ?1, pattern = ?2, field = ?3, is_regex = ?4, enabled = ?5 \
                     WHERE id = ?6",
                )
                .map_err(|e| format!("Failed to prepare filter update: {}", e))?;

            for filter in &to_insert {
                insert_autoinc_stmt
                    .execute(params![
                        filter.name,
                        filter.pattern,
                        filter_field_to_string(&filter.field),
                        if filter.is_regex { 1 } else { 0 },
                        if filter.enabled { 1 } else { 0 }
                    ])
                    .map_err(|e| format!("Failed to insert filter: {}", e))?;
                let new_id = tx.last_insert_rowid();
                let mut cloned = filter.clone();
                cloned.id = new_id;
                inserted_filters.push(cloned);
            }

            for filter in to_update.iter().chain(to_touch.iter()) {
                update_stmt
                    .execute(params![
                        filter.name,
                        filter.pattern,
                        filter_field_to_string(&filter.field),
                        if filter.is_regex { 1 } else { 0 },
                        if filter.enabled { 1 } else { 0 },
                        filter.id
                    ])
                    .map_err(|e| format!("Failed to update filter: {}", e))?;
            }
        }

        tx.commit()
            .map_err(|e| format!("Failed to commit transaction: {}", e))?;

        let mut refresh_filters: Vec<FilterPattern> = to_update;
        refresh_filters.extend(inserted_filters);
        if !refresh_filters.is_empty() {
            let accounts = load_filter_accounts(&conn)?;
            for account in accounts {
                refresh_filter_matches_for_account(&mut conn, &account, &refresh_filters, 500)?;
            }
        }
        load_filters_from_conn(&conn)
    }

    fn set_email_filters(
        &self,
        account: &str,
        uid: u32,
        filter_ids: &[i64],
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

fn migrate(conn: &mut Connection) -> Result<(), String> {
    conn.execute_batch(
        "BEGIN;
         CREATE TABLE IF NOT EXISTS emails (
           id INTEGER PRIMARY KEY,
           uid INTEGER NOT NULL,
           message_id TEXT NOT NULL,
           subject TEXT NOT NULL,
           sender TEXT NOT NULL,
           date TEXT NOT NULL,
           date_epoch INTEGER NOT NULL DEFAULT 0,
           mailbox TEXT NOT NULL,
           account TEXT NOT NULL,
           is_read INTEGER NOT NULL DEFAULT 0,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           UNIQUE(account, uid)
         );
         CREATE TABLE IF NOT EXISTS filters (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           name TEXT NOT NULL,
           pattern TEXT NOT NULL,
           field TEXT NOT NULL,
           is_regex INTEGER NOT NULL DEFAULT 0,
           enabled INTEGER NOT NULL DEFAULT 1,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS sync_state (
           account TEXT PRIMARY KEY,
           last_uid INTEGER NOT NULL DEFAULT 0,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS filtered_emails (
           email_id INTEGER NOT NULL,
           filter_id INTEGER NOT NULL,
           matched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           PRIMARY KEY (email_id, filter_id),
           FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
           FOREIGN KEY (filter_id) REFERENCES filters(id) ON DELETE CASCADE
         );
         CREATE TABLE IF NOT EXISTS filter_sync_state (
           account TEXT PRIMARY KEY,
           last_email_id INTEGER NOT NULL DEFAULT 0,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE IF NOT EXISTS filter_sync_state_v2 (
           account TEXT NOT NULL,
           scope TEXT NOT NULL,
           last_email_id INTEGER NOT NULL DEFAULT 0,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           PRIMARY KEY (account, scope)
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

    migrate_filters_to_integer_ids(conn)?;
    ensure_column(conn, "emails", "body_html", "TEXT")?;
    ensure_column(conn, "emails", "body_text", "TEXT")?;
    ensure_column(conn, "emails", "date_epoch", "INTEGER")?;
    backfill_date_epoch(conn)?;
    Ok(())
}

fn migrate_filters_to_integer_ids(conn: &mut Connection) -> Result<(), String> {
    let Some(column_type) = get_column_type(conn, "filters", "id")? else {
        return Ok(());
    };
    if column_type.to_lowercase().contains("int") {
        return Ok(());
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start filter id migration: {}", e))?;
    tx.execute_batch(
        "CREATE TABLE filters_v2 (
           id INTEGER PRIMARY KEY AUTOINCREMENT,
           name TEXT NOT NULL,
           pattern TEXT NOT NULL,
           field TEXT NOT NULL,
           is_regex INTEGER NOT NULL DEFAULT 0,
           enabled INTEGER NOT NULL DEFAULT 1,
           created_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           updated_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP
         );
         CREATE TABLE filtered_emails_v2 (
           email_id INTEGER NOT NULL,
           filter_id INTEGER NOT NULL,
           matched_at TEXT NOT NULL DEFAULT CURRENT_TIMESTAMP,
           PRIMARY KEY (email_id, filter_id),
           FOREIGN KEY (email_id) REFERENCES emails(id) ON DELETE CASCADE,
           FOREIGN KEY (filter_id) REFERENCES filters_v2(id) ON DELETE CASCADE
         );",
    )
    .map_err(|e| format!("Failed to create filter id migration tables: {}", e))?;

    let mut id_map: HashMap<String, i64> = HashMap::new();
    {
        let mut stmt = tx
            .prepare(
                "SELECT id, name, pattern, field, is_regex, enabled, created_at, updated_at \
                 FROM filters ORDER BY rowid ASC",
            )
            .map_err(|e| format!("Failed to query filters for migration: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, i64>(4)?,
                    row.get::<_, i64>(5)?,
                    row.get::<_, String>(6)?,
                    row.get::<_, String>(7)?,
                ))
            })
            .map_err(|e| format!("Failed to read filters for migration: {}", e))?;

        let mut insert_stmt = tx
            .prepare(
                "INSERT INTO filters_v2 \
                    (name, pattern, field, is_regex, enabled, created_at, updated_at) \
                 VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            )
            .map_err(|e| format!("Failed to prepare filter migration insert: {}", e))?;

        for row in rows {
            let (old_id, name, pattern, field, is_regex, enabled, created_at, updated_at) =
                row.map_err(|e| format!("Failed to read filter migration row: {}", e))?;
            insert_stmt
                .execute(params![
                    name,
                    pattern,
                    field,
                    is_regex,
                    enabled,
                    created_at,
                    updated_at
                ])
                .map_err(|e| format!("Failed to insert migrated filter: {}", e))?;
            let new_id = tx.last_insert_rowid();
            id_map.insert(old_id, new_id);
        }
    }

    {
        let mut stmt = tx
            .prepare("SELECT email_id, filter_id, matched_at FROM filtered_emails")
            .map_err(|e| format!("Failed to query filtered_emails for migration: {}", e))?;
        let rows = stmt
            .query_map([], |row| {
                Ok((
                    row.get::<_, i64>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(|e| format!("Failed to read filtered_emails for migration: {}", e))?;

        let mut insert_stmt = tx
            .prepare(
                "INSERT OR IGNORE INTO filtered_emails_v2 \
                 (email_id, filter_id, matched_at) VALUES (?1, ?2, ?3)",
            )
            .map_err(|e| format!("Failed to prepare filtered_emails migration insert: {}", e))?;

        for row in rows {
            let (email_id, old_filter_id, matched_at) =
                row.map_err(|e| format!("Failed to read filtered_emails migration row: {}", e))?;
            if let Some(new_id) = id_map.get(&old_filter_id) {
                insert_stmt
                    .execute(params![email_id, new_id, matched_at])
                    .map_err(|e| format!("Failed to insert migrated filtered email: {}", e))?;
            }
        }
    }

    tx.execute_batch(
        "DROP TABLE filtered_emails;
         DROP TABLE filters;
         ALTER TABLE filters_v2 RENAME TO filters;
         ALTER TABLE filtered_emails_v2 RENAME TO filtered_emails;
         CREATE INDEX IF NOT EXISTS idx_filtered_emails_filter_id ON filtered_emails(filter_id);
         CREATE INDEX IF NOT EXISTS idx_filtered_emails_email_id ON filtered_emails(email_id);",
    )
    .map_err(|e| format!("Failed to finalize filter id migration: {}", e))?;

    tx.commit()
        .map_err(|e| format!("Failed to commit filter id migration: {}", e))?;
    Ok(())
}

fn get_column_type(conn: &Connection, table: &str, column: &str) -> Result<Option<String>, String> {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to inspect schema: {}", e))?;
    let rows = stmt
        .query_map([], |row| Ok((row.get::<_, String>(1)?, row.get::<_, String>(2)?)))
        .map_err(|e| format!("Failed to read schema: {}", e))?;
    for row in rows {
        let (name, column_type) = row.map_err(|e| format!("Failed to read schema row: {}", e))?;
        if name == column {
            return Ok(Some(column_type));
        }
    }
    Ok(None)
}

fn backfill_date_epoch(conn: &mut Connection) -> Result<(), String> {
    let mut updates = Vec::new();
    {
        let mut stmt = conn
            .prepare("SELECT id, date FROM emails WHERE date_epoch = 0 OR date_epoch IS NULL")
            .map_err(|e| format!("Failed to query dates: {}", e))?;
        let rows = stmt
            .query_map([], |row| Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?)))
            .map_err(|e| format!("Failed to read dates: {}", e))?;

        for row in rows {
            let (id, date_str) = row.map_err(|e| format!("Failed to read row: {}", e))?;
            if let Ok(dt) = DateTime::parse_from_rfc2822(&date_str) {
                updates.push((dt.timestamp(), id));
            }
        }
    }

    if updates.is_empty() {
        return Ok(());
    }

    let tx = conn
        .transaction()
        .map_err(|e| format!("Failed to start backfill transaction: {}", e))?;
    {
        let mut update_stmt = tx
            .prepare("UPDATE emails SET date_epoch = ?1 WHERE id = ?2")
            .map_err(|e| format!("Failed to prepare backfill: {}", e))?;
        for (epoch, id) in updates {
            update_stmt
                .execute(params![epoch, id])
                .map_err(|e| format!("Failed to update date_epoch: {}", e))?;
        }
    }
    tx.commit()
        .map_err(|e| format!("Failed to commit backfill: {}", e))?;
    Ok(())
}

fn ensure_column(conn: &Connection, table: &str, column: &str, column_type: &str) -> Result<(), String> {
    let sql = format!("PRAGMA table_info({})", table);
    let mut stmt = conn
        .prepare(&sql)
        .map_err(|e| format!("Failed to inspect schema: {}", e))?;
    let existing = stmt
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(|e| format!("Failed to read schema: {}", e))?
        .collect::<Result<Vec<String>, _>>()
        .map_err(|e| format!("Failed to read columns: {}", e))?;

    if existing.iter().any(|name| name == column) {
        return Ok(());
    }

    let sql = format!(
        "ALTER TABLE {} ADD COLUMN {} {}",
        table, column, column_type
    );
    conn.execute(&sql, [])
        .map_err(|e| format!("Failed to add column {}: {}", column, e))?;
    Ok(())
}

const FILTER_SYNC_SCOPE: &str = "filters_v1";

fn get_filter_last_email_id(conn: &Connection, account: &str) -> Result<i64, String> {
    let last_id: Option<i64> = conn
        .query_row(
            "SELECT last_email_id FROM filter_sync_state_v2 WHERE account = ?1 AND scope = ?2",
            params![account, FILTER_SYNC_SCOPE],
            |row| row.get(0),
        )
        .optional()
        .map_err(|e| format!("Failed to read filter sync state: {}", e))?;
    Ok(last_id.unwrap_or(0))
}

fn set_filter_last_email_id(conn: &Connection, account: &str, last_id: i64) -> Result<(), String> {
    conn.execute(
        "INSERT INTO filter_sync_state_v2 (account, scope, last_email_id, updated_at) \
         VALUES (?1, ?2, ?3, CURRENT_TIMESTAMP) \
         ON CONFLICT(account, scope) DO UPDATE SET \
            last_email_id = excluded.last_email_id, \
            updated_at = CURRENT_TIMESTAMP",
        params![account, FILTER_SYNC_SCOPE, last_id],
    )
    .map_err(|e| format!("Failed to update filter sync state: {}", e))?;
    Ok(())
}

fn load_filters_from_conn(conn: &Connection) -> Result<Vec<FilterPattern>, String> {
    let mut stmt = conn
        .prepare(
            "SELECT id, name, pattern, field, is_regex, enabled \
             FROM filters ORDER BY rowid ASC",
        )
        .map_err(|e| format!("Failed to prepare filters query: {}", e))?;
    let rows = stmt
        .query_map([], |row| {
            let field: String = row.get(3)?;
            Ok(FilterPattern {
                id: row.get::<_, i64>(0)?,
                name: row.get(1)?,
                pattern: row.get(2)?,
                field: parse_filter_field(&field)?,
                is_regex: row.get::<_, i64>(4)? != 0,
                enabled: row.get::<_, i64>(5)? != 0,
            })
        })
        .map_err(|e| format!("Failed to read filters: {}", e))?;
    let mut filters = Vec::new();
    for row in rows {
        filters.push(row.map_err(|e| format!("Failed to read filter: {}", e))?);
    }
    Ok(filters)
}

#[derive(Clone)]
struct CompiledFilter {
    id: i64,
    field: FilterField,
    regex: Option<regex::Regex>,
    pattern_lower: Option<String>,
}

fn compile_filters(filters: &[FilterPattern]) -> Vec<CompiledFilter> {
    filters
        .iter()
        .map(|filter| {
            let regex = if filter.is_regex {
                RegexBuilder::new(&filter.pattern)
                    .case_insensitive(true)
                    .build()
                    .ok()
            } else {
                None
            };
            let pattern_lower = if filter.is_regex {
                None
            } else {
                Some(filter.pattern.to_lowercase())
            };
            CompiledFilter {
                id: filter.id.clone(),
                field: filter.field.clone(),
                regex,
                pattern_lower,
            }
        })
        .collect()
}

fn match_filters(subject: &str, sender: &str, filters: &[CompiledFilter]) -> Vec<i64> {
    let subject_lower = subject.to_lowercase();
    let sender_lower = sender.to_lowercase();
    let mut matches = Vec::new();

    for filter in filters {
        let is_match = if let Some(regex) = &filter.regex {
            match filter.field {
                FilterField::Subject => regex.is_match(subject),
                FilterField::Sender => regex.is_match(sender),
                FilterField::Any => regex.is_match(subject) || regex.is_match(sender),
            }
        } else if let Some(pattern) = &filter.pattern_lower {
            match filter.field {
                FilterField::Subject => subject_lower.contains(pattern),
                FilterField::Sender => sender_lower.contains(pattern),
                FilterField::Any => subject_lower.contains(pattern) || sender_lower.contains(pattern),
            }
        } else {
            false
        };

        if is_match {
            matches.push(filter.id.clone());
        }
    }

    matches
}

fn load_filter_accounts(conn: &Connection) -> Result<Vec<String>, String> {
    let mut stmt = conn
        .prepare("SELECT DISTINCT account FROM emails")
        .map_err(|e| format!("Failed to prepare account query: {}", e))?;
    let rows = stmt
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(|e| format!("Failed to query accounts: {}", e))?;
    let mut accounts = Vec::new();
    for row in rows {
        accounts.push(row.map_err(|e| format!("Failed to read account: {}", e))?);
    }
    Ok(accounts)
}

fn refresh_filter_matches_for_account(
    conn: &mut Connection,
    account: &str,
    filters: &[FilterPattern],
    chunk_size: u32,
) -> Result<(), String> {
    if filters.is_empty() {
        return Ok(());
    }

    let compiled_filters = compile_filters(filters);
    let mut last_id = 0i64;

    loop {
        let batch = {
            let mut stmt = conn
                .prepare(
                    "SELECT id, subject, sender \
                     FROM emails \
                     WHERE account = ?1 AND id > ?2 \
                     ORDER BY id ASC \
                     LIMIT ?3",
                )
                .map_err(|e| format!("Failed to prepare filter refresh query: {}", e))?;
            let rows = stmt
                .query_map(params![account, last_id, chunk_size], |row| {
                    Ok((row.get::<_, i64>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?))
                })
                .map_err(|e| format!("Failed to query emails for filter refresh: {}", e))?;

            let mut batch = Vec::new();
            for row in rows {
                batch.push(row.map_err(|e| format!("Failed to read email row: {}", e))?);
            }
            batch
        };

        if batch.is_empty() {
            break;
        }

        let max_id = batch.last().map(|row| row.0).unwrap_or(last_id);
        let tx = conn
            .transaction()
            .map_err(|e| format!("Failed to start filter refresh transaction: {}", e))?;
        {
            let mut insert_stmt = tx
                .prepare(
                    "INSERT OR IGNORE INTO filtered_emails (email_id, filter_id) \
                     VALUES (?1, ?2)",
                )
                .map_err(|e| format!("Failed to prepare filter insert: {}", e))?;

            for (email_id, subject, sender) in &batch {
                let matches = match_filters(subject, sender, &compiled_filters);
                for filter_id in matches {
                    insert_stmt
                        .execute(params![email_id, filter_id])
                        .map_err(|e| format!("Failed to insert filter match: {}", e))?;
                }
            }
        }
        tx.commit()
            .map_err(|e| format!("Failed to commit filter refresh: {}", e))?;
        last_id = max_id;
    }

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
                    (name, pattern, field, is_regex, enabled) \
                 VALUES (?1, ?2, ?3, ?4, ?5)",
            )
            .map_err(|e| format!("Failed to prepare filter import: {}", e))?;

        for filter in config.patterns {
            stmt.execute(params![
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
    use std::collections::HashMap;
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
                    date_epoch: 1704103200,
                    is_read: false,
                },
                GmailEmail {
                    uid: 102,
                    message_id: "msg-102".to_string(),
                    subject: "Update".to_string(),
                    sender: "Bob <bob@example.com>".to_string(),
                    date: "2024-01-02T12:00:00Z".to_string(),
                    date_epoch: 1704196800,
                    is_read: true,
                },
            ];

            storage
                .upsert_emails("test@example.com", "INBOX", &emails)
                .unwrap();

            let unread = storage
                .list_emails("test@example.com", true, 50, 0)
                .unwrap();
            assert_eq!(unread.len(), 1);
            assert_eq!(unread[0].account, "test@example.com");
            assert!(!unread[0].is_read);

            let updated = storage
                .mark_emails_read("test@example.com", &[101])
                .unwrap();
            assert_eq!(updated, 1);

            let unread_after = storage
                .list_emails("test@example.com", true, 50, 0)
                .unwrap();
            assert_eq!(unread_after.len(), 0);
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
                    id: 0,
                    name: "Subject contains".to_string(),
                    pattern: "Hello".to_string(),
                    field: FilterField::Subject,
                    is_regex: false,
                    enabled: true,
                },
                FilterPattern {
                    id: 0,
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
            assert!(loaded[0].id > 0);
            assert!(loaded[1].id > 0);
        }
        let _ = std::fs::remove_file(path);
    }

    fn make_email(uid: u32, subject: &str, sender: &str) -> GmailEmail {
        GmailEmail {
            uid,
            message_id: format!("msg-{}", uid),
            subject: subject.to_string(),
            sender: sender.to_string(),
            date: "2024-01-02T12:00:00Z".to_string(),
            date_epoch: 1704196800,
            is_read: false,
        }
    }

    #[test]
    fn filter_refresh_matches_old_and_new_emails_in_batches() {
        let path = temp_db_path("filters-batch");
        {
            let storage = SqliteStorage::new_with_path(path.clone()).unwrap();
            let patterns = vec![
                FilterPattern {
                    id: 0,
                    name: "Subject contains invoice".to_string(),
                    pattern: "invoice".to_string(),
                    field: FilterField::Subject,
                    is_regex: false,
                    enabled: true,
                },
                FilterPattern {
                    id: 0,
                    name: "Sender regex".to_string(),
                    pattern: "@vip\\.example\\.com$".to_string(),
                    field: FilterField::Sender,
                    is_regex: true,
                    enabled: true,
                },
            ];
            let saved = storage.save_filters(&patterns).unwrap();
            let subject_id = saved[0].id;
            let sender_id = saved[1].id;

            let account = "old-new@example.com";
            let old_emails = vec![
                make_email(10, "Invoice March", "billing@corp.com"),
                make_email(11, "Hello", "ceo@vip.example.com"),
            ];
            storage.upsert_emails(account, "INBOX", &old_emails).unwrap();

            let processed_first = storage.refresh_filtered_emails(account, 1, true).unwrap();
            assert_eq!(processed_first, 1);
            let processed_second = storage.refresh_filtered_emails(account, 1, false).unwrap();
            assert_eq!(processed_second, 1);
            let processed_third = storage.refresh_filtered_emails(account, 1, false).unwrap();
            assert_eq!(processed_third, 0);

            let counts = storage.filter_match_counts(account, false).unwrap();
            let counts_map: HashMap<i64, u64> = counts.into_iter().collect();
            assert_eq!(counts_map.get(&subject_id), Some(&1));
            assert_eq!(counts_map.get(&sender_id), Some(&1));

            let new_emails = vec![make_email(12, "Invoice April", "billing@corp.com")];
            storage.upsert_emails(account, "INBOX", &new_emails).unwrap();

            let processed_new = storage.refresh_filtered_emails(account, 10, false).unwrap();
            assert_eq!(processed_new, 1);

            let counts_after = storage.filter_match_counts(account, false).unwrap();
            let counts_after_map: HashMap<i64, u64> = counts_after.into_iter().collect();
            assert_eq!(counts_after_map.get(&subject_id), Some(&2));
            assert_eq!(counts_after_map.get(&sender_id), Some(&1));
        }
        let _ = std::fs::remove_file(path);
    }

    #[test]
    fn filter_refresh_rewinds_when_filtered_empty_but_last_id_set() {
        let path = temp_db_path("filters-rematch");
        {
            let storage = SqliteStorage::new_with_path(path.clone()).unwrap();
            let patterns = vec![FilterPattern {
                id: 0,
                name: "Subject contains".to_string(),
                pattern: "Hello".to_string(),
                field: FilterField::Subject,
                is_regex: false,
                enabled: true,
            }];
            let saved = storage.save_filters(&patterns).unwrap();
            let filter_id = saved[0].id;

            let account = "rematch@example.com";
            let emails = vec![
                make_email(20, "Hello World", "alice@example.com"),
                make_email(21, "Hello Again", "bob@example.com"),
            ];
            storage.upsert_emails(account, "INBOX", &emails).unwrap();

            {
                let conn = storage.conn.lock().unwrap();
                set_filter_last_email_id(&conn, account, 999).unwrap();
            }

            let processed = storage.refresh_filtered_emails(account, 50, false).unwrap();
            assert_eq!(processed, 2);

            let counts = storage.filter_match_counts(account, false).unwrap();
            let counts_map: HashMap<i64, u64> = counts.into_iter().collect();
            assert_eq!(counts_map.get(&filter_id), Some(&2));

            let last_id = {
                let conn = storage.conn.lock().unwrap();
                get_filter_last_email_id(&conn, account).unwrap()
            };
            assert_eq!(last_id, 2);
        }
        let _ = std::fs::remove_file(path);
    }
}
