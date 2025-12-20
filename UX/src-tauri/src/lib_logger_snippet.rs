mod db;
mod logger;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use rusqlite::Error as SqlError;

use db::{Database, FontRecord, Collection, create_font_record};
use logger::AppLogger;

/// Font entry in library
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontEntry {
    id: String,
    name: String,
    file_path: String,
    category: Option<String>,
    date_added: String,
}

/// Font file info for scanning
#[derive(Debug, Serialize, Deserialize)]
pub struct FontFileInfo {
    path: String,
    name: String,
    size: u64,
    is_local: bool, // true if file is available locally, false if in cloud
}

/// Log entry from frontend
#[derive(Debug, Serialize, Deserialize)]
pub struct LogEntry {
    level: String,
    message: String,
    details: Option<serde_json::Value>,
}

/// Write log entry to file
#[tauri::command]
fn write_log(app: tauri::AppHandle, entry: LogEntry) -> Result<(), String> {
    let logger = app.state::<Mutex<AppLogger>>();
    let logger = logger.lock().map_err(|e| format!("Logger lock error: {}", e))?;
    
    logger.log(&entry.level, "frontend", &entry.message, entry.details)
        .map_err(|e| format!("Failed to write log: {}", e))
}
