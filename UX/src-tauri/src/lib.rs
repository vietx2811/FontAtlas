mod db;
mod logger;
mod qdrant;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, Manager, State};
use rusqlite::Error as SqlError;
use uuid::Uuid;
use chrono::Utc;

use db::{Database, FontRecord, Collection, Source, create_font_record};
use logger::AppLogger;

/// Helper to log commands to file
fn log_to_file(app: &tauri::AppHandle, level: &str, source: &str, message: &str, details: Option<serde_json::Value>) {
    let logger_state = app.state::<Mutex<AppLogger>>();
    if let Ok(mut logger) = logger_state.lock() {
        let _ = logger.log(level, source, message, details);
    };
}


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

/// Scan a directory for font files
#[tauri::command]
fn scan_directory(path: String) -> Result<Vec<FontFileInfo>, String> {
    let dir_path = PathBuf::from(&path);
    
    if !dir_path.exists() {
        return Err(format!("Directory does not exist: {}", path));
    }
    
    let mut fonts = Vec::new();
    
    fn scan_recursive(dir: &PathBuf, fonts: &mut Vec<FontFileInfo>) -> std::io::Result<()> {
        if dir.is_dir() {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                
                if path.is_dir() {
                    // Skip .icloud bundle directories
                    if let Some(name) = path.file_name() {
                        let name_str = name.to_string_lossy();
                        if name_str.starts_with(".") && name_str.ends_with(".icloud") {
                            continue;
                        }
                    }
                    scan_recursive(&path, fonts)?;
                } else if let Some(ext) = path.extension() {
                    let ext_lower = ext.to_string_lossy().to_lowercase();
                    if ext_lower == "ttf" || ext_lower == "otf" || ext_lower == "woff" || ext_lower == "woff2" {
                        if let Some(name) = path.file_name() {
                            let metadata = entry.metadata()?;
                            let file_size = metadata.len();
                            
                            // Check if file is actually available locally
                            // iCloud placeholder files often have 0 bytes or are very small
                            // Also check if we can actually read the file
                            let is_local = file_size > 100 && fs::File::open(&path).is_ok();
                            
                            fonts.push(FontFileInfo {
                                path: path.to_string_lossy().to_string(),
                                name: name.to_string_lossy().to_string(),
                                size: file_size,
                                is_local,
                            });
                        }
                    }
                }
            }
        }
        Ok(())
    }
    
    scan_recursive(&dir_path, &mut fonts)
        .map_err(|e| format!("Error scanning directory: {}", e))?;
    
    Ok(fonts)
}

/// Open a folder dialog to select a directory
#[tauri::command]
async fn open_folder_dialog(app: tauri::AppHandle) -> Result<Option<String>, String> {
    use tauri_plugin_dialog::DialogExt;
    use std::sync::mpsc;
    
    let (tx, rx) = mpsc::channel();
    
    app.dialog()
        .file()
        .set_title("Select Font Directory")
        .pick_folder(move |result| {
            let _ = tx.send(result.map(|p| p.to_string()));
        });
    
    // Wait for the dialog result
    match rx.recv() {
        Ok(Some(path)) => Ok(Some(path)),
        Ok(None) => Ok(None),
        Err(_) => Err("Dialog cancelled".to_string())
    }
}

/// Open a file dialog to select font files
#[tauri::command]
async fn open_font_dialog() -> Result<Vec<String>, String> {
    // TODO: Implement with tauri-plugin-dialog when needed
    // For now, return empty - user can use scan_directory instead
    Ok(vec![])
}

/// Get app data directory
#[tauri::command]
fn get_app_data_dir(app: tauri::AppHandle) -> Result<String, String> {
    app.path()
        .app_data_dir()
        .map(|p| p.to_string_lossy().to_string())
        .map_err(|e| format!("Failed to get app data dir: {}", e))
}

/// Write log entry to file for AI agent access
#[tauri::command]
fn write_log(app: tauri::AppHandle, entry: LogEntry) -> Result<(), String> {
    let logger = app.state::<Mutex<AppLogger>>();
    let mut logger = logger.lock().map_err(|e| format!("Logger lock error: {}", e))?;
    
    logger.log(&entry.level, "frontend", &entry.message, entry.details)
        .map_err(|e| format!("Failed to write log: {}", e))
}

/// Read file as bytes (for font preview)
#[tauri::command]
fn read_font_file(path: String) -> Result<Vec<u8>, String> {
    fs::read(&path)
        .map_err(|e| format!("Failed to read font file: {}", e))
}

/// Check if file exists
#[tauri::command]
fn file_exists(path: String) -> bool {
    PathBuf::from(path).exists()
}

/// Font parse result from Python sidecar
#[derive(Debug, Serialize, Deserialize)]
pub struct FontParseResult {
    success: bool,
    name: Option<String>,
    file_path: Option<String>,
    basic_metrics: Option<serde_json::Value>,
    anatomy_metrics: Option<serde_json::Value>,
    classification: Option<String>,
    error: Option<String>,
    foundry: Option<String>,
    weight: Option<u32>,
    is_italic: Option<bool>,
    supports_vietnamese: Option<bool>,
    // Enhanced taxonomy fields (Phase 4)
    primary_class: Option<String>,
    secondary_tags: Option<Vec<String>>,
    serif_subtype: Option<String>,
    sans_subtype: Option<String>,
    script_subtype: Option<String>,
}

/// Parse font using Python sidecar
#[tauri::command]
#[allow(non_snake_case)]
async fn parse_font_with_sidecar(fontPath: String, fontId: Option<String>) -> Result<FontParseResult, String> {
    use std::process::{Command, Stdio};
    use std::io::Write;
    
    // For dev, use the Python script directly with venv
    // For production, would use bundled executable
    let sidecar_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("No parent dir")?
        .join("python-sidecar");
    
    let sidecar_path = sidecar_dir.join("sidecar.py");
    let python_path = sidecar_dir.join(".venv").join("bin").join("python3");
    
    // Use venv python if available, otherwise fall back to system python3
    let python_exe = if python_path.exists() {
        python_path
    } else {
        std::path::PathBuf::from("python3")
    };
    
    // Build request
    let request = serde_json::json!({
        "command": "parse",
        "filepath": fontPath,
        "font_id": fontId
    });
    
    // Execute Python script with stdin
    let mut child = Command::new(&python_exe)
        .arg(&sidecar_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn sidecar: {}", e))?;
    
    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{}", request)
            .map_err(|e| format!("Failed to write to sidecar: {}", e))?;
    }
    
    // Get output
    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to get sidecar output: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Sidecar failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    // Parse output - skip first line (ready signal) and parse second line
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    if lines.len() < 2 {
        return Err(format!("No response from sidecar. Output: {}", stdout));
    }
    
    // Parse the parse result (second line)
    serde_json::from_str(lines[1])
        .map_err(|e| format!("Failed to parse sidecar response: {}", e))
}

/// Database state wrapper
pub struct DbState(pub Mutex<Database>);

/// Get all fonts from library
/// Get all fonts from library (paginated)
#[tauri::command]
fn get_library_fonts(db: State<DbState>, limit: Option<usize>, offset: Option<usize>, category: Option<String>) -> Result<Vec<FontRecord>, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    // Default to a reasonably large number if not specified, 
    // but frontend should ideally specify. 
    // If user wants ALL, they might pass a huge limit, but for "loading slow" fix we want small defaults.
    let limit_val = limit.unwrap_or(50); 
    let offset_val = offset.unwrap_or(0);
    db.get_all_fonts(limit_val, offset_val, category).map_err(|e: SqlError| e.to_string())
}

/// Search fonts in library
#[tauri::command]
fn search_library(db: State<DbState>, query: String) -> Result<Vec<FontRecord>, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.search_fonts(&query).map_err(|e: SqlError| e.to_string())
}

/// Add font to library
#[tauri::command]
#[allow(non_snake_case)]
fn add_font_to_library(
    db: State<DbState>,
    name: String,
    filePath: String,
    category: Option<String>,
    classification: Option<String>,
    xHeight: Option<f64>,
    widthVariance: Option<f64>,
    strokeContrast: Option<f64>,
    aperture: Option<f64>,
    terminalSharpness: Option<f64>,
    stressAngle: Option<f64>,
    curvatureRatio: Option<f64>,
    counterDensity: Option<f64>,
    foundry: Option<String>,
    weight: Option<u32>,
    isItalic: Option<bool>,
    supportsVietnamese: Option<bool>,
    familyName: Option<String>,
    subfamily: Option<String>,
    weightClass: Option<i32>,
    widthClass: Option<i32>,
    sourceId: Option<String>,
    primaryClass: Option<String>,
    secondaryTags: Option<String>,
    serifSubtype: Option<String>,
    sansSubtype: Option<String>,
    scriptSubtype: Option<String>,
) -> Result<FontRecord, String> {
    let font = create_font_record(
        name, filePath, category, classification,
        xHeight, widthVariance, strokeContrast, aperture,
        terminalSharpness, stressAngle, curvatureRatio, counterDensity,
        foundry, weight, isItalic.unwrap_or(false), supportsVietnamese.unwrap_or(false),
        familyName, subfamily, weightClass, widthClass, sourceId,
        primaryClass, secondaryTags, serifSubtype, sansSubtype, scriptSubtype
    );
    
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.add_font(&font).map_err(|e: SqlError| e.to_string())?;
    Ok(font)
}

/// Delete font from library
#[tauri::command]
fn remove_font_from_library(db: State<DbState>, font_id: String) -> Result<(), String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.delete_font(&font_id).map_err(|e: SqlError| e.to_string())
}

/// Get library font count
#[tauri::command]
fn get_font_count(db: State<DbState>) -> Result<usize, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.font_count().map_err(|e: SqlError| e.to_string())
}

/// Get all collections
#[tauri::command]
fn get_collections(db: State<DbState>) -> Result<Vec<Collection>, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.get_all_collections().map_err(|e: SqlError| e.to_string())
}

/// Clear all fonts and sources from library
#[tauri::command]
fn clear_library(db: State<DbState>) -> Result<usize, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    // Clear fonts first
    let fonts_deleted = db.clear_all_fonts().map_err(|e: SqlError| e.to_string())?;
    // Also clear sources (directories)
    db.clear_all_sources().map_err(|e: SqlError| e.to_string())?;
    Ok(fonts_deleted)
}

/// Generate visual embedding for a single font (calls sidecar)
#[tauri::command]
#[allow(non_snake_case)]
async fn generate_visual_embedding(
    filePath: String,
    fontId: Option<String>
) -> Result<serde_json::Value, String> {
    use std::process::{Command, Stdio};
    use std::io::Write;
    
    // Use same sidecar path pattern as parse_font_with_sidecar
    let sidecar_dir = std::env::current_dir()
        .map_err(|e| e.to_string())?
        .parent()
        .ok_or("No parent dir")?
        .join("python-sidecar");
    
    let sidecar_path = sidecar_dir.join("sidecar.py");
    let python_path = sidecar_dir.join(".venv").join("bin").join("python3");
    
    let python_exe = if python_path.exists() {
        python_path
    } else {
        std::path::PathBuf::from("python3")
    };
    
    // Build embed_visual request
    let request = serde_json::json!({
        "command": "embed_visual",
        "filepath": filePath,
        "font_id": fontId
    });
    
    // Execute Python script with stdin
    let mut child = Command::new(&python_exe)
        .arg(&sidecar_path)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|e| format!("Failed to spawn sidecar: {}", e))?;
    
    // Write request to stdin
    if let Some(mut stdin) = child.stdin.take() {
        writeln!(stdin, "{}", request)
            .map_err(|e| format!("Failed to write to sidecar: {}", e))?;
    }
    
    // Get output
    let output = child.wait_with_output()
        .map_err(|e| format!("Failed to get sidecar output: {}", e))?;
    
    if !output.status.success() {
        return Err(format!("Sidecar failed: {}", String::from_utf8_lossy(&output.stderr)));
    }
    
    // Parse output - skip first line (ready signal) and parse second line
    let stdout = String::from_utf8_lossy(&output.stdout);
    let lines: Vec<&str> = stdout.lines().collect();
    
    if lines.len() < 2 {
        return Err(format!("No response from sidecar. Output: {}", stdout));
    }
    
    // Parse the response (second line)
    serde_json::from_str(lines[1])
        .map_err(|e| format!("Failed to parse sidecar response: {}", e))
}

/// Rescan a single font (force re-analyze)
#[tauri::command]
async fn rescan_font(
    app: tauri::AppHandle,
    font_id: String,
) -> Result<serde_json::Value, String> {
    log::info!("COMMAND: rescan_font for font_id: {}", font_id);
    
    let db = app.state::<DbState>();
    
    // 1. Get font from DB
    let font = {
        let db_guard = db.0.lock().map_err(|_| "Database lock error".to_string())?;
        match db_guard.get_font(&font_id) {
            Ok(Some(f)) => f,
            Ok(None) => {
                log::error!("rescan_font: Font not found: {}", font_id);
                return Err("Font not found in database".to_string());
            },
            Err(e) => {
                log::error!("rescan_font: Database error: {}", e);
                return Err(e.to_string());
            }
        }
    };


    // Emit start progress
    let _ = app.emit("rescan-progress", ProgressPayload {
        count: 0,
        total: 1,
        font_name: font.name.clone(),
    });

    // Step 1: Parse font (metrics + taxonomy)
    println!("Rescanning: Parsing metrics for {}", font.name);
    let mut updated_font = font.clone();
    match parse_font_with_sidecar(font.file_path.clone(), Some(font.id.clone())).await {
        Ok(parsed) => {
            if let Some(am) = parsed.anatomy_metrics {
                 updated_font.x_height = am.get("x_height").and_then(|v| v.as_f64());
                 updated_font.width_variance = am.get("width_variance").and_then(|v| v.as_f64());
                 updated_font.stroke_contrast = am.get("stroke_contrast").and_then(|v| v.as_f64());
                 updated_font.aperture = am.get("aperture").and_then(|v| v.as_f64());
                 updated_font.terminal_sharpness = am.get("terminal_sharpness").and_then(|v| v.as_f64());
                 updated_font.stress_angle = am.get("stress_angle").and_then(|v| v.as_f64());
                 updated_font.curvature_ratio = am.get("curvature_ratio").and_then(|v| v.as_f64());
                 updated_font.counter_density = am.get("counter_density").and_then(|v| v.as_f64());
            }
            if let Some(cls) = parsed.classification { updated_font.classification = Some(cls); }
            if let Some(fnd) = parsed.foundry { updated_font.foundry = Some(fnd); }
            if let Some(w) = parsed.weight { updated_font.weight = w; }
            if let Some(it) = parsed.is_italic { updated_font.is_italic = it; }
            if let Some(sv) = parsed.supports_vietnamese { updated_font.supports_vietnamese = sv; }
            
            // Enhanced Taxonomy
            if let Some(pc) = parsed.primary_class { updated_font.primary_class = Some(pc); }
            if let Some(st) = parsed.secondary_tags { 
                updated_font.secondary_tags = Some(serde_json::to_string(&st).unwrap_or_default());
            }
            if let Some(ss) = parsed.serif_subtype { updated_font.serif_subtype = Some(ss); }
            if let Some(sn) = parsed.sans_subtype { updated_font.sans_subtype = Some(sn); }
            if let Some(sc) = parsed.script_subtype { updated_font.script_subtype = Some(sc); }
            
            // Save to DB
            let db_lock = db.0.lock();
            if let Ok(db_guard) = db_lock {
                let _ = db_guard.add_font(&updated_font);
            }
        },
        Err(e) => {
            let error_msg = format!("Parse failed for {}: {}", font.name, e);
            let _ = app.emit("rescan-error", error_msg.clone());
            return Err(error_msg);
        }
    }

    // Update progress
    let _ = app.emit("rescan-progress", ProgressPayload {
        count: 1,
        total: 1,
        font_name: font.name.clone(),
    });

    // Step 2: Visual Embedding
    println!("Rescanning: Generating visual embedding for {}", font.name);
    match generate_visual_embedding(font.file_path.clone(), Some(font.id.clone())).await {
        Ok(val) => {
            if val.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                // Mark as fully synced
                updated_font.is_synced = true;
                let db_lock = db.0.lock();
                if let Ok(db_guard) = db_lock {
                    let _ = db_guard.add_font(&updated_font);
                }
                
                // Emit complete
                let _ = app.emit("rescan-complete", serde_json::json!({
                    "success": 1,
                    "errors": []
                }));
                
                Ok(serde_json::to_value(updated_font).unwrap_or(serde_json::Value::Null))
            } else {
                let err_msg = val.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown sidecar error");
                let full_err = format!("Visual embedding failed for {}: {}", font.name, err_msg);
                let _ = app.emit("rescan-error", full_err.clone());
                Err(full_err)
            }
        },
        Err(e) => {
            let full_err = format!("Visual embedding failed for {}: {}", font.name, e);
            let _ = app.emit("rescan-error", full_err.clone());
            Err(full_err)
        }
    }
}


#[derive(Clone, serde::Serialize)]
struct ProgressPayload {
    count: usize,
    total: usize,
    font_name: String,
}

struct RescanStatus(Mutex<bool>);

/// Resource guard that resets rescan status when dropped
struct RescanGuard {
    app: tauri::AppHandle,
}

impl RescanGuard {
    fn new(app: tauri::AppHandle) -> Result<Self, String> {
        {
            let status = app.state::<RescanStatus>();
            let mut is_running = status.0.lock().map_err(|_| "Failed to lock rescan status")?;
            
            if *is_running {
                return Err("Rescan already in progress".to_string());
            }
            
            *is_running = true;
        }
        Ok(Self { app })
    }
}

impl Drop for RescanGuard {
    fn drop(&mut self) {
        let status = self.app.state::<RescanStatus>();
        if let Ok(mut is_running) = status.0.lock() {
            *is_running = false;
        }
        println!("COMMAND: start_background_rescan guard dropped (status reset)");
    }
}

/// Start background rescan of all fonts in library
#[tauri::command]
fn start_background_rescan(app: tauri::AppHandle, force: Option<bool>) -> Result<String, String> {
    // Attempt to create guard. If fails, scan is already running.
    let _guard = match RescanGuard::new(app.clone()) {
        Ok(g) => g,
        Err(e) => {
            println!("COMMAND: start_background_rescan ignored ({})", e);
            return Ok(e);
        }
    };
    
    let force_val = force.unwrap_or(false);
    println!("COMMAND: start_background_rescan started (force: {})", force_val);
    
    tauri::async_runtime::spawn(async move {
        // Move guard into task so it's dropped when task finishes
        let _guard = _guard;
        
        match perform_library_rescan(app.clone(), force_val).await {

            Ok(result) => {
                let _ = app.emit("rescan-complete", result);
            },
            Err(e) => {
                let _ = app.emit("rescan-error", e);
            }
        }
    });
    
    Ok("Background rescan started".to_string())
}

/// Actual worker for library rescan
async fn perform_library_rescan(app: tauri::AppHandle, force: bool) -> Result<serde_json::Value, String> {

    let db = app.state::<DbState>();
    
    // 1. Get all fonts from DB
    let fonts = {
        let db_guard = db.0.lock().map_err(|_| "Database lock error".to_string())?;
        db_guard.get_all_fonts(10000, 0, None).map_err(|e| format!("{:?}", e))?
    };

    let total = fonts.len();
    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors: Vec<String> = Vec::new();

    println!("STARTING background rescan of {} fonts", total);
    
    for (index, font) in fonts.iter().enumerate() {
        // Emit progress event
        let _ = app.emit("rescan-progress", ProgressPayload {
            count: index + 1,
            total,
            font_name: font.name.clone(),
        });

        // Skip if already processed (unless forced)
        if !force && font.is_synced {
            println!("[{}/{}] Skipping {} (Already processed)", index + 1, total, font.name);
            success_count += 1;
            continue;
        }


        // Step 1: Parse font (metrics + taxonomy)
        println!("[{}/{}] Processing: Parsing metrics for {}", index + 1, total, font.name);
        
        // We'll keep track of the updated record
        let mut updated_font = font.clone();

        match parse_font_with_sidecar(font.file_path.clone(), Some(font.id.clone())).await {
            Ok(parsed) => {
                // Update SQLite database with parsed results
                if let Some(am) = parsed.anatomy_metrics {
                     updated_font.x_height = am.get("x_height").and_then(|v| v.as_f64());
                     updated_font.width_variance = am.get("width_variance").and_then(|v| v.as_f64());
                     updated_font.stroke_contrast = am.get("stroke_contrast").and_then(|v| v.as_f64());
                     updated_font.aperture = am.get("aperture").and_then(|v| v.as_f64());
                     updated_font.terminal_sharpness = am.get("terminal_sharpness").and_then(|v| v.as_f64());
                     updated_font.stress_angle = am.get("stress_angle").and_then(|v| v.as_f64());
                     updated_font.curvature_ratio = am.get("curvature_ratio").and_then(|v| v.as_f64());
                     updated_font.counter_density = am.get("counter_density").and_then(|v| v.as_f64());
                }
                
                if let Some(cls) = parsed.classification { updated_font.classification = Some(cls); }
                if let Some(fnd) = parsed.foundry { updated_font.foundry = Some(fnd); }
                if let Some(w) = parsed.weight { updated_font.weight = w; }
                if let Some(it) = parsed.is_italic { updated_font.is_italic = it; }
                if let Some(sv) = parsed.supports_vietnamese { updated_font.supports_vietnamese = sv; }
                
                // Enhanced Taxonomy
                if let Some(pc) = parsed.primary_class { updated_font.primary_class = Some(pc); }
                if let Some(st) = parsed.secondary_tags { 
                    updated_font.secondary_tags = Some(serde_json::to_string(&st).unwrap_or_default());
                }
                if let Some(ss) = parsed.serif_subtype { updated_font.serif_subtype = Some(ss); }
                if let Some(sn) = parsed.sans_subtype { updated_font.sans_subtype = Some(sn); }
                if let Some(sc) = parsed.script_subtype { updated_font.script_subtype = Some(sc); }

                // Save metrics to DB immediately
                let db_lock = db.0.lock();
                match db_lock {
                    Ok(db_guard) => {
                        let _ = db_guard.add_font(&updated_font);
                    },
                    Err(e) => {
                        eprintln!("Database lock error during rescan for {}: {}", font.name, e);
                    }
                }
            },
            Err(e) => {
                println!("Parse failed for {}: {}", font.name, e);
                errors.push(format!("Parse failed for {}: {}", font.name, e));
            }
        }

        // Step 2: Generate Visual Embedding
        println!("[{}/{}] Processing: Generating visual embedding for {}", index + 1, total, font.name);
        match generate_visual_embedding(
            font.file_path.clone(),
            Some(font.id.clone())
        ).await {
            Ok(val) => {
                if val.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                    success_count += 1;
                    
                    // Mark as fully synced in DB
                    updated_font.is_synced = true;
                    let db_lock = db.0.lock();
                    if let Ok(db_guard) = db_lock {
                        let _ = db_guard.add_font(&updated_font);
                    }
                } else {
                    error_count += 1;
                    let err = val.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown sidecar error");
                    errors.push(format!("Embedding failed for {}: {}", font.name, err));
                }
            },
            Err(e) => {
                error_count += 1;
                errors.push(format!("Embedding failed for {}: {}", font.name, e));
            }
        }
    }

    println!("Background rescan finished. Total: {}, Success: {}, Errors: {}", total, success_count, error_count);
    Ok(serde_json::json!({
        "total": total,
        "success": success_count,
        "errors": error_count,
        "error_details": errors
    }))
}

/// Batch generate visual embeddings for all fonts in library (LEGACY - use start_background_rescan)
#[tauri::command]
async fn batch_generate_embeddings(
    window: tauri::Window,
    db: State<'_, DbState>,
) -> Result<serde_json::Value, String> {
    // For now, keep the old command but potentially redirect or warn
    // Actually, legacy support is fine if we want to keep current UI working while transitioning.
    // Let's just make it call the same logic but wait for it.
    let fonts = {
        let db_guard = db.0.lock().map_err(|_| "Database lock error".to_string())?;
        db_guard.get_all_fonts(1000, 0, None).map_err(|e| format!("{:?}", e))?
    };

    let mut success_count = 0;
    let mut error_count = 0;
    let mut errors: Vec<String> = Vec::new();
    let total = fonts.len();

    for (index, font) in fonts.iter().enumerate() {
        // Emit progress event
        let _ = window.emit("rescan-progress", ProgressPayload {
            count: index + 1,
            total,
            font_name: font.name.clone(),
        });

        // Step 1: Parse and Index font (ensures point exists in Qdrant with metrics)
        let _ = parse_font_with_sidecar(font.file_path.clone(), Some(font.id.clone())).await;

        // Step 2: Generate Visual Embedding
        match generate_visual_embedding(
            font.file_path.clone(),
            Some(font.id.clone())
        ).await {
            Ok(val) => {
                if val.get("success").and_then(|v| v.as_bool()).unwrap_or(false) {
                    success_count += 1;
                } else {
                    error_count += 1;
                    let err = val.get("error").and_then(|v| v.as_str()).unwrap_or("Unknown sidecar error");
                    errors.push(format!("{}: {}", font.name, err));
                }
            },
            Err(e) => {
                error_count += 1;
                errors.push(format!("{}: {}", font.name, e));
            }
        }
    }

    Ok(serde_json::json!({
        "total": fonts.len(),
        "success": success_count,
        "errors": error_count,
        "error_details": errors
    }))
}

/// Create a new collection
#[tauri::command]
fn create_collection(db: State<DbState>, name: String, color: Option<String>) -> Result<Collection, String> {
    let collection = Collection {
        id: uuid::Uuid::new_v4().to_string(),
        name,
        color,
        created_at: chrono::Utc::now().to_rfc3339(),
    };
    
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.add_collection(&collection).map_err(|e: SqlError| e.to_string())?;
    Ok(collection)
}

/// Delete a collection
#[tauri::command]
fn delete_collection(db: State<DbState>, collection_id: String) -> Result<(), String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.delete_collection(&collection_id).map_err(|e: SqlError| e.to_string())
}

/// Rename a collection
#[tauri::command]
fn rename_collection(db: State<DbState>, collection_id: String, name: String) -> Result<(), String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.rename_collection(&collection_id, &name).map_err(|e: SqlError| e.to_string())
}

/// Toggle favorite status of a font
#[tauri::command]
fn toggle_favorite(db: State<DbState>, id: String, is_favorite: bool) -> Result<(), String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.toggle_favorite(&id, is_favorite).map_err(|e: SqlError| e.to_string())
}

// ============ Source Management Commands ============

/// Create a new source
#[tauri::command]
fn create_source(
    db: State<DbState>,
    name: String,
    source_type: String,
    path: Option<String>,
) -> Result<Source, String> {
    let source = Source {
        id: Uuid::new_v4().to_string(),
        name,
        source_type,
        path,
        metadata: None,
        date_added: Utc::now().to_rfc3339(),
    };
    
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.add_source(&source).map_err(|e: SqlError| e.to_string())?;
    Ok(source)
}

/// Get all sources
#[tauri::command]
fn get_sources(db: State<DbState>) -> Result<Vec<Source>, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.get_sources().map_err(|e: SqlError| e.to_string())
}

/// Delete a source
#[tauri::command]
fn delete_source(db: State<DbState>, source_id: String) -> Result<(), String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.delete_source(&source_id).map_err(|e: SqlError| e.to_string())
}

/// Get font count by source
#[tauri::command]
#[allow(non_snake_case)]
fn get_font_count_by_source(db: State<DbState>, sourceId: String) -> Result<usize, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.get_font_count_by_source(&sourceId).map_err(|e: SqlError| e.to_string())
}

/// Get fonts by source
#[tauri::command]
fn get_fonts_by_source(
    db: State<DbState>,
    source_id: String,
    limit: usize,
    offset: usize
) -> Result<Vec<FontRecord>, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.get_fonts_by_source(&source_id, limit, offset).map_err(|e: SqlError| e.to_string())
}

// ============ Qdrant Vector Search Commands ============

use qdrant::{QdrantService, SimilarFont, FontPairing, PairingByLogic};


/// Qdrant service state wrapper
pub struct QdrantState(pub Mutex<Option<QdrantService>>);

/// Find similar fonts using vector similarity
#[tauri::command]
async fn find_similar_fonts(
    font_id: String,
    limit: Option<usize>,
    mode: Option<String>,
) -> Result<Vec<SimilarFont>, String> {
    let limit_val = limit.unwrap_or(5);
    let mode_val = mode.unwrap_or_else(|| "perceptual".to_string());
    
    // Create Qdrant client on-demand
    let service = QdrantService::new().await?;
    
    if mode_val == "metrics" {
        service.find_similar_by_metrics(&font_id, limit_val).await
    } else {
        service.find_similar_by_perceptual(&font_id, limit_val).await
    }
}

/// Find complementary font pairs for a given font
#[tauri::command]
async fn find_font_pairs(
    app: tauri::AppHandle,
    font_id: String,
    limit: Option<usize>,
) -> Result<Vec<FontPairing>, String> {
    let limit_val = limit.unwrap_or(5);
    log::info!("COMMAND: find_font_pairs for font_id: {}, limit: {}", font_id, limit_val);
    
    // Create Qdrant client on-demand
    match QdrantService::new().await {
        Ok(service) => {
            match service.find_font_pairs(&font_id, limit_val).await {
                Ok(pairs) => {
                    log::info!("find_font_pairs: Found {} pairs for {}", pairs.len(), font_id);
                    Ok(pairs)
                },
                Err(e) => {
                    log::error!("find_font_pairs: Failed to find pairs: {}", e);
                    Err(e)
                }
            }
        },
        Err(e) => {
            log::error!("find_font_pairs: Failed to create Qdrant service: {}", e);
            Err(e)
        }
    }
}

/// Find font combinations using multiple pairing logics
#[tauri::command]
async fn find_font_combos(
    font_id: String,
    limit_per_logic: Option<usize>,
) -> Result<Vec<PairingByLogic>, String> {
    let limit = limit_per_logic.unwrap_or(3);
    log::info!("COMMAND: find_font_combos for font_id: {}, limit_per_logic: {}", font_id, limit);
    
    match QdrantService::new().await {
        Ok(service) => {
            match service.find_font_combos(&font_id, limit).await {
                Ok(logics) => {
                    let total: usize = logics.iter().map(|l| l.results.len()).sum();
                    log::info!("find_font_combos: Found {} total combos across {} logics", total, logics.len());
                    Ok(logics)
                },
                Err(e) => {
                    log::error!("find_font_combos: Failed: {}", e);
                    Err(e)
                }
            }
        },
        Err(e) => {
            log::error!("find_font_combos: Failed to create Qdrant service: {}", e);
            Err(e)
        }
    }
}



#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize logging with both console and file output
            app.handle().plugin(
                tauri_plugin_log::Builder::default()
                    .level(log::LevelFilter::Info)
                    .clear_targets()  // Clear default targets first
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::Stdout,
                    ))
                    .target(tauri_plugin_log::Target::new(
                        tauri_plugin_log::TargetKind::LogDir {
                            file_name: Some("app.log".to_string()),
                        },
                    ))
                    .rotation_strategy(tauri_plugin_log::RotationStrategy::KeepOne)
                    .build(),
            )?;
            

            // Initialize database
            let data_dir = app.path().app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            fs::create_dir_all(&data_dir)?;
            
            let db_path = data_dir.join("fontatlas.db");
            let database = Database::new(&db_path)
                .map_err(|e| format!("Failed to init database: {}", e))?;
            
            app.manage(DbState(Mutex::new(database)));
            app.manage(RescanStatus(Mutex::new(false)));
            
            log::info!("FontAtlas database initialized at {:?}", db_path);
            
            // Initialize logger for AI agent access
            let log_dir = data_dir.join("logs");
            let log_path = log_dir.join("app.log");
            let app_logger = AppLogger::new(log_path.clone())
                .map_err(|e| format!("Failed to init logger: {}", e))?;
            
            app.manage(Mutex::new(app_logger));
            log::info!("AI agent logger initialized at {:?}", log_path);
            
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            scan_directory,
            open_folder_dialog,
            open_font_dialog,
            get_app_data_dir,
            read_font_file,
            file_exists,
            parse_font_with_sidecar,
            write_log,
            // Font library commands
            get_library_fonts,
            search_library,
            add_font_to_library,
            remove_font_from_library,
            get_font_count,
            clear_library,
            toggle_favorite,
            // Collection commands
            get_collections,
            create_collection,
            delete_collection,
            rename_collection,
            // Source commands
            create_source,
            get_sources,
            delete_source,
            get_font_count_by_source,
            get_fonts_by_source,
            // Visual embedding (toolbox)
            generate_visual_embedding,
            batch_generate_embeddings,
            start_background_rescan,
            rescan_font,
            // Qdrant vector search
            find_similar_fonts,

            find_font_pairs,
            find_font_combos,
        ])

        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

