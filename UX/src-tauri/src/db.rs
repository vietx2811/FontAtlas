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
    // Extended Metadata
    pub foundry: Option<String>,
    pub weight: u32,
    pub is_italic: bool,
    pub supports_vietnamese: bool,
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
        // Attempt to add new columns if they don't exist (Simple migration for dev)
        // This is a "best effort" migration for the dev environment without a full migration system
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN foundry TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN weight INTEGER DEFAULT 400", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN is_italic INTEGER DEFAULT 0", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN supports_vietnamese INTEGER DEFAULT 0", []);

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
                aperture REAL,
                foundry TEXT,
                weight INTEGER DEFAULT 400,
                is_italic INTEGER DEFAULT 0,
                supports_vietnamese INTEGER DEFAULT 0
            )",
            [],
        )?;
        
        // ... (rest of indexes) ...
        Ok(())
    }
    
    /// Add a font to the library
    pub fn add_font(&self, font: &FontRecord) -> Result<()> {
        let tags_json = serde_json::to_string(&font.tags).unwrap_or_default();
        
        self.conn.execute(
            "INSERT OR REPLACE INTO fonts 
             (id, name, file_path, original_directory, category, classification,
              tags, collection_id, date_added, date_modified, is_synced, file_exists,
              x_height, width_variance, stroke_contrast, aperture,
              foundry, weight, is_italic, supports_vietnamese)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20)",
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
                font.foundry,
                font.weight,
                font.is_italic as i32,
                font.supports_vietnamese as i32,
            ],
        )?;
        Ok(())
    }
    
    /// Get all fonts (paginated and filtered)
    pub fn get_all_fonts(&self, limit: usize, offset: usize, category_filter: Option<String>) -> Result<Vec<FontRecord>> {
        let sql = if let Some(ref cat) = category_filter {
            if cat == "All" {
                "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    foundry, weight, is_italic, supports_vietnamese
                 FROM fonts ORDER BY name LIMIT ?1 OFFSET ?2"
            } else {
                "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    foundry, weight, is_italic, supports_vietnamese
                 FROM fonts WHERE category = ?3 ORDER BY name LIMIT ?1 OFFSET ?2"
            }
        } else {
            "SELECT id, name, file_path, original_directory, category, classification,
                tags, collection_id, date_added, date_modified, is_synced, file_exists,
                x_height, width_variance, stroke_contrast, aperture,
                foundry, weight, is_italic, supports_vietnamese
             FROM fonts ORDER BY name LIMIT ?1 OFFSET ?2"
        };

        let mut stmt = self.conn.prepare(sql)?;
        
        let rows = if let Some(cat) = category_filter {
            if cat == "All" {
                 stmt.query_map(params![limit, offset], |row: &rusqlite::Row| {
                     self.map_font_row(row)
                 })?.collect::<Result<Vec<_>>>()?
            } else {
                 stmt.query_map(params![limit, offset, cat], |row: &rusqlite::Row| {
                     self.map_font_row(row)
                 })?.collect::<Result<Vec<_>>>()?
            }
        } else {
             stmt.query_map(params![limit, offset], |row: &rusqlite::Row| {
                 self.map_font_row(row)
             })?.collect::<Result<Vec<_>>>()?
        };

        Ok(rows)
    }

    // Helper to map row to FontRecord to avoid duplication
    fn map_font_row(&self, row: &rusqlite::Row) -> Result<FontRecord> {
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
            foundry: row.get(16)?,
            weight: row.get(17)?,
            is_italic: row.get::<_, i32>(18)? != 0,
            supports_vietnamese: row.get::<_, i32>(19)? != 0,
        })
    }
    
    /// Search fonts by name
    pub fn search_fonts(&self, query: &str) -> Result<Vec<FontRecord>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    foundry, weight, is_italic, supports_vietnamese
             FROM fonts WHERE name LIKE ?1 ORDER BY name"
        )?;
        
        let fonts = stmt.query_map([search_pattern], |row: &rusqlite::Row| {
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
                foundry: row.get(16)?,
                weight: row.get(17)?,
                is_italic: row.get::<_, i32>(18)? != 0,
                supports_vietnamese: row.get::<_, i32>(19)? != 0,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(fonts)
    }
    
    /// Delete a font by ID
    pub fn delete_font(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM fonts WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Get total font count
    pub fn font_count(&self) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM fonts",
            [],
            |row| row.get(0),
        )?;
        Ok(count)
    }

    /// Clear all fonts
    pub fn clear_all_fonts(&self) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM fonts",
            [],
        )?;
        Ok(count)
    }

    /// Add a collection
    pub fn add_collection(&self, collection: &Collection) -> Result<()> {
        self.conn.execute(
            "INSERT INTO collections (id, name, color, created_at)
             VALUES (?1, ?2, ?3, ?4)",
            params![
                collection.id,
                collection.name,
                collection.color,
                collection.created_at
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

    /// Delete a collection
    pub fn delete_collection(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM collections WHERE id = ?1",
            params![id],
        )?;
        Ok(())
    }

    /// Rename a collection
    pub fn rename_collection(&self, id: &str, name: &str) -> Result<()> {
        self.conn.execute(
            "UPDATE collections SET name = ?2 WHERE id = ?1",
            params![id, name],
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
    foundry: Option<String>,
    weight: Option<u32>,
    is_italic: bool,
    supports_vietnamese: bool,
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
        foundry,
        weight: weight.unwrap_or(400),
        is_italic,
        supports_vietnamese,
    }
}
