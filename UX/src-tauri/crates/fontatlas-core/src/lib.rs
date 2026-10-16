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
