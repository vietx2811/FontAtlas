mod db;

use std::fs;
use std::path::PathBuf;
use std::sync::Mutex;
use serde::{Deserialize, Serialize};
use tauri::{Manager, State};
use rusqlite::Error as SqlError;

use db::{Database, FontRecord, Collection, create_font_record};

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
}

/// Parse font using Python sidecar
#[tauri::command]
#[allow(non_snake_case)]
async fn parse_font_with_sidecar(fontPath: String) -> Result<FontParseResult, String> {
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
        "filepath": fontPath
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
    foundry: Option<String>,
    weight: Option<u32>,
    isItalic: Option<bool>,
    supportsVietnamese: Option<bool>,
) -> Result<FontRecord, String> {
    let font = create_font_record(
        name, filePath, category, classification,
        xHeight, widthVariance, strokeContrast, aperture,
        foundry, weight, isItalic.unwrap_or(false), supportsVietnamese.unwrap_or(false)
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

/// Clear all fonts from library
#[tauri::command]
fn clear_library(db: State<DbState>) -> Result<usize, String> {
    let db = db.0.lock().map_err(|_| "Database lock error".to_string())?;
    db.clear_all_fonts().map_err(|e: SqlError| e.to_string())
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


#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_shell::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .setup(|app| {
            // Initialize logging
            if cfg!(debug_assertions) {
                app.handle().plugin(
                    tauri_plugin_log::Builder::default()
                        .level(log::LevelFilter::Info)
                        .build(),
                )?;
            }
            
            // Initialize database
            let data_dir = app.path().app_data_dir()
                .map_err(|e| format!("Failed to get app data dir: {}", e))?;
            fs::create_dir_all(&data_dir)?;
            
            let db_path = data_dir.join("fontatlas.db");
            let database = Database::new(&db_path)
                .map_err(|e| format!("Failed to init database: {}", e))?;
            
            app.manage(DbState(Mutex::new(database)));
            
            log::info!("FontAtlas database initialized at {:?}", db_path);
            
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
            // Font library commands
            get_library_fonts,
            search_library,
            add_font_to_library,
            remove_font_from_library,
            get_font_count,
            clear_library,
            // Collection commands
            get_collections,
            create_collection,
            delete_collection,
            rename_collection,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}

