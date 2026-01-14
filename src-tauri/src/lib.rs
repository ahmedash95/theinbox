mod filters;
mod gmail;
mod storage;

use filters::FilterPattern;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Manager;
use tauri::State;

struct AppState {
    storage: Arc<dyn storage::Storage>,
}

#[derive(serde::Serialize, Clone)]
struct SyncProgress {
    stage: String,
    processed: usize,
    total: usize,
    message: Option<String>,
}

#[tauri::command]
fn get_filters(state: State<AppState>) -> Result<Vec<FilterPattern>, String> {
    state.storage.get_filters()
}

#[tauri::command]
fn save_filter_patterns(state: State<AppState>, patterns: Vec<FilterPattern>) -> Result<(), String> {
    state.storage.save_filters(&patterns)
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
async fn gmail_fetch_unread(
    state: State<'_, AppState>,
    email: String,
) -> Result<Vec<gmail::GmailEmail>, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        let emails = gmail::fetch_unread_emails(&email)?;
        storage.upsert_emails(&email, "INBOX", &emails, false)?;
        Ok(emails)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Mark Gmail emails as read (batch operation)
#[tauri::command]
async fn gmail_mark_as_read(
    state: State<'_, AppState>,
    email: String,
    uids: Vec<u32>,
) -> Result<usize, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        let count = gmail::mark_emails_as_read(&email, uids.clone())?;
        storage.mark_emails_read(&email, &uids)?;
        Ok(count)
    })
    .await
    .map_err(|e| format!("Task error: {}", e))?
}

/// Run IMAP fetch in the background and emit progress events.
#[tauri::command]
async fn gmail_sync_unread_background(
    app: AppHandle,
    state: State<'_, AppState>,
    email: String,
) -> Result<(), String> {
    let storage = state.storage.clone();
    let handle = app.clone();
    tokio::spawn(async move {
        let _ = handle.emit_all(
            "imap_sync_progress",
            SyncProgress {
                stage: "start".to_string(),
                processed: 0,
                total: 0,
                message: None,
            },
        );

        let result = tokio::task::spawn_blocking(move || {
            let emails = gmail::fetch_unread_emails(&email)?;
            storage.upsert_emails(&email, "INBOX", &emails, false)?;
            Ok::<usize, String>(emails.len())
        })
        .await;

        match result {
            Ok(Ok(count)) => {
                let _ = handle.emit_all(
                    "imap_sync_progress",
                    SyncProgress {
                        stage: "complete".to_string(),
                        processed: count,
                        total: count,
                        message: None,
                    },
                );
            }
            Ok(Err(err)) => {
                let _ = handle.emit_all(
                    "imap_sync_progress",
                    SyncProgress {
                        stage: "error".to_string(),
                        processed: 0,
                        total: 0,
                        message: Some(err),
                    },
                );
            }
            Err(err) => {
                let _ = handle.emit_all(
                    "imap_sync_progress",
                    SyncProgress {
                        stage: "error".to_string(),
                        processed: 0,
                        total: 0,
                        message: Some(format!("Task error: {}", err)),
                    },
                );
            }
        }
    });

    Ok(())
}

/// List cached emails from SQLite
#[tauri::command]
fn gmail_list_cached_unread(
    state: State<AppState>,
    email: String,
) -> Result<Vec<storage::StoredEmail>, String> {
    state.storage.list_emails(&email, true)
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
            gmail_fetch_body,
            gmail_sync_unread_background,
            gmail_list_cached_unread
        ])
        .setup(|app| {
            let storage = storage::SqliteStorage::new().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("Storage init failed: {}", e))
            })?;
            app.manage(AppState {
                storage: Arc::new(storage),
            });
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
