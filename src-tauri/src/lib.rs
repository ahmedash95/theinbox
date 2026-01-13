mod filters;
mod gmail;

use filters::{load_filters, save_filters, FilterConfig, FilterPattern};
use tauri::Manager;

#[tauri::command]
fn get_filters() -> Result<Vec<FilterPattern>, String> {
    let config = load_filters()?;
    Ok(config.patterns)
}

#[tauri::command]
fn save_filter_patterns(patterns: Vec<FilterPattern>) -> Result<(), String> {
    let config = FilterConfig { patterns };
    save_filters(&config)
}

// =============================================================================
// Gmail IMAP Commands (App Passwords)
// =============================================================================

/// Store Gmail credentials securely in macOS Keychain
#[tauri::command]
async fn gmail_store_credentials(email: String, app_password: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || gmail::store_credentials(&email, &app_password))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

/// Test Gmail connection without storing credentials
#[tauri::command]
async fn gmail_test_connection(email: String, app_password: String) -> Result<String, String> {
    tokio::task::spawn_blocking(move || gmail::test_connection(&email, &app_password))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

/// Check if Gmail account is configured
#[tauri::command]
fn gmail_is_configured(email: String) -> bool {
    gmail::has_credentials(&email)
}

/// Delete Gmail credentials from Keychain
#[tauri::command]
async fn gmail_delete_credentials(email: String) -> Result<(), String> {
    tokio::task::spawn_blocking(move || gmail::delete_credentials(&email))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

/// Fetch unread emails from Gmail via IMAP
#[tauri::command]
async fn gmail_fetch_unread(email: String) -> Result<Vec<gmail::GmailEmail>, String> {
    tokio::task::spawn_blocking(move || gmail::fetch_unread_emails(&email))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

/// Mark Gmail emails as read (batch operation)
#[tauri::command]
async fn gmail_mark_as_read(email: String, uids: Vec<u32>) -> Result<usize, String> {
    tokio::task::spawn_blocking(move || gmail::mark_emails_as_read(&email, uids))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

/// Fetch Gmail email body by UID
#[tauri::command]
async fn gmail_fetch_body(email: String, uid: u32) -> Result<gmail::EmailBody, String> {
    tokio::task::spawn_blocking(move || gmail::fetch_email_body(&email, uid))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .invoke_handler(tauri::generate_handler![
            get_filters,
            save_filter_patterns,
            // Gmail IMAP commands
            gmail_store_credentials,
            gmail_test_connection,
            gmail_is_configured,
            gmail_delete_credentials,
            gmail_fetch_unread,
            gmail_mark_as_read,
            gmail_fetch_body
        ])
        .setup(|app| {
            let window = app.get_webview_window("main").unwrap();

            #[cfg(target_os = "macos")]
            {
                use window_vibrancy::{apply_vibrancy, NSVisualEffectMaterial};
                apply_vibrancy(&window, NSVisualEffectMaterial::HudWindow, None, None)
                    .expect("Unsupported platform! 'apply_vibrancy' is only supported on macOS");
            }

            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
