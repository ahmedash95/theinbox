use crate::mail::{Email, FilterField, FilterPattern};
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::fs;
use std::path::PathBuf;

/// Log a message to stdout for debugging
macro_rules! log {
    ($($arg:tt)*) => {
        println!("[InboxCleanup] {}", format!($($arg)*));
    };
}

#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct FilterConfig {
    pub patterns: Vec<FilterPattern>,
}

/// Get the path to the filters config file
fn get_config_path() -> Result<PathBuf, String> {
    let config_dir = dirs::config_dir()
        .ok_or_else(|| "Could not find config directory".to_string())?
        .join("InboxCleanup");

    fs::create_dir_all(&config_dir)
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    Ok(config_dir.join("filters.json"))
}

/// Load filters from disk
pub fn load_filters() -> Result<FilterConfig, String> {
    log!("Loading filters from disk...");
    let path = get_config_path()?;

    if !path.exists() {
        log!("No filters file found, using defaults");
        return Ok(FilterConfig::default());
    }

    let content =
        fs::read_to_string(&path).map_err(|e| format!("Failed to read filters file: {}", e))?;

    let config: FilterConfig = serde_json::from_str(&content).map_err(|e| format!("Failed to parse filters file: {}", e))?;
    log!("Loaded {} filters", config.patterns.len());
    Ok(config)
}

/// Save filters to disk
pub fn save_filters(config: &FilterConfig) -> Result<(), String> {
    let path = get_config_path()?;

    let content = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize filters: {}", e))?;

    fs::write(&path, content).map_err(|e| format!("Failed to write filters file: {}", e))
}

/// Check if an email matches a single pattern
fn email_matches_pattern(email: &Email, pattern: &FilterPattern) -> bool {
    let matches_field = |text: &str| -> bool {
        if pattern.is_regex {
            // Regex matching
            match Regex::new(&pattern.pattern) {
                Ok(regex) => regex.is_match(text),
                Err(_) => false,
            }
        } else {
            // Simple case-insensitive substring match
            text.to_lowercase().contains(&pattern.pattern.to_lowercase())
        }
    };

    match pattern.field {
        FilterField::Subject => matches_field(&email.subject),
        FilterField::Sender => matches_field(&email.sender),
        FilterField::Any => matches_field(&email.subject) || matches_field(&email.sender),
    }
}

/// Apply filters to a list of emails, returning only those that match any enabled pattern
pub fn apply_filters(emails: &[Email], patterns: &[FilterPattern]) -> Vec<Email> {
    let enabled_patterns: Vec<_> = patterns.iter().filter(|p| p.enabled).collect();
    log!(
        "Applying {} enabled filters to {} emails",
        enabled_patterns.len(),
        emails.len()
    );

    if enabled_patterns.is_empty() {
        log!("No enabled filters, returning empty list");
        return vec![];
    }

    let result: Vec<Email> = emails
        .iter()
        .filter(|email| {
            enabled_patterns
                .iter()
                .any(|pattern| email_matches_pattern(email, pattern))
        })
        .cloned()
        .collect();

    log!("Filters matched {} emails", result.len());
    result
}

/// Test a single pattern against emails to preview matches
pub fn test_pattern(
    emails: &[Email],
    pattern: &str,
    field: &FilterField,
) -> Result<Vec<Email>, String> {
    let regex = Regex::new(pattern).map_err(|e| format!("Invalid regex: {}", e))?;

    let matched: Vec<Email> = emails
        .iter()
        .filter(|email| match field {
            FilterField::Subject => regex.is_match(&email.subject),
            FilterField::Sender => regex.is_match(&email.sender),
            FilterField::Any => regex.is_match(&email.subject) || regex.is_match(&email.sender),
        })
        .cloned()
        .collect();

    Ok(matched)
}
