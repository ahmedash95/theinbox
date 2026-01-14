mod filters;
mod gmail;
mod storage;

use filters::FilterPattern;
use std::sync::Arc;
use tauri::AppHandle;
use tauri::Emitter;
use tauri::Manager;
use tauri::State;
use tokio::sync::mpsc;
use std::collections::HashSet;

struct AppState {
    storage: Arc<dyn storage::Storage>,
    syncing: Arc<tokio::sync::Mutex<HashSet<String>>>,
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
        storage.upsert_emails(&email, "INBOX", &emails)?;
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
        let _ = handle.emit(
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
            storage.upsert_emails(&email, "INBOX", &emails)?;
            Ok::<usize, String>(emails.len())
        })
        .await;

        match result {
            Ok(Ok(count)) => {
                let _ = handle.emit(
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
                let _ = handle.emit(
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
                let _ = handle.emit(
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

/// Run IMAP fetch for all emails in the background and emit progress events.
#[tauri::command]
async fn gmail_sync_all_background(
    app: AppHandle,
    state: State<'_, AppState>,
    email: String,
) -> Result<(), String> {
    let storage = state.storage.clone();
    let syncing = state.syncing.clone();
    let handle = app.clone();

    {
        let mut guard = syncing.lock().await;
        if guard.contains(&email) {
            println!("[InboxCleanup] Sync already running for {}", email);
            return Ok(());
        }
        guard.insert(email.clone());
    }

    tokio::spawn(async move {
        println!("[InboxCleanup] Background sync started for {}", email);
        let _ = handle.emit(
            "imap_sync_progress",
            SyncProgress {
                stage: "start".to_string(),
                processed: 0,
                total: 0,
                message: None,
            },
        );

        let (tx, mut rx) = mpsc::unbounded_channel::<(usize, usize)>();
        let progress_handle = handle.clone();
        let progress_task = tokio::spawn(async move {
            while let Some((processed, total)) = rx.recv().await {
                println!(
                    "[InboxCleanup] Sync progress: {}/{} ({:.0}%)",
                    processed,
                    total,
                    if total > 0 {
                        (processed as f64 / total as f64) * 100.0
                    } else {
                        0.0
                    }
                );
                let _ = progress_handle.emit(
                    "imap_sync_progress",
                    SyncProgress {
                        stage: "progress".to_string(),
                        processed,
                        total,
                        message: None,
                    },
                );
            }
        });

        let storage_for_sync = storage.clone();
        let email_for_sync = email.clone();
        let result = tokio::task::spawn_blocking(move || {
            let mut last_uid = storage_for_sync.get_last_uid(&email_for_sync)?;
            if last_uid == 0 {
                if let Ok(Some(max_uid)) = storage_for_sync.get_max_uid(&email_for_sync) {
                    let _ = storage_for_sync.set_last_uid(&email_for_sync, max_uid);
                    last_uid = max_uid;
                }
            }
            println!(
                "[InboxCleanup] Sync starting from last UID {} (batch size: 1000)",
                last_uid
            );
            gmail::fetch_emails_since(&email_for_sync, last_uid, 1000, 500, |chunk| {
                let _ = storage_for_sync.upsert_emails(&email_for_sync, "INBOX", &chunk.emails);
                let _ = storage_for_sync.set_email_bodies(&email_for_sync, &chunk.bodies);
                if let Some(max_uid) = chunk.emails.iter().map(|email| email.uid).max() {
                    let _ = storage_for_sync.set_last_uid(&email_for_sync, max_uid);
                }
                let _ = tx.send((chunk.processed, chunk.total));
            })
        })
        .await;

        drop(progress_task);

        match result {
            Ok(Ok((count, max_uid))) => {
                if let Some(max_uid) = max_uid {
                    let _ = storage.set_last_uid(&email, max_uid);
                } else if let Ok(Some(max_uid)) = storage.get_max_uid(&email) {
                    let _ = storage.set_last_uid(&email, max_uid);
                }
                println!("[InboxCleanup] Background sync complete ({} emails)", count);
                let _ = handle.emit(
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
                println!("[InboxCleanup] Background sync failed: {}", err);
                let _ = handle.emit(
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
                println!("[InboxCleanup] Background sync task error: {}", err);
                let _ = handle.emit(
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

        let mut guard = syncing.lock().await;
        guard.remove(&email);
    });

    Ok(())
}

/// List cached emails from SQLite
#[tauri::command]
fn gmail_list_cached_unread(
    state: State<AppState>,
    email: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<storage::StoredEmail>, String> {
    state.storage.list_emails(&email, true, limit, offset)
}

#[tauri::command]
fn gmail_list_cached_all(
    state: State<AppState>,
    email: String,
    limit: u32,
    offset: u32,
) -> Result<Vec<storage::StoredEmail>, String> {
    state.storage.list_emails(&email, false, limit, offset)
}

#[derive(serde::Serialize)]
struct EmailCounts {
    total: u64,
    unread: u64,
}

#[tauri::command]
fn gmail_cached_counts(state: State<AppState>, email: String) -> Result<EmailCounts, String> {
    let total = state.storage.count_emails(&email, false)?;
    let unread = state.storage.count_emails(&email, true)?;
    Ok(EmailCounts { total, unread })
}

#[tauri::command]
fn get_db_directory() -> Result<String, String> {
    storage::get_db_dir()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| e)
}

#[tauri::command]
fn get_db_file_path() -> Result<String, String> {
    storage::get_db_file_path()
        .map(|path| path.to_string_lossy().to_string())
        .map_err(|e| e)
}

/// Fetch Gmail email body by UID
#[tauri::command]
async fn gmail_fetch_body(
    state: State<'_, AppState>,
    email: String,
    uid: u32,
) -> Result<gmail::EmailBody, String> {
    let storage = state.storage.clone();
    tokio::task::spawn_blocking(move || {
        if let Some(body) = storage.get_email_body(&email, uid)? {
            return Ok(body);
        }
        let body = gmail::fetch_email_body(&email, uid)?;
        storage.set_email_bodies(
            &email,
            &[gmail::GmailEmailBody { uid, body: body.clone() }],
        )?;
        Ok(body)
    })
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
            gmail_sync_all_background,
            gmail_list_cached_unread,
            gmail_list_cached_all,
            gmail_cached_counts,
            get_db_directory,
            get_db_file_path
        ])
        .setup(|app| {
            let storage = storage::SqliteStorage::new().map_err(|e| {
                std::io::Error::new(std::io::ErrorKind::Other, format!("Storage init failed: {}", e))
            })?;
            app.manage(AppState {
                storage: Arc::new(storage),
                syncing: Arc::new(tokio::sync::Mutex::new(HashSet::new())),
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
