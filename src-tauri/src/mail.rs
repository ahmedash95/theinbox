use glob::glob;
use rusqlite::{Connection, OpenFlags};
use serde::{Deserialize, Serialize};
use std::process::Command;
use mail_parser::MessageParser;

/// Log a message to stdout for debugging
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[InboxCleanup] {}", format!($($arg)*));
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Email {
    pub id: String,
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub date_received: String,
    pub mailbox: String,
    pub account: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPattern {
    pub id: String,
    pub name: String,
    pub pattern: String,
    pub field: FilterField,
    #[serde(default)]
    pub is_regex: bool,
    pub enabled: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBody {
    pub html: Option<String>,
    pub text: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum FilterField {
    Subject,
    Sender,
    Any,
}

/// Execute AppleScript and return output
fn run_applescript(script: &str) -> Result<String, String> {
    let output = Command::new("osascript")
        .arg("-e")
        .arg(script)
        .output()
        .map_err(|e| format!("Failed to execute AppleScript: {}", e))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr);
        return Err(format!("AppleScript error: {}", stderr));
    }

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}

/// Parse email output from AppleScript
fn parse_email_output(stdout: &str) -> Vec<Email> {
    stdout
        .lines()
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split("||").collect();
            if parts.len() >= 6 {
                Some(Email {
                    id: parts[0].to_string(),
                    message_id: parts[0].to_string(), // Use same as id
                    subject: parts[1].to_string(),
                    sender: parts[2].to_string(),
                    date_received: parts[3].to_string(),
                    mailbox: parts[4].to_string(),
                    account: parts[5].to_string(),
                })
            } else {
                None
            }
        })
        .collect()
}

/// Fetch unread emails from inbox only (faster)
pub fn fetch_unread_emails_inbox_only() -> Result<Vec<Email>, String> {
    log!("Fetching unread emails from Inbox only...");
    let start = std::time::Instant::now();

    let script = r#"
        set output to ""
        set delim to "||"
        set rowDelim to ASCII character 10
        
        tell application "Mail"
            set unreadMsgs to (every message of inbox whose read status is false)
            
            repeat with msg in unreadMsgs
                try
                    set msgId to id of msg
                    set msgSubject to subject of msg
                    set msgSender to sender of msg
                    set msgDate to date received of msg as string
                    set msgMailbox to name of mailbox of msg
                    set msgAccount to name of account of mailbox of msg
                    set output to output & msgId & delim & msgSubject & delim & msgSender & delim & msgDate & delim & msgMailbox & delim & msgAccount & rowDelim
                end try
            end repeat
        end tell
        
        return output
    "#;

    let stdout = run_applescript(script)?;
    log!("AppleScript completed in {:?}", start.elapsed());

    let emails = parse_email_output(&stdout);
    log!("Fetched {} unread emails in {:?}", emails.len(), start.elapsed());
    Ok(emails)
}

/// Fetch all unread emails from all mailboxes (slower but complete)
pub fn fetch_unread_emails() -> Result<Vec<Email>, String> {
    log!("Fetching unread emails from all mailboxes...");
    let start = std::time::Instant::now();

    let script = r#"
        set output to ""
        set delim to "||"
        set rowDelim to ASCII character 10
        
        tell application "Mail"
            set allAccounts to every account
            repeat with acc in allAccounts
                set accName to name of acc
                set accMailboxes to every mailbox of acc
                repeat with mb in accMailboxes
                    try
                        set mbName to name of mb
                        set unreadMsgs to (every message of mb whose read status is false)
                        repeat with msg in unreadMsgs
                            try
                                set msgId to id of msg
                                set msgSubject to subject of msg
                                set msgSender to sender of msg
                                set msgDate to date received of msg as string
                                set output to output & msgId & delim & msgSubject & delim & msgSender & delim & msgDate & delim & mbName & delim & accName & rowDelim
                            end try
                        end repeat
                    end try
                end repeat
            end repeat
        end tell
        
        return output
    "#;

    let stdout = run_applescript(script)?;
    log!("AppleScript completed in {:?}", start.elapsed());

    let emails = parse_email_output(&stdout);
    log!("Fetched {} unread emails in {:?}", emails.len(), start.elapsed());
    Ok(emails)
}

/// Mark specific emails as read using their IDs
pub fn mark_emails_as_read(email_ids: Vec<String>) -> Result<usize, String> {
    log!("Marking {} emails as read...", email_ids.len());

    if email_ids.is_empty() {
        return Ok(0);
    }

    let start = std::time::Instant::now();

    let ids_list = email_ids
        .iter()
        .map(|id| id.to_string())
        .collect::<Vec<_>>()
        .join(", ");

    let script = format!(
        r#"
        set idsToMark to {{{}}}
        set markedCount to 0
        
        tell application "Mail"
            set msgs to messages of inbox whose read status is false
            repeat with msg in msgs
                if (id of msg) is in idsToMark then
                    set read status of msg to true
                    set markedCount to markedCount + 1
                end if
            end repeat
        end tell
        
        return markedCount
    "#,
        ids_list
    );

    let stdout = run_applescript(&script)?;
    let count: usize = stdout.trim().parse().unwrap_or(0);

    log!("Marked {} emails as read in {:?}", count, start.elapsed());
    Ok(count)
}

// =============================================================================
// Direct SQLite Access (High Performance)
// =============================================================================

/// Find the Apple Mail Envelope Index database path
/// Handles V9 (Monterey), V10 (Ventura), V11 (Sonoma+) variations
fn find_mail_db_path() -> Result<String, String> {
    let home = dirs::home_dir().ok_or("Could not find home directory")?;
    let pattern = home
        .join("Library/Mail/V*/MailData/Envelope Index")
        .to_string_lossy()
        .to_string();

    let mut matches: Vec<_> = glob(&pattern)
        .map_err(|e| format!("Invalid glob pattern: {}", e))?
        .filter_map(|r| r.ok())
        .collect();

    if matches.is_empty() {
        return Err(
            "Mail database not found. Ensure Apple Mail is set up and Full Disk Access is granted."
                .to_string(),
        );
    }

    // Sort to get the highest version (V11 > V10 > V9)
    matches.sort();
    let db_path = matches.last().unwrap();

    log!("Found Mail DB at: {:?}", db_path);
    Ok(db_path.to_string_lossy().to_string())
}

/// Fetch unread emails directly from Apple Mail's SQLite database
/// This is ~225x faster than AppleScript for large mailboxes
pub fn fetch_unread_emails_sqlite() -> Result<Vec<Email>, String> {
    log!("Fetching unread emails via SQLite (high-performance mode)...");
    let start = std::time::Instant::now();

    let db_path = find_mail_db_path()?;

    // Open in read-only mode to avoid any risk of corruption
    let conn = Connection::open_with_flags(&db_path, OpenFlags::SQLITE_OPEN_READ_ONLY)
        .map_err(|e| format!("Failed to open Mail database: {}. Ensure Full Disk Access is granted.", e))?;

    // Query unread messages with subject and sender info
    // The schema joins messages -> subjects and messages -> addresses
    let query = r#"
        SELECT 
            m.ROWID,
            m.message_id,
            COALESCE(subj.subject, '(No Subject)') as subject,
            COALESCE(addr.address, 'Unknown') as sender,
            m.date_received,
            COALESCE(mb.url, 'Inbox') as mailbox,
            COALESCE(mb.account_id, 0) as account_id
        FROM messages m
        LEFT JOIN subjects subj ON m.subject = subj.ROWID
        LEFT JOIN addresses addr ON m.sender = addr.ROWID
        LEFT JOIN mailboxes mb ON m.mailbox = mb.ROWID
        WHERE m.read = 0
          AND m.deleted = 0
        ORDER BY m.date_received DESC
    "#;

    let mut stmt = conn
        .prepare(query)
        .map_err(|e| format!("SQL prepare error: {}", e))?;

    let email_iter = stmt
        .query_map([], |row| {
            let rowid: i64 = row.get(0)?;
            let message_id: String = row.get::<_, Option<String>>(1)?.unwrap_or_default();
            let subject: String = row.get(2)?;
            let sender: String = row.get(3)?;
            let date_received: f64 = row.get(4)?;
            let mailbox_url: String = row.get(5)?;
            let account_id: i64 = row.get(6)?;

            // Convert Core Data timestamp (seconds since 2001-01-01) to readable date
            let date_str = format_core_data_timestamp(date_received);

            // Extract mailbox name from URL (e.g., "imap://...INBOX" -> "INBOX")
            let mailbox_name = extract_mailbox_name(&mailbox_url);

            Ok(Email {
                id: rowid.to_string(),
                message_id,
                subject,
                sender,
                date_received: date_str,
                mailbox: mailbox_name,
                account: format!("Account {}", account_id),
            })
        })
        .map_err(|e| format!("SQL query error: {}", e))?;

    let emails: Vec<Email> = email_iter.filter_map(|r| r.ok()).collect();

    log!(
        "SQLite fetch complete: {} unread emails in {:?}",
        emails.len(),
        start.elapsed()
    );
    Ok(emails)
}

/// Convert Core Data timestamp to human-readable date string
/// Core Data uses seconds since January 1, 2001 (Apple's reference date)
fn format_core_data_timestamp(timestamp: f64) -> String {
    // Core Data epoch: 2001-01-01 00:00:00 UTC
    // Unix epoch: 1970-01-01 00:00:00 UTC
    // Difference: 978307200 seconds
    const CORE_DATA_EPOCH_OFFSET: i64 = 978307200;

    let unix_timestamp = timestamp as i64 + CORE_DATA_EPOCH_OFFSET;

    // Format as ISO-like date string
    use std::time::{Duration, UNIX_EPOCH};
    let datetime = UNIX_EPOCH + Duration::from_secs(unix_timestamp as u64);

    // Simple formatting (we don't want to add chrono dependency)
    format!("{:?}", datetime)
}

/// Extract mailbox name from mailbox URL
/// e.g., "imap://user@imap.gmail.com/INBOX" -> "INBOX"
fn extract_mailbox_name(url: &str) -> String {
    url.rsplit('/')
        .next()
        .unwrap_or("Inbox")
        .to_string()
}

/// Fetch email body content by email ID and parse it
pub fn fetch_email_body(email_id: &str) -> Result<EmailBody, String> {
    log!("Fetching email body for ID: {}", email_id);
    let start = std::time::Instant::now();

    let script = format!(
        r#"
        tell application "Mail"
            set targetMsg to first message of inbox whose id is {}
            set msgContent to source of targetMsg
            return msgContent
        end tell
    "#,
        email_id
    );

    let raw_body = run_applescript(&script)?;

    // Parse the email with mail-parser
    let parser = MessageParser::default();
    let message = parser
        .parse(raw_body.as_bytes())
        .ok_or_else(|| "Failed to parse email".to_string())?;

    // Extract HTML and text parts
    let html = message.body_html(0).map(|s| s.to_string());
    let text = message.body_text(0).map(|s| s.to_string());

    log!("Fetched and parsed email body in {:?}", start.elapsed());

    Ok(EmailBody { html, text })
}
