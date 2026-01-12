use crate::mail::Email;
use std::fs;
use std::path::PathBuf;
use std::time::{Duration, SystemTime};

const CACHE_FILE: &str = "email_cache.json";
const CACHE_MAX_AGE_SECS: u64 = 3600; // 1 hour 

/// Log a message to stdout for debugging
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[InboxCleanup] {}", format!($($arg)*));
    };
}

#[derive(serde::Serialize, serde::Deserialize)]
struct CacheData {
    timestamp: u64,
    emails: Vec<Email>,
}

fn get_cache_path() -> Result<PathBuf, String> {
    let cache_dir = dirs::cache_dir()
        .ok_or_else(|| "Could not find cache directory".to_string())?
        .join("InboxCleanup");

    fs::create_dir_all(&cache_dir)
        .map_err(|e| format!("Failed to create cache directory: {}", e))?;

    Ok(cache_dir.join(CACHE_FILE))
}

fn current_timestamp() -> u64 {
    SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap_or(Duration::ZERO)
        .as_secs()
}

/// Try to load emails from cache (if cache is fresh)
pub fn load_from_cache() -> Option<Vec<Email>> {
    let path = get_cache_path().ok()?;
    
    if !path.exists() {
        log!("No cache file found");
        return None;
    }

    let content = fs::read_to_string(&path).ok()?;
    let cache: CacheData = serde_json::from_str(&content).ok()?;

    let age = current_timestamp().saturating_sub(cache.timestamp);
    
    if age > CACHE_MAX_AGE_SECS {
        log!("Cache expired (age: {}s, max: {}s)", age, CACHE_MAX_AGE_SECS);
        return None;
    }

    log!("Loaded {} emails from cache (age: {}s)", cache.emails.len(), age);
    Some(cache.emails)
}

/// Save emails to cache
pub fn save_to_cache(emails: &[Email]) -> Result<(), String> {
    let path = get_cache_path()?;

    let cache = CacheData {
        timestamp: current_timestamp(),
        emails: emails.to_vec(),
    };

    let content = serde_json::to_string_pretty(&cache)
        .map_err(|e| format!("Failed to serialize cache: {}", e))?;

    fs::write(&path, content).map_err(|e| format!("Failed to write cache: {}", e))?;

    log!("Saved {} emails to cache", emails.len());
    Ok(())
}

/// Force clear the cache
pub fn clear_cache() -> Result<(), String> {
    let path = get_cache_path()?;
    
    if path.exists() {
        fs::remove_file(&path).map_err(|e| format!("Failed to clear cache: {}", e))?;
        log!("Cache cleared");
    }
    
    Ok(())
}
