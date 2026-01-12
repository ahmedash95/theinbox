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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct FilterPattern {
    #[serde(default, deserialize_with = "deserialize_filter_id")]
    pub id: i64,
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

fn deserialize_filter_id<'de, D>(deserializer: D) -> Result<i64, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let value = serde_json::Value::deserialize(deserializer)?;
    match value {
        serde_json::Value::Number(number) => Ok(number.as_i64().unwrap_or(0)),
        serde_json::Value::String(value) => Ok(value.parse::<i64>().unwrap_or(0)),
        _ => Ok(0),
    }
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

    let config: FilterConfig = serde_json::from_str(&content)
        .map_err(|e| format!("Failed to parse filters file: {}", e))?;
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
