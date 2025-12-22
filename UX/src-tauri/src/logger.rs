/// Logger module for AI agent access
/// Writes structured logs to file for debugging and testing

use std::fs::{File, OpenOptions};
use std::io::{Write, BufWriter};
use std::path::PathBuf;
use chrono::Utc;
use serde_json::json;

pub struct AppLogger {
    log_file: BufWriter<File>,
}

impl AppLogger {
    /// Create new logger with file at given path
    pub fn new(log_path: PathBuf) -> std::io::Result<Self> {
        // Ensure parent directory exists
        if let Some(parent) = log_path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(log_path)?;
        
        Ok(Self {
            log_file: BufWriter::new(file),
        })
    }
    
    /// Write a log entry (JSON Lines format)
    pub fn log(
        &mut self,
        level: &str,
        source: &str,
        message: &str,
        details: Option<serde_json::Value>,
    ) -> std::io::Result<()> {
        let timestamp = Utc::now().to_rfc3339();
        
        let entry = if let Some(details) = details {
            json!({
                "timestamp": timestamp,
                "level": level,
                "source": source,
                "message": message,
                "details": details,
            })
        } else {
            json!({
                "timestamp": timestamp,
                "level": level,
                "source": source,
                "message": message,
            })
        };
        
        // Write JSON line
        writeln!(&mut self.log_file, "{}", entry)?;
        self.log_file.flush()?;
        
        Ok(())
    }
}
