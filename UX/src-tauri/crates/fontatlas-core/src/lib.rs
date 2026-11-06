//! fontatlas-core: phân tích font thuần, không I/O DB.
mod anatomy;
pub mod axes;
mod error;
pub mod geom;
mod load;
mod metadata;
mod metric;
mod vietnamese;

pub use anatomy::{Anatomy, MeasureAt, measure_with_filter, standard_location};
pub use error::CoreError;
pub use load::{Container, Face, FontFile};
pub use metadata::{AxisInfo, LicenseClass, Metadata, NamedInstanceInfo, OutlineFormat, classify_license};
pub use metric::{FailReason, Metric, Quality};
pub use vietnamese::{StackStyle, ViMarks, ViStatus, VietnameseCoverage, vi_required_chars};

use std::path::{Path, PathBuf};

use rayon::prelude::*;
use serde::Serialize;

pub const ANALYZER_VERSION: &str = "core-0.4.1";

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
    Ok(FileReport {
        hash_hex: file.hash_hex(),
        container: file.container(),
        analyzer_version: ANALYZER_VERSION,
        faces,
    })
}

pub fn analyze_path(path: &Path) -> Result<FileReport, CoreError> {
    analyze_bytes(std::fs::read(path)?)
}

/// Song song theo file; lỗi một file không ảnh hưởng file khác; giữ thứ tự đầu vào.
pub fn analyze_paths(paths: &[PathBuf]) -> Vec<(PathBuf, Result<FileReport, CoreError>)> {
    paths
        .par_iter()
        .map(|p| (p.clone(), guarded(|| analyze_path(p))))
        .collect()
}

/// Bắt panic (từ dependency parse dữ liệu lạ) thành lỗi `parse` của riêng file đó.
fn guarded(f: impl FnOnce() -> Result<FileReport, CoreError>) -> Result<FileReport, CoreError> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(f)).unwrap_or_else(|p| {
        let msg = p
            .downcast_ref::<&str>()
            .map(|s| s.to_string())
            .or_else(|| p.downcast_ref::<String>().cloned())
            .unwrap_or_default();
        Err(CoreError::Parse(format!("panic: {msg}")))
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn panic_in_one_file_becomes_parse_error() {
        // Panic trong dependency (skrifa/kurbo/wuff) không được làm hỏng cả lô.
        let r = guarded(|| panic!("boom"));
        let e = r.unwrap_err();
        assert_eq!(e.code(), "parse");
        assert!(e.to_string().contains("boom"), "{e}");
    }
}
