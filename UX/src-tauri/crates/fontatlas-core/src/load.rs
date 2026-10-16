//! Nạp file: nhận diện container (SFNT / TTC / WOFF / WOFF2), giải nén, băm.
use std::path::Path;

use read_fonts::{FileRef, FontRef, TableProvider, types::Tag};

use crate::error::CoreError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Container {
    Sfnt,
    Collection,
    Woff1,
    Woff2,
}

pub struct FontFile {
    hash: blake3::Hash,
    container: Container,
    /// SFNT hoặc TTC (đã giải nén nếu là WOFF).
    data: Vec<u8>,
}

impl std::fmt::Debug for FontFile {
    // Không in bytes font.
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("FontFile")
            .field("hash", &self.hash_hex())
            .field("container", &self.container)
            .field("len", &self.data.len())
            .finish()
    }
}

impl FontFile {
    pub fn open(path: &Path) -> Result<Self, CoreError> {
        Self::from_bytes(std::fs::read(path)?)
    }

    pub fn from_bytes(raw: Vec<u8>) -> Result<Self, CoreError> {
        // Danh tính = file GỐC trên đĩa, không phải dạng đã giải nén.
        let hash = blake3::hash(&raw);
        if raw.len() < 12 {
            return Err(CoreError::NotAFont);
        }
        let (data, container) = match &raw[0..4] {
            b"wOFF" => (
                wuff::decompress_woff1(&raw).map_err(|e| CoreError::WoffDecode(format!("{e:?}")))?,
                Container::Woff1,
            ),
            b"wOF2" => (
                wuff::decompress_woff2(&raw).map_err(|e| CoreError::WoffDecode(format!("{e:?}")))?,
                Container::Woff2,
            ),
            b"ttcf" => (raw, Container::Collection),
            [0, 1, 0, 0] | b"OTTO" | b"true" => (raw, Container::Sfnt),
            _ => return Err(CoreError::NotAFont),
        };
        Ok(FontFile { hash, container, data })
    }

    pub fn hash(&self) -> &blake3::Hash {
        &self.hash
    }

    pub fn hash_hex(&self) -> String {
        self.hash.to_hex().to_string()
    }

    pub fn container(&self) -> Container {
        self.container
    }

    pub fn data(&self) -> &[u8] {
        &self.data
    }
}


pub struct Face<'a> {
    index: u32,
    font: FontRef<'a>,
}

impl<'a> Face<'a> {
    pub fn index(&self) -> u32 {
        self.index
    }
    pub fn font(&self) -> &FontRef<'a> {
        &self.font
    }
}

impl FontFile {
    pub fn faces(&self) -> Result<Vec<Face<'_>>, CoreError> {
        match FileRef::new(&self.data).map_err(|e| CoreError::Parse(e.to_string()))? {
            FileRef::Font(font) => Ok(vec![Face { index: 0, font }]),
            FileRef::Collection(c) => c
                .iter()
                .enumerate()
                .map(|(i, r)| {
                    r.map(|font| Face { index: i as u32, font })
                        .map_err(|e| CoreError::Parse(format!("face {i}: {e}")))
                })
                .collect(),
        }
    }

    /// read-fonts đọc lười: bảng cụt vẫn "parse được" → phải tự kiểm tra (rủi ro R1 của spike).
    pub fn validate(&self) -> Result<(), CoreError> {
        for face in self.faces()? {
            validate_face(&face.font, self.data.len())?;
        }
        Ok(())
    }
}

const REQUIRED: [&[u8; 4]; 5] = [b"head", b"hhea", b"maxp", b"cmap", b"hmtx"];

fn validate_face(font: &FontRef, file_len: usize) -> Result<(), CoreError> {
    for rec in font.table_directory.table_records() {
        let end = u64::from(rec.offset()) + u64::from(rec.length());
        if end > file_len as u64 {
            return Err(CoreError::Truncated(format!(
                "bảng {} kết thúc ở {end} > {file_len}",
                rec.tag()
            )));
        }
    }
    for tag in REQUIRED {
        if font.table_data(Tag::new(tag)).is_none() {
            return Err(CoreError::MissingTable(String::from_utf8_lossy(tag).into_owned()));
        }
    }
    // Font bitmap-only (sbix/CBDT) hợp lệ khi không có outline → báo no_outlines ở tầng đo.
    if let Some(glyf) = font.table_data(Tag::new(b"glyf")) {
        let loca = font.loca(None).map_err(|_| CoreError::MissingTable("loca".into()))?;
        let last = loca.get_raw(loca.len()).unwrap_or(0) as usize;
        if last > glyf.len() {
            return Err(CoreError::Truncated(format!("loca trỏ tới {last} > glyf {}", glyf.len())));
        }
    }
    Ok(())
}
