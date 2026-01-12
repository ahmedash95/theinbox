use serde::{Deserialize, Serialize};
use std::process::Command;

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
