/// Database module for font library storage
/// Uses SQLite with rusqlite

use rusqlite::{Connection, Result, params};
use serde::{Deserialize, Serialize};
use std::path::Path;
use chrono::{DateTime, Utc};
use uuid::Uuid;

/// Font entry stored in database
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FontRecord {
    pub id: String,
    pub name: String,
    pub file_path: String,
    pub original_directory: String,
    pub category: Option<String>,
    pub classification: Option<String>,
    pub tags: Vec<String>,
    pub collection_id: Option<String>,
    pub date_added: String,
    pub date_modified: String,
    pub is_synced: bool,
    pub file_exists: bool,
    // Metrics
    pub x_height: Option<f64>,
    pub width_variance: Option<f64>,
    pub stroke_contrast: Option<f64>,
    pub aperture: Option<f64>,
}

/// Collection (virtual folder)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
}

/// Database wrapper
pub struct Database {
    conn: Connection,
}

impl Database {
    /// Create or open database at path
    pub fn new(db_path: &Path) -> Result<Self> {
        let conn = Connection::open(db_path)?;
        let db = Self { conn };
        db.init_tables()?;
        Ok(db)
    }
    
    /// Initialize database tables
    fn init_tables(&self) -> Result<()> {
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS fonts (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                file_path TEXT NOT NULL UNIQUE,
                original_directory TEXT,
                category TEXT,
                classification TEXT,
                tags TEXT,
                collection_id TEXT,
                date_added TEXT NOT NULL,
                date_modified TEXT NOT NULL,
                is_synced INTEGER DEFAULT 0,
                file_exists INTEGER DEFAULT 1,
                x_height REAL,
                width_variance REAL,
                stroke_contrast REAL,
                aperture REAL
            )",
            [],
        )?;
        
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS collections (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                color TEXT,
                created_at TEXT NOT NULL
            )",
            [],
        )?;
        
        // Create indexes for fast search
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_fonts_name ON fonts (name)",
            [],
        )?;
        
        self.conn.execute(
            "CREATE INDEX IF NOT EXISTS idx_fonts_category ON fonts (category)",
            [],
        )?;
        
        Ok(())
    }
    
    /// Add a font to the library
    pub fn add_font(&self, font: &FontRecord) -> Result<()> {
        let tags_json = serde_json::to_string(&font.tags).unwrap_or_default();
        
        self.conn.execute(
            "INSERT OR REPLACE INTO fonts 
             (id, name, file_path, original_directory, category, classification,
              tags, collection_id, date_added, date_modified, is_synced, file_exists,
              x_height, width_variance, stroke_contrast, aperture)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16)",
            params![
                font.id,
                font.name,
                font.file_path,
                font.original_directory,
                font.category,
                font.classification,
                tags_json,
                font.collection_id,
                font.date_added,
                font.date_modified,
                font.is_synced as i32,
                font.file_exists as i32,
                font.x_height,
                font.width_variance,
                font.stroke_contrast,
                font.aperture,
            ],
        )?;
        Ok(())
    }
    
    /// Get all fonts
    pub fn get_all_fonts(&self) -> Result<Vec<FontRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture
             FROM fonts ORDER BY name"
        )?;
        
        let fonts = stmt.query_map([], |row| {
            let tags_json: String = row.get(6)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            
            Ok(FontRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                file_path: row.get(2)?,
                original_directory: row.get(3)?,
                category: row.get(4)?,
                classification: row.get(5)?,
                tags,
                collection_id: row.get(7)?,
                date_added: row.get(8)?,
                date_modified: row.get(9)?,
                is_synced: row.get::<_, i32>(10)? != 0,
                file_exists: row.get::<_, i32>(11)? != 0,
                x_height: row.get(12)?,
                width_variance: row.get(13)?,
                stroke_contrast: row.get(14)?,
                aperture: row.get(15)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(fonts)
    }
    
    /// Search fonts by name
    pub fn search_fonts(&self, query: &str) -> Result<Vec<FontRecord>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture
             FROM fonts WHERE name LIKE ?1 ORDER BY name"
        )?;
        
        let fonts = stmt.query_map([search_pattern], |row| {
            let tags_json: String = row.get(6)?;
            let tags: Vec<String> = serde_json::from_str(&tags_json).unwrap_or_default();
            
            Ok(FontRecord {
                id: row.get(0)?,
                name: row.get(1)?,
                file_path: row.get(2)?,
                original_directory: row.get(3)?,
                category: row.get(4)?,
                classification: row.get(5)?,
                tags,
                collection_id: row.get(7)?,
                date_added: row.get(8)?,
                date_modified: row.get(9)?,
                is_synced: row.get::<_, i32>(10)? != 0,
                file_exists: row.get::<_, i32>(11)? != 0,
                x_height: row.get(12)?,
                width_variance: row.get(13)?,
                stroke_contrast: row.get(14)?,
                aperture: row.get(15)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(fonts)
    }
    
    /// Delete font by id
    pub fn delete_font(&self, id: &str) -> Result<()> {
        self.conn.execute("DELETE FROM fonts WHERE id = ?1", [id])?;
        Ok(())
    }
    
    /// Clear all fonts from library
    pub fn clear_all_fonts(&self) -> Result<usize> {
        let deleted = self.conn.execute("DELETE FROM fonts", [])?;
        Ok(deleted)
    }
    
    /// Get font count
    pub fn font_count(&self) -> Result<usize> {
        let count: i64 = self.conn.query_row(
            "SELECT COUNT(*) FROM fonts",
            [],
            |row| row.get(0)
        )?;
        Ok(count as usize)
    }
    
    /// Add collection
    pub fn add_collection(&self, collection: &Collection) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO collections (id, name, color, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                collection.id,
                collection.name,
                collection.color,
                collection.created_at,
            ],
        )?;
        Ok(())
    }
    
    /// Get all collections
    pub fn get_all_collections(&self) -> Result<Vec<Collection>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, color, created_at FROM collections ORDER BY name"
        )?;
        
        let collections = stmt.query_map([], |row| {
            Ok(Collection {
                id: row.get(0)?,
                name: row.get(1)?,
                color: row.get(2)?,
                created_at: row.get(3)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(collections)
    }
    
    /// Delete collection
    pub fn delete_collection(&self, id: &str) -> Result<()> {
        // First, clear collection_id from fonts in this collection
        self.conn.execute(
            "UPDATE fonts SET collection_id = NULL WHERE collection_id = ?1",
            [id]
        )?;
        // Then delete the collection
        self.conn.execute("DELETE FROM collections WHERE id = ?1", [id])?;
        Ok(())
    }
    
    /// Rename collection
    pub fn rename_collection(&self, id: &str, new_name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE collections SET name = ?1 WHERE id = ?2",
            params![new_name, id]
        )?;
        Ok(())
    }
    
    /// Add font to collection
    pub fn add_font_to_collection(&self, font_id: &str, collection_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE fonts SET collection_id = ?1 WHERE id = ?2",
            params![collection_id, font_id]
        )?;
        Ok(())
    }
    
    /// Remove font from collection
    pub fn remove_font_from_collection(&self, font_id: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE fonts SET collection_id = NULL WHERE id = ?1",
            [font_id]
        )?;
        Ok(())
    }
}

/// Create a new font record from parsed data
pub fn create_font_record(
    name: String,
    file_path: String,
    category: Option<String>,
    classification: Option<String>,
    x_height: Option<f64>,
    width_variance: Option<f64>,
    stroke_contrast: Option<f64>,
    aperture: Option<f64>,
) -> FontRecord {
    let now = Utc::now().to_rfc3339();
    let parent = std::path::Path::new(&file_path)
        .parent()
        .map(|p| p.to_string_lossy().to_string())
        .unwrap_or_default();
    
    FontRecord {
        id: Uuid::new_v4().to_string(),
        name,
        file_path,
        original_directory: parent,
        category,
        classification,
        tags: vec![],
        collection_id: None,
        date_added: now.clone(),
        date_modified: now,
        is_synced: false,
        file_exists: true,
        x_height,
        width_variance,
        stroke_contrast,
        aperture,
    }
}
