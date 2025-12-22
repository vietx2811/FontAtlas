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
    pub terminal_sharpness: Option<f64>,
    pub stress_angle: Option<f64>,
    pub curvature_ratio: Option<f64>,
    pub counter_density: Option<f64>,
    // Extended Metadata
    pub foundry: Option<String>,
    pub weight: u32,
    pub is_italic: bool,
    pub supports_vietnamese: bool,
    // Family Info
    pub family_name: Option<String>,
    pub subfamily: Option<String>,
    pub weight_class: Option<i32>,
    pub width_class: Option<i32>,
    // Source
    pub source_id: Option<String>,
    // Taxonomy (new classification system)
    pub primary_class: Option<String>,
    pub secondary_tags: Option<String>,  // JSON string
    pub serif_subtype: Option<String>,
    pub sans_subtype: Option<String>,
    pub script_subtype: Option<String>,
    pub is_favorite: bool,
}

/// Collection (virtual folder)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Collection {
    pub id: String,
    pub name: String,
    pub color: Option<String>,
    pub created_at: String,
}

/// Source (directory, Google Fonts, etc.)
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Source {
    pub id: String,
    pub name: String,
    pub source_type: String,  // 'directory' | 'google_fonts' | 'adobe_fonts'
    pub path: Option<String>,
    pub metadata: Option<String>,  // JSON blob
    pub date_added: String,
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
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN terminal_sharpness REAL", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN stress_angle REAL", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN curvature_ratio REAL", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN counter_density REAL", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN family_name TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN subfamily TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN weight_class INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN width_class INTEGER", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN source_id TEXT", []);
        // Taxonomy columns
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN primary_class TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN secondary_tags TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN serif_subtype TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN sans_subtype TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN script_subtype TEXT", []);
        let _ = self.conn.execute("ALTER TABLE fonts ADD COLUMN is_favorite INTEGER DEFAULT 0", []);

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
                terminal_sharpness REAL,
                stress_angle REAL,
                curvature_ratio REAL,
                counter_density REAL,
                foundry TEXT,
                weight INTEGER DEFAULT 400,
                is_italic INTEGER DEFAULT 0,
                supports_vietnamese INTEGER DEFAULT 0,
                family_name TEXT,
                subfamily TEXT,
                weight_class INTEGER,
                width_class INTEGER,
                source_id TEXT,
                primary_class TEXT,
                secondary_tags TEXT,
                serif_subtype TEXT,
                sans_subtype TEXT,
                script_subtype TEXT,
                is_favorite INTEGER DEFAULT 0
            )",
            [],
        )?;
        
        // Create sources table
        self.conn.execute(
            "CREATE TABLE IF NOT EXISTS sources (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                type TEXT NOT NULL,
                path TEXT,
                metadata TEXT,
                date_added TEXT NOT NULL
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
              terminal_sharpness, stress_angle, curvature_ratio, counter_density,
              foundry, weight, is_italic, supports_vietnamese,
              family_name, subfamily, weight_class, width_class, source_id,
              primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15, ?16, ?17, ?18, ?19, ?20, ?21, ?22, ?23, ?24, ?25, ?26, ?27, ?28, ?29, ?30, ?31, ?32, ?33, ?34, ?35)",
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
                font.terminal_sharpness,
                font.stress_angle,
                font.curvature_ratio,
                font.counter_density,
                font.foundry,
                font.weight,
                font.is_italic as i32,
                font.supports_vietnamese as i32,
                font.family_name,
                font.subfamily,
                font.weight_class,
                font.width_class,
                font.source_id,
                font.primary_class,
                font.secondary_tags,
                font.serif_subtype,
                font.sans_subtype,
                font.script_subtype,
                font.is_favorite as i32,
            ],
        )?;
        Ok(())
    }
    
    /// Get a font by ID
    pub fn get_font(&self, id: &str) -> Result<Option<FontRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    terminal_sharpness, stress_angle, curvature_ratio, counter_density,
                    foundry, weight, is_italic, supports_vietnamese,
                    family_name, subfamily, weight_class, width_class, source_id,
                    primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite
             FROM fonts WHERE id = ?1"
        )?;
        
        let mut rows = stmt.query_map([id], |row| {
            self.map_font_row(row)
        })?;
        
        if let Some(row) = rows.next() {
            return Ok(Some(row?));
        }
        
        Ok(None)
    }
    
    /// Get all fonts (paginated and filtered)
    pub fn get_all_fonts(&self, limit: usize, offset: usize, category_filter: Option<String>) -> Result<Vec<FontRecord>> {
        let sql = if let Some(ref cat) = category_filter {
            if cat == "All" {
                "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    terminal_sharpness, stress_angle, curvature_ratio, counter_density,
                    foundry, weight, is_italic, supports_vietnamese,
                    family_name, subfamily, weight_class, width_class, source_id,
                    primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite
                 FROM fonts ORDER BY name LIMIT ?1 OFFSET ?2"
            } else {
                "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    terminal_sharpness, stress_angle, curvature_ratio, counter_density,
                    foundry, weight, is_italic, supports_vietnamese,
                    family_name, subfamily, weight_class, width_class, source_id,
                    primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite
                 FROM fonts WHERE category = ?3 ORDER BY name LIMIT ?1 OFFSET ?2"
            }
        } else {
            "SELECT id, name, file_path, original_directory, category, classification,
                tags, collection_id, date_added, date_modified, is_synced, file_exists,
                x_height, width_variance, stroke_contrast, aperture,
                terminal_sharpness, stress_angle, curvature_ratio, counter_density,
                foundry, weight, is_italic, supports_vietnamese,
                family_name, subfamily, weight_class, width_class, source_id,
                primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite
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
            terminal_sharpness: row.get(16)?,
            stress_angle: row.get(17)?,
            curvature_ratio: row.get(18)?,
            counter_density: row.get(19)?,
            foundry: row.get(20)?,
            weight: row.get(21)?,
            is_italic: row.get::<_, i32>(22)? != 0,
            supports_vietnamese: row.get::<_, i32>(23)? != 0,
            family_name: row.get(24)?,
            subfamily: row.get(25)?,
            weight_class: row.get(26)?,
            width_class: row.get(27)?,
            source_id: row.get(28)?,
            primary_class: row.get(29)?,
            secondary_tags: row.get(30)?,
            serif_subtype: row.get(31)?,
            sans_subtype: row.get(32)?,
            script_subtype: row.get(33)?,
            is_favorite: row.get::<_, i32>(34)? != 0,
        })
    }
    
    /// Search fonts by name
    pub fn search_fonts(&self, query: &str) -> Result<Vec<FontRecord>> {
        let search_pattern = format!("%{}%", query);
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    terminal_sharpness, stress_angle, curvature_ratio, counter_density,
                    foundry, weight, is_italic, supports_vietnamese,
                    family_name, subfamily, weight_class, width_class, source_id,
                    primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite
             FROM fonts WHERE name LIKE ?1 ORDER BY name"
        )?;
        
        let fonts = stmt.query_map([search_pattern], |row: &rusqlite::Row| {
            self.map_font_row(row)
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

    /// Clear all sources
    pub fn clear_all_sources(&self) -> Result<usize> {
        let count = self.conn.execute(
            "DELETE FROM sources",
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

    /// Toggle favorite status of a font
    pub fn toggle_favorite(&self, id: &str, is_favorite: bool) -> Result<()> {
        self.conn.execute(
            "UPDATE fonts SET is_favorite = ?2 WHERE id = ?1",
            params![id, is_favorite as i32],
        )?;
        Ok(())
    }
    
    // ============ Source Management ============
    
    /// Add a source to the database
    pub fn add_source(&self, source: &Source) -> Result<()> {
        self.conn.execute(
            "INSERT OR REPLACE INTO sources 
             (id, name, type, path, metadata, date_added)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params![
                source.id,
                source.name,
                source.source_type,
                source.path,
                source.metadata,
                source.date_added,
            ],
        )?;
        Ok(())
    }
    
    /// Get all sources
    pub fn get_sources(&self) -> Result<Vec<Source>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, type, path, metadata, date_added
             FROM sources ORDER BY name"
        )?;
        
        let sources = stmt.query_map([], |row| {
            Ok(Source {
                id: row.get(0)?,
                name: row.get(1)?,
                source_type: row.get(2)?,
                path: row.get(3)?,
                metadata: row.get(4)?,
                date_added: row.get(5)?,
            })
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(sources)
    }
    
    /// Delete a source by ID
    pub fn delete_source(&self, id: &str) -> Result<()> {
        self.conn.execute(
            "DELETE FROM sources WHERE id = ?1",
            [id],
        )?;
        Ok(())
    }
    
    /// Get font count by source
    pub fn get_font_count_by_source(&self, source_id: &str) -> Result<usize> {
        let count: usize = self.conn.query_row(
            "SELECT COUNT(*) FROM fonts WHERE source_id = ?1",
            [source_id],
            |row| row.get(0)
        )?;
        Ok(count)
    }
    
    /// Get fonts by source with pagination
    pub fn get_fonts_by_source(
        &self,
        source_id: &str,
        limit: usize,
        offset: usize
    ) -> Result<Vec<FontRecord>> {
        let mut stmt = self.conn.prepare(
            "SELECT id, name, file_path, original_directory, category, classification,
                    tags, collection_id, date_added, date_modified, is_synced, file_exists,
                    x_height, width_variance, stroke_contrast, aperture,
                    terminal_sharpness, stress_angle, curvature_ratio, counter_density,
                    foundry, weight, is_italic, supports_vietnamese,
                    family_name, subfamily, weight_class, width_class, source_id,
                    primary_class, secondary_tags, serif_subtype, sans_subtype, script_subtype, is_favorite
             FROM fonts WHERE source_id = ?1 ORDER BY name LIMIT ?2 OFFSET ?3"
        )?;
        
        let fonts = stmt.query_map(params![source_id, limit, offset], |row| {
            self.map_font_row(row)
        })?.collect::<Result<Vec<_>>>()?;
        
        Ok(fonts)
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
    terminal_sharpness: Option<f64>,
    stress_angle: Option<f64>,
    curvature_ratio: Option<f64>,
    counter_density: Option<f64>,
    foundry: Option<String>,
    weight: Option<u32>,
    is_italic: bool,
    supports_vietnamese: bool,
    family_name: Option<String>,
    subfamily: Option<String>,
    weight_class: Option<i32>,
    width_class: Option<i32>,
    source_id: Option<String>,
    primary_class: Option<String>,
    secondary_tags: Option<String>,
    serif_subtype: Option<String>,
    sans_subtype: Option<String>,
    script_subtype: Option<String>,
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
        terminal_sharpness,
        stress_angle,
        curvature_ratio,
        counter_density,
        foundry,
        weight: weight.unwrap_or(400),
        is_italic,
        supports_vietnamese,
        family_name,
        subfamily,
        weight_class,
        width_class,
        source_id,
        // Taxonomy fields - use params or None
        primary_class,
        secondary_tags,
        serif_subtype,
        sans_subtype,
        script_subtype,
        is_favorite: false,
    }
}
