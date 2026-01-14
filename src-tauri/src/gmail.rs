//! Gmail IMAP Module - High-performance email access via App Passwords
//!
//! Uses direct IMAP connections instead of OAuth for simplicity and speed.
//! Credentials are stored securely in the macOS Keychain.

use imap::Session;
use native_tls::TlsStream;
use security_framework::passwords::{delete_generic_password, get_generic_password, set_generic_password};
use serde::{Deserialize, Serialize};
use std::net::TcpStream;
use base64::engine::general_purpose;
use base64::Engine;
use mail_parser::MessageParser;

const KEYCHAIN_SERVICE: &str = "com.inboxcleanup.gmail";
const IMAP_HOST: &str = "imap.gmail.com";
const IMAP_PORT: u16 = 993;

/// Log a message to stdout for debugging
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[InboxCleanup:Gmail] {}", format!($($arg)*));
    };
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GmailEmail {
    pub uid: u32,
    pub message_id: String,
    pub subject: String,
    pub sender: String,
    pub date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EmailBody {
    pub html: Option<String>,
    pub text: Option<String>,
}


// =============================================================================
// Keychain Operations
// =============================================================================

/// Store Gmail credentials in the macOS Keychain
pub fn store_credentials(email: &str, app_password: &str) -> Result<(), String> {
    log!("Storing credentials for {} in Keychain", email);
    
    // Store the password with email as the account name
    set_generic_password(KEYCHAIN_SERVICE, email, app_password.as_bytes())
        .map_err(|e| format!("Failed to store in Keychain: {}", e))?;
    
    log!("Credentials stored successfully");
    Ok(())
}

/// Retrieve Gmail credentials from the macOS Keychain
pub fn get_credentials(email: &str) -> Result<String, String> {
    let password_bytes = get_generic_password(KEYCHAIN_SERVICE, email)
        .map_err(|e| format!("Failed to retrieve from Keychain: {}", e))?;
    
    String::from_utf8(password_bytes.to_vec())
        .map_err(|e| format!("Invalid password encoding: {}", e))
}

/// Delete Gmail credentials from the macOS Keychain
pub fn delete_credentials(email: &str) -> Result<(), String> {
    log!("Deleting credentials for {} from Keychain", email);
    
    delete_generic_password(KEYCHAIN_SERVICE, email)
        .map_err(|e| format!("Failed to delete from Keychain: {}", e))?;
    
    log!("Credentials deleted successfully");
    Ok(())
}

/// Check if credentials exist for an email
pub fn has_credentials(email: &str) -> bool {
    get_generic_password(KEYCHAIN_SERVICE, email).is_ok()
}

// =============================================================================
// IMAP Connection
// =============================================================================

/// Create an authenticated IMAP session
fn connect_imap(email: &str, app_password: &str) -> Result<Session<TlsStream<TcpStream>>, String> {
    log!("Connecting to {} for {}...", IMAP_HOST, email);
    
    let tls = native_tls::TlsConnector::new()
        .map_err(|e| format!("TLS error: {}", e))?;
    
    let client = imap::connect((IMAP_HOST, IMAP_PORT), IMAP_HOST, &tls)
        .map_err(|e| format!("Connection failed: {}", e))?;
    
    let session = client
        .login(email, app_password)
        .map_err(|e| format!("Login failed: {}. Ensure you're using an App Password (not your regular password). Generate one at myaccount.google.com/apppasswords", e.0))?;
    
    log!("Connected successfully");
    Ok(session)
}

// =============================================================================
// Email Operations
// =============================================================================

/// Fetch unread emails from Gmail inbox via IMAP
/// This is much faster than OAuth-based approaches
pub fn fetch_unread_emails(email: &str) -> Result<Vec<GmailEmail>, String> {
    let app_password = get_credentials(email)?;
    
    log!("Fetching unread emails for {}...", email);
    let start = std::time::Instant::now();
    
    let mut session = connect_imap(email, &app_password)?;
    
    // Select INBOX
    session.select("INBOX")
        .map_err(|e| format!("Failed to select INBOX: {}", e))?;
    
    // Search for unread messages (returns UIDs)
    let uids = session.uid_search("UNSEEN")
        .map_err(|e| format!("Search failed: {}", e))?;
    
    if uids.is_empty() {
        log!("No unread emails found");
        session.logout().ok();
        return Ok(vec![]);
    }
    
    log!("Found {} unread emails, fetching headers...", uids.len());
    
    // Build UID sequence for batch fetch
    let uid_list: Vec<String> = uids.iter().map(|u| u.to_string()).collect();
    let uid_sequence = uid_list.join(",");
    
    // Fetch headers for all unread messages in one request
    let messages = session.uid_fetch(&uid_sequence, "(UID ENVELOPE)")
        .map_err(|e| format!("Fetch failed: {}", e))?;
    
    let emails: Vec<GmailEmail> = messages
        .iter()
        .filter_map(|msg| {
            let uid = msg.uid?;
            let envelope = msg.envelope()?;
            
            let subject = envelope.subject
                .map(|s| decode_mime_header(s))
                .unwrap_or_else(|| "(No Subject)".to_string());
            
            let sender = envelope.from
                .as_ref()
                .and_then(|addrs| addrs.first())
                .map(|addr| {
                    let mailbox = addr.mailbox
                        .map(|m| String::from_utf8_lossy(m).to_string())
                        .unwrap_or_default();
                    let host = addr.host
                        .map(|h| String::from_utf8_lossy(h).to_string())
                        .unwrap_or_default();
                    let email = if mailbox.is_empty() || host.is_empty() {
                        String::new()
                    } else {
                        format!("{}@{}", mailbox, host)
                    };
                    let name = addr.name
                        .map(|n| decode_mime_header(n))
                        .unwrap_or_default();

                    if !name.is_empty() && !email.is_empty() {
                        format!("{} <{}>", name, email)
                    } else if !email.is_empty() {
                        email
                    } else {
                        "Unknown".to_string()
                    }
                })
                .unwrap_or_else(|| "Unknown".to_string());
            
            let date = envelope.date
                .map(|d| String::from_utf8_lossy(d).to_string())
                .unwrap_or_default();
            
            let message_id = envelope.message_id
                .map(|m| String::from_utf8_lossy(m).to_string())
                .unwrap_or_default();
            
            Some(GmailEmail {
                uid,
                message_id,
                subject,
                sender,
                date,
            })
        })
        .collect();
    
    session.logout().ok();
    
    log!("Fetched {} emails in {:?}", emails.len(), start.elapsed());
    Ok(emails)
}

/// Mark emails as read using batch IMAP STORE command
/// This is O(1) network request vs O(n) for individual updates
pub fn mark_emails_as_read(email: &str, uids: Vec<u32>) -> Result<usize, String> {
    if uids.is_empty() {
        return Ok(0);
    }
    
    let app_password = get_credentials(email)?;
    
    log!("Marking {} emails as read for {}...", uids.len(), email);
    let start = std::time::Instant::now();
    
    let mut session = connect_imap(email, &app_password)?;
    
    session.select("INBOX")
        .map_err(|e| format!("Failed to select INBOX: {}", e))?;
    
    // Build UID sequence for batch operation
    let uid_list: Vec<String> = uids.iter().map(|u| u.to_string()).collect();
    let uid_sequence = uid_list.join(",");
    
    // Single STORE command to mark all as read
    session.uid_store(&uid_sequence, "+FLAGS (\\Seen)")
        .map_err(|e| format!("Failed to mark as read: {}", e))?;
    
    session.logout().ok();
    
    let count = uids.len();
    log!("Marked {} emails as read in {:?}", count, start.elapsed());
    Ok(count)
}

/// Test connection with provided credentials (without storing)
pub fn test_connection(email: &str, app_password: &str) -> Result<String, String> {
    log!("Testing connection for {}...", email);
    
    let mut session = connect_imap(email, app_password)?;
    
    // Get mailbox info
    let mailbox = session.select("INBOX")
        .map_err(|e| format!("Failed to select INBOX: {}", e))?;
    
    let message_count = mailbox.exists;
    
    session.logout().ok();
    
    Ok(format!("Connection successful! Inbox has {} messages.", message_count))
}

// =============================================================================
// Helpers
// =============================================================================

/// Decode MIME encoded header (basic implementation)
fn decode_mime_header(bytes: &[u8]) -> String {
    let input = String::from_utf8_lossy(bytes).to_string();
    decode_rfc2047_words(&input)
}

fn decode_rfc2047_words(input: &str) -> String {
    let mut output = String::new();
    let mut index = 0;

    while let Some(start_rel) = input[index..].find("=?") {
        let start = index + start_rel;
        output.push_str(&input[index..start]);

        let rest = &input[start + 2..];
        let Some(q1) = rest.find('?') else {
            output.push_str("=?");
            index = start + 2;
            continue;
        };
        let charset = &rest[..q1];
        let rest = &rest[q1 + 1..];
        let Some(q2) = rest.find('?') else {
            output.push_str("=?");
            index = start + 2;
            continue;
        };
        let encoding = &rest[..q2];
        let rest = &rest[q2 + 1..];
        let Some(q3) = rest.find("?=") else {
            output.push_str("=?");
            index = start + 2;
            continue;
        };
        let encoded = &rest[..q3];

        let decoded = decode_encoded_word(charset, encoding, encoded);
        output.push_str(&decoded);
        index = start + 2 + q1 + 1 + q2 + 1 + q3 + 2;
    }

    output.push_str(&input[index..]);
    output
}

fn decode_encoded_word(charset: &str, encoding: &str, encoded: &str) -> String {
    let bytes = match encoding.to_ascii_lowercase().as_str() {
        "q" => decode_q(encoded),
        "b" => decode_b(encoded),
        _ => encoded.as_bytes().to_vec(),
    };

    match charset.to_ascii_lowercase().as_str() {
        "utf-8" | "utf8" => String::from_utf8_lossy(&bytes).to_string(),
        _ => String::from_utf8_lossy(&bytes).to_string(),
    }
}

fn decode_q(encoded: &str) -> Vec<u8> {
    let mut out = Vec::with_capacity(encoded.len());
    let bytes = encoded.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        match bytes[i] {
            b'_' => out.push(b' '),
            b'=' if i + 2 < bytes.len() => {
                if let (Some(hi), Some(lo)) = (hex_val(bytes[i + 1]), hex_val(bytes[i + 2])) {
                    out.push((hi << 4) | lo);
                    i += 3;
                    continue;
                } else {
                    out.push(bytes[i]);
                }
            }
            b => out.push(b),
        }
        i += 1;
    }
    out
}

fn decode_b(encoded: &str) -> Vec<u8> {
    general_purpose::STANDARD
        .decode(encoded.as_bytes())
        .unwrap_or_else(|_| encoded.as_bytes().to_vec())
}

fn hex_val(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        _ => None,
    }
}

/// Fetch email body by UID and parse it properly
pub fn fetch_email_body(email: &str, uid: u32) -> Result<EmailBody, String> {
    let app_password = get_credentials(email)?;

    log!("Fetching email body for UID {} from {}...", uid, email);
    let start = std::time::Instant::now();

    let mut session = connect_imap(email, &app_password)?;

    session.select("INBOX")
        .map_err(|e| format!("Failed to select INBOX: {}", e))?;

    // Fetch the full message body (BODY[] gets the full message content)
    let messages = session.uid_fetch(uid.to_string(), "BODY[]")
        .map_err(|e| format!("Failed to fetch body: {}", e))?;

    let raw_body = messages
        .iter()
        .next()
        .and_then(|msg| msg.body())
        .ok_or_else(|| "Could not retrieve email body".to_string())?;

    session.logout().ok();

    // Parse the email with mail-parser
    let parser = MessageParser::default();
    let message = parser
        .parse(raw_body)
        .ok_or_else(|| "Failed to parse email".to_string())?;

    // Extract HTML and text parts
    let html = message.body_html(0).map(|s| s.to_string());
    let text = message.body_text(0).map(|s| s.to_string());

    log!("Fetched and parsed email body in {:?}", start.elapsed());

    Ok(EmailBody { html, text })
}
