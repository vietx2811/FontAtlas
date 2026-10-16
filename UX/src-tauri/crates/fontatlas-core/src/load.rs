//! Nạp file: nhận diện container (SFNT / TTC / WOFF / WOFF2), giải nén, băm.
use std::path::Path;

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
