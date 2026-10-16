//! fontatlas-core: phân tích font thuần, không I/O DB.
mod anatomy;
mod error;
pub mod axes;
pub mod geom;
mod load;
mod metadata;
mod metric;
mod vietnamese;

pub use error::CoreError;
pub use load::{Container, Face, FontFile};
pub use metadata::{AxisInfo, LicenseClass, Metadata, OutlineFormat, classify_license};
pub use metric::{FailReason, Metric, Quality};
pub use vietnamese::{ViStatus, VietnameseCoverage, vi_required_chars};
pub use anatomy::{Anatomy, MeasureAt, measure_with_filter};

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::Serialize;

pub const ANALYZER_VERSION: &str = "core-0.1.0";

#[derive(Debug, Clone, Serialize)]
pub struct FaceReport {
    pub index: u32,
    pub metadata: Metadata,
    pub vietnamese: VietnameseCoverage,
    pub anatomy: Anatomy,
}

#[derive(Debug, Clone, Serialize)]
pub struct FileReport {
    pub hash_hex: String,
    pub container: Container,
    pub analyzer_version: &'static str,
    pub faces: Vec<FaceReport>,
}

pub fn analyze_bytes(raw: Vec<u8>) -> Result<FileReport, CoreError> {
    let file = FontFile::from_bytes(raw)?;
    file.validate()?;
    let faces = file
        .faces()?
        .iter()
        .map(|f| FaceReport {
            index: f.index(),
            metadata: f.metadata(),
            vietnamese: f.vietnamese(),
            anatomy: f.measure(MeasureAt::Standard),
        })
        .collect();
    Ok(FileReport { hash_hex: file.hash_hex(), container: file.container(), analyzer_version: ANALYZER_VERSION, faces })
}

pub fn analyze_path(path: &Path) -> Result<FileReport, CoreError> {
    analyze_bytes(std::fs::read(path)?)
}

/// Song song theo file; lỗi một file không ảnh hưởng file khác; giữ thứ tự đầu vào.
pub fn analyze_paths(paths: &[PathBuf]) -> Vec<(PathBuf, Result<FileReport, CoreError>)> {
    paths.par_iter().map(|p| (p.clone(), analyze_path(p))).collect()
}
