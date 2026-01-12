mod cache;
mod filters;
mod mail;

use tauri::Manager;
use cache::{clear_cache, load_from_cache, save_to_cache};
use filters::{apply_filters, load_filters, save_filters, test_pattern, FilterConfig};
use mail::{fetch_unread_emails, fetch_unread_emails_inbox_only, mark_emails_as_read, Email, FilterField, FilterPattern};
use std::sync::Mutex;
use tauri::State;

/// Cache for emails to avoid re-fetching
struct EmailCache {
    emails: Mutex<Option<Vec<Email>>>,
}

/// Fetch emails, using file cache if available
async fn fetch_with_cache(inbox_only: bool, use_cache: bool) -> Result<Vec<Email>, String> {
    // Try file cache first (for dev/debug)
    if use_cache {
        if let Some(cached) = load_from_cache() {
            return Ok(cached);
        }
    }

    // Fetch from Mail.app
    let emails = tokio::task::spawn_blocking(move || {
        if inbox_only {
            fetch_unread_emails_inbox_only()
        } else {
            fetch_unread_emails()
        }
    })
    .await
    .map_err(|e| format!("Task error: {}", e))??;

    // Save to file cache
    let _ = save_to_cache(&emails);

    Ok(emails)
}

#[tauri::command]
async fn get_unread_emails(
    inbox_only: Option<bool>,
    use_cache: Option<bool>,
    cache: State<'_, EmailCache>,
) -> Result<Vec<Email>, String> {
    let inbox_only = inbox_only.unwrap_or(true);
    let use_cache_flag = use_cache.unwrap_or(true); // Use cache by default

    let emails = fetch_with_cache(inbox_only, use_cache_flag).await?;

    // Update memory cache
    *cache.emails.lock().unwrap() = Some(emails.clone());

    Ok(emails)
}

#[tauri::command]
async fn get_filtered_emails(
    inbox_only: Option<bool>,
    use_cache: Option<bool>,
    cache: State<'_, EmailCache>,
) -> Result<Vec<Email>, String> {
    let inbox_only = inbox_only.unwrap_or(true);
    let use_cache_flag = use_cache.unwrap_or(true);

    let emails = fetch_with_cache(inbox_only, use_cache_flag).await?;

    // Update memory cache
    *cache.emails.lock().unwrap() = Some(emails.clone());

    let config = load_filters()?;
    Ok(apply_filters(&emails, &config.patterns))
}

#[tauri::command]
async fn force_refresh(inbox_only: Option<bool>, cache: State<'_, EmailCache>) -> Result<Vec<Email>, String> {
    // Clear file cache and fetch fresh
    let _ = clear_cache();

    let inbox_only = inbox_only.unwrap_or(true);
    let emails = fetch_with_cache(inbox_only, false).await?;

    // Update memory cache
    *cache.emails.lock().unwrap() = Some(emails.clone());

    Ok(emails)
}

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

#[tauri::command]
async fn mark_as_read(email_ids: Vec<String>) -> Result<usize, String> {
    // Clear cache since read status changed
    let _ = clear_cache();

    // Run in blocking thread to not freeze UI
    tokio::task::spawn_blocking(move || mark_emails_as_read(email_ids))
        .await
        .map_err(|e| format!("Task error: {}", e))?
}

#[tauri::command]
async fn preview_pattern(
    pattern: String,
    field: FilterField,
    cache: State<'_, EmailCache>,
) -> Result<Vec<Email>, String> {
    // Use cached emails if available, otherwise fetch
    let emails = {
        let cached = cache.emails.lock().unwrap();
        cached.clone()
    };

    let emails = match emails {
        Some(e) => e,
        None => {
            let fetched = fetch_with_cache(true, true).await?;
            *cache.emails.lock().unwrap() = Some(fetched.clone());
            fetched
        }
    };

    test_pattern(&emails, &pattern, &field)
}

/// Apply filters to cached emails without re-fetching
#[tauri::command]
fn apply_filters_to_cache(cache: State<'_, EmailCache>) -> Result<Vec<Email>, String> {
    let emails = {
        let cached = cache.emails.lock().unwrap();
        cached.clone()
    };

    match emails {
        Some(emails) => {
            let config = load_filters()?;
            Ok(apply_filters(&emails, &config.patterns))
        }
        None => Ok(vec![]),
    }
}

/// Test a pattern against cached emails and return match count
#[tauri::command]
fn test_pattern_match_count(
    pattern: String,
    field: FilterField,
    is_regex: bool,
    cache: State<'_, EmailCache>,
) -> Result<TestPatternResult, String> {
    let emails = {
        let cached = cache.emails.lock().unwrap();
        cached.clone()
    };

    let emails = emails.unwrap_or_default();
    
    let matched = match_emails(&emails, &pattern, &field, is_regex)?;
    
    Ok(TestPatternResult {
        match_count: matched.len(),
        total_count: emails.len(),
        sample_matches: matched.into_iter().take(5).collect(),
    })
}

#[derive(serde::Serialize)]
struct TestPatternResult {
    match_count: usize,
    total_count: usize,
    sample_matches: Vec<Email>,
}

/// Match emails using either simple string or regex
fn match_emails(
    emails: &[Email],
    pattern: &str,
    field: &FilterField,
    is_regex: bool,
) -> Result<Vec<Email>, String> {
    if pattern.trim().is_empty() {
        return Ok(vec![]);
    }

    let matched: Vec<Email> = if is_regex {
        let regex = regex::Regex::new(pattern)
            .map_err(|e| format!("Invalid regex: {}", e))?;

        emails
            .iter()
            .filter(|email| match field {
                FilterField::Subject => regex.is_match(&email.subject),
                FilterField::Sender => regex.is_match(&email.sender),
                FilterField::Any => regex.is_match(&email.subject) || regex.is_match(&email.sender),
            })
            .cloned()
            .collect()
    } else {
        // Simple case-insensitive substring match
        let pattern_lower = pattern.to_lowercase();

        emails
            .iter()
            .filter(|email| match field {
                FilterField::Subject => email.subject.to_lowercase().contains(&pattern_lower),
                FilterField::Sender => email.sender.to_lowercase().contains(&pattern_lower),
                FilterField::Any => {
                    email.subject.to_lowercase().contains(&pattern_lower)
                        || email.sender.to_lowercase().contains(&pattern_lower)
                }
            })
            .cloned()
            .collect()
    };

    Ok(matched)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .manage(EmailCache {
            emails: Mutex::new(None),
        })
        .invoke_handler(tauri::generate_handler![
            get_unread_emails,
            get_filtered_emails,
            get_filters,
            save_filter_patterns,
            mark_as_read,
            preview_pattern,
            apply_filters_to_cache,
            force_refresh,
            test_pattern_match_count
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
